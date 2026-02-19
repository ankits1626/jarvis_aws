// ChatGPT conversation extractor — uses DOM extraction via Chrome adapter
// Extracts conversation title, messages (user + assistant), and message count.
// Uses stable selectors: [data-message-author-role], [data-message-id], document.title.

use super::PageGist;
use crate::browser::adapters::chrome::ChromeAppleScriptAdapter;
use crate::browser::adapters::BrowserAdapter;
use crate::browser::tabs::SourceType;
use serde::Deserialize;

/// Data extracted from ChatGPT DOM via JavaScript
#[derive(Deserialize)]
struct ChatGptDomData {
    title: Option<String>,
    message_count: Option<u32>,
    conversation_text: Option<String>,
    is_conversation: Option<bool>,
}

/// JavaScript that extracts ChatGPT conversation data in one call.
/// Uses stable selectors: document.title, [data-message-author-role], [data-message-id].
const EXTRACT_JS: &str = r#"(function(){
  var d = {};
  var rawTitle = document.title || '';
  if (rawTitle.endsWith(' - ChatGPT')) {
    d.title = rawTitle.slice(0, -10).trim();
  } else if (rawTitle === 'ChatGPT') {
    d.title = 'New Conversation';
  } else {
    d.title = rawTitle;
  }
  var messages = document.querySelectorAll('[data-message-author-role]');
  var parts = [];
  var messageCount = messages.length;
  for (var i = 0; i < messages.length; i++) {
    var role = messages[i].getAttribute('data-message-author-role');
    var label = role === 'user' ? 'You' : role === 'assistant' ? 'ChatGPT' : role;
    var text = messages[i].innerText;
    if (text && text.trim().length > 0) {
      parts.push('--- ' + label + ' ---\n' + text.trim());
    }
  }
  d.message_count = messageCount;
  d.conversation_text = parts.join('\n\n');
  d.is_conversation = messageCount > 0;
  if (d.conversation_text && d.conversation_text.length > 50000) {
    d.conversation_text = d.conversation_text.substring(0, 50000) + '\n\n[conversation truncated]';
  }
  return JSON.stringify(d);
})()"#;

/// Extract a gist from a ChatGPT conversation via DOM extraction
pub async fn extract(
    url: &str,
    source_type: &SourceType,
    domain: &str,
) -> Result<PageGist, String> {
    let adapter = ChromeAppleScriptAdapter;

    let json_str = adapter.execute_js_in_tab(url, EXTRACT_JS).await?;

    let data: ChatGptDomData = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse ChatGPT conversation data: {}", e))?;

    // Guard: must be viewing a conversation with messages
    if !data.is_conversation.unwrap_or(false) {
        return Err(
            "No conversation found — please open a specific ChatGPT conversation".to_string(),
        );
    }

    let title = data
        .title
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled Conversation".to_string());
    let message_count = data.message_count.unwrap_or(0);

    let description = format!(
        "{} message{}",
        message_count,
        if message_count == 1 { "" } else { "s" }
    );

    let conversation_text = data.conversation_text.filter(|t| !t.is_empty());

    let mut extra = serde_json::Map::new();
    extra.insert(
        "message_count".to_string(),
        serde_json::Value::Number(message_count.into()),
    );

    Ok(PageGist {
        url: url.to_string(),
        title,
        source_type: source_type.clone(),
        domain: domain.to_string(),
        author: None,
        description: Some(description),
        content_excerpt: conversation_text,
        published_date: None,
        image_url: None,
        extra: serde_json::Value::Object(extra),
    })
}
