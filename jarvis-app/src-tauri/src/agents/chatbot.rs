// Chatbot — Reusable Chat Engine
//
// This module implements a trait-driven chatbot that works with any Chatable source.
// It manages sessions, builds LLM prompts, submits requests through IntelQueue,
// and maintains persistent markdown logs.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use super::chatable::Chatable;
use crate::intelligence::queue::{IntelCommand, IntelQueue, IntelResponse};

/// Maximum characters of context to include in system prompt
const MAX_CONTEXT_CHARS: usize = 14_000;

/// Maximum number of exchanges (user + assistant pairs) to include in history
const MAX_HISTORY_EXCHANGES: usize = 10;

/// Chatbot engine managing multiple concurrent chat sessions
pub struct Chatbot {
    sessions: HashMap<String, ChatSession>,
}

/// A single chat session with a content source
pub struct ChatSession {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub log_path: PathBuf,
    pub created_at: String,
}

/// A single message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,       // "user" | "assistant"
    pub content: String,
    pub timestamp: String,  // "HH:MM:SS"
}

impl Chatbot {
    /// Create a new chatbot instance
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Start a new chat session against any Chatable source.
    ///
    /// Creates the session and log file immediately. Does NOT trigger context
    /// preparation — the caller is responsible for spawning preparation if needed.
    /// Context is fetched lazily on each `send_message()` call.
    ///
    /// # Arguments
    ///
    /// * `source` - The content source to chat with
    ///
    /// # Returns
    ///
    /// The session ID string
    pub async fn start_session(
        &mut self,
        source: &dyn Chatable,
    ) -> Result<String, String> {
        // Generate session ID (no context generation here — caller handles preparation)
        let timestamp = chrono::Utc::now().timestamp();
        let session_id = format!("chat_{}", timestamp);

        // Create log file path
        let log_filename = format!("chat_session_{}.md", timestamp);
        let log_path = source.session_dir().join(log_filename);

        // Ensure session directory exists
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| format!("Failed to create session directory: {}", e))?;
        }

        // Write session header
        let header = format!(
            "# Chat Session\n\n**Label:** {}\n**Started:** {}\n\n---\n\n",
            source.label(),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        );

        tokio::fs::write(&log_path, header).await
            .map_err(|e| format!("Failed to write session log header: {}", e))?;

        // Create session
        let session = ChatSession {
            session_id: session_id.clone(),
            messages: Vec::new(),
            log_path,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    /// Send a message in an existing session.
    /// 
    /// Fetches fresh context from source on every call, builds system prompt
    /// with context + history, submits to IntelQueue, records messages,
    /// and appends to session log.
    /// 
    /// # Arguments
    /// 
    /// * `session_id` - The session ID
    /// * `user_message` - The user's message text
    /// * `source` - The content source (for fresh context)
    /// * `intel_queue` - Queue for submitting chat request
    /// 
    /// # Returns
    /// 
    /// The assistant's response text
    pub async fn send_message(
        &mut self,
        session_id: &str,
        user_message: &str,
        source: &dyn Chatable,
        intel_queue: &IntelQueue,
    ) -> Result<String, String> {
        // Get session
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        // Get fresh context from source
        let context = source.get_context(intel_queue).await?;

        // Build system message with truncated context
        let truncated_context = truncate_context(&context, MAX_CONTEXT_CHARS);
        let system_msg = format!(
            "You are a helpful assistant. Answer questions based on the following context. \
             Be concise and accurate. If the answer isn't in the context, say so.\n\n\
             --- CONTEXT ---\n{}",
            truncated_context
        );

        // Assemble messages: system + history (last 10 exchanges) + user message
        let mut llm_messages: Vec<(String, String)> = vec![
            ("system".into(), system_msg),
        ];

        // Add chat history (last 10 exchanges = 20 messages)
        let history_start = session.messages.len().saturating_sub(MAX_HISTORY_EXCHANGES * 2);
        for msg in &session.messages[history_start..] {
            llm_messages.push((msg.role.clone(), msg.content.clone()));
        }

        // Add current user message
        llm_messages.push(("user".into(), user_message.to_string()));

        // Submit to queue
        let response = intel_queue.submit(IntelCommand::Chat {
            messages: llm_messages,
        }).await?;

        // Extract response text
        let assistant_text = match response {
            IntelResponse::Chat(text) => text,
            _ => return Err("Unexpected response type from chat command".into()),
        };

        // Record messages with timestamps
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        let user_msg = ChatMessage {
            role: "user".into(),
            content: user_message.to_string(),
            timestamp: now.clone(),
        };
        let assistant_msg = ChatMessage {
            role: "assistant".into(),
            content: assistant_text.clone(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        };

        session.messages.push(user_msg.clone());
        session.messages.push(assistant_msg.clone());

        // Append to session log
        let log_entry = format!(
            "## User ({})\n{}\n\n## Assistant ({})\n{}\n\n---\n\n",
            user_msg.timestamp,
            user_msg.content,
            assistant_msg.timestamp,
            assistant_msg.content,
        );

        let mut file = OpenOptions::new()
            .append(true)
            .open(&session.log_path)
            .await
            .map_err(|e| format!("Failed to open session log: {}", e))?;

        file.write_all(log_entry.as_bytes()).await
            .map_err(|e| format!("Failed to write to session log: {}", e))?;

        Ok(assistant_text)
    }

    /// Get in-memory message history for a session.
    /// 
    /// # Arguments
    /// 
    /// * `session_id` - The session ID
    /// 
    /// # Returns
    /// 
    /// A cloned vector of all messages in the session
    pub fn get_history(&self, session_id: &str) -> Result<Vec<ChatMessage>, String> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        Ok(session.messages.clone())
    }

    /// Remove session from memory.
    /// 
    /// The session log file remains on disk.
    /// 
    /// # Arguments
    /// 
    /// * `session_id` - The session ID to end
    pub fn end_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}

/// Truncate context to last N characters if it exceeds the limit.
/// 
/// Takes the tail (most recent content) rather than the head, as recent
/// content is typically most relevant for answering questions.
/// 
/// # Arguments
/// 
/// * `text` - The context text
/// * `max_chars` - Maximum characters to keep
/// 
/// # Returns
/// 
/// A string slice containing at most `max_chars` characters from the end
fn truncate_context(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        text
    } else {
        // Take the tail — most recent content is most relevant
        &text[text.len() - max_chars..]
    }
}
