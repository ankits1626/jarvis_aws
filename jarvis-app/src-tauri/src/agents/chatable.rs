// Chatable Trait — Content Source Interface for Chatbot
//
// This trait defines the contract for any content source that can have a chatbot
// attached to it. The chatbot is completely generic — it only interacts through
// this trait, never importing or referencing concrete source types.

use async_trait::async_trait;
use std::path::PathBuf;
use crate::intelligence::queue::IntelQueue;

/// A content source that can be chatted with.
/// 
/// Any type implementing this trait becomes chatbot-compatible. The chatbot
/// calls these methods to get context, determine storage location, and show
/// preparation progress.
#[async_trait]
pub trait Chatable: Send + Sync {
    /// Get the text context the chatbot will answer questions from.
    /// 
    /// Called on every message — must be fast for static sources (disk read),
    /// and fresh for growing sources (live transcript).
    /// 
    /// # Arguments
    /// 
    /// * `intel_queue` - Queue for submitting generation requests if needed
    /// 
    /// # Returns
    /// 
    /// The context text, or an error if context cannot be obtained
    async fn get_context(&self, intel_queue: &IntelQueue) -> Result<String, String>;

    /// Human-readable label for session log headers.
    /// 
    /// # Examples
    /// 
    /// * "Recording 20260228_143022"
    /// * "Gem: Pricing Meeting"
    fn label(&self) -> String;

    /// Directory where chat session .md files are stored.
    fn session_dir(&self) -> PathBuf;

    /// Whether context preparation is needed (e.g. transcript generation).
    /// 
    /// If true, chatbot shows "Preparing..." before first message.
    async fn needs_preparation(&self) -> bool;

    /// Optional: called during preparation to show progress to the user.
    /// 
    /// Default implementation is a no-op. Sources that need preparation
    /// can override this to emit status events.
    /// 
    /// # Arguments
    /// 
    /// * `status` - Status identifier (e.g. "preparing", "ready", "error")
    /// * `message` - Human-readable status message
    fn on_preparation_status(&self, _status: &str, _message: &str) {
        // Default: no-op
    }
}
