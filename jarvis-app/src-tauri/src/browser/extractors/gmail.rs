// Gmail email thread extractor — uses DOM extraction via Chrome adapter
// Extracts subject, sender, participants, and full thread text from Gmail.
// Uses stable selectors only (ARIA roles, data-message-id, [email] attributes)
// to avoid dependence on Gmail's obfuscated class names.

use super::PageGist;
use crate::browser::adapters::chrome::ChromeAppleScriptAdapter;
use crate::browser::adapters::BrowserAdapter;
use crate::browser::tabs::SourceType;
use serde::Deserialize;

/// Data extracted from Gmail DOM via JavaScript
#[derive(Deserialize)]
struct GmailDomData {
    subject: Option<String>,
    sender: Option<String>,
    participants: Option<Vec<String>>,
    email_count: Option<u32>,
    thread_text: Option<String>,
    is_thread: Option<bool>,
}

/// JavaScript that extracts Gmail thread data in one call.
/// Uses only stable selectors: document.title, [role="main"], [data-message-id], [email].
const EXTRACT_JS: &str = r#"(function(){
  var d = {};
  var rawTitle = document.title || '';
  var titleParts = rawTitle.split(' - ');
  if (titleParts.length >= 2) {
    titleParts.pop();
    d.sender = (titleParts.pop() || '').trim();
    d.subject = titleParts.join(' - ');
  } else {
    d.subject = rawTitle;
    d.sender = null;
  }
  var main = document.querySelector('[role="main"]');
  var threadParts = [];
  var emailCount = 0;
  if (main) {
    var messages = main.querySelectorAll('[data-message-id]');
    emailCount = messages.length;
    for (var i = 0; i < messages.length; i++) {
      var text = messages[i].innerText;
      if (text && text.trim().length > 0) {
        threadParts.push('--- Email ' + (i + 1) + ' ---\n' + text.trim());
      }
    }
  }
  d.email_count = emailCount;
  d.thread_text = threadParts.join('\n\n');
  var emailAttrs = [];
  if (main) {
    var spans = main.querySelectorAll('[email]');
    for (var j = 0; j < spans.length; j++) {
      var addr = spans[j].getAttribute('email');
      if (addr && emailAttrs.indexOf(addr) === -1) {
        emailAttrs.push(addr);
      }
    }
  }
  d.participants = emailAttrs;
  d.is_thread = emailCount > 0;
  if (d.thread_text && d.thread_text.length > 50000) {
    d.thread_text = d.thread_text.substring(0, 50000) + '\n\n[thread truncated]';
  }
  return JSON.stringify(d);
})()"#;

/// Extract a gist from a Gmail email thread via DOM extraction
pub async fn extract(
    url: &str,
    source_type: &SourceType,
    domain: &str,
) -> Result<PageGist, String> {
    let adapter = ChromeAppleScriptAdapter;

    let json_str = adapter.execute_js_in_tab(url, EXTRACT_JS).await?;

    let data: GmailDomData = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse Gmail thread data: {}", e))?;

    // Guard: must be viewing a specific thread
    if !data.is_thread.unwrap_or(false) {
        return Err(
            "No email thread found — please open a specific email thread in Gmail".to_string(),
        );
    }

    let subject = data
        .subject
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown Thread".to_string());
    let email_count = data.email_count.unwrap_or(0);
    let participants = data.participants.unwrap_or_default();

    // Description: "3 emails · alice@x.com, bob@y.com"
    let description = {
        let count_part = format!(
            "{} email{}",
            email_count,
            if email_count == 1 { "" } else { "s" }
        );
        if participants.is_empty() {
            count_part
        } else {
            format!("{} · {}", count_part, participants.join(", "))
        }
    };

    let thread_text = data.thread_text.filter(|t| !t.is_empty());

    let mut extra = serde_json::Map::new();
    extra.insert(
        "email_count".to_string(),
        serde_json::Value::Number(email_count.into()),
    );
    extra.insert(
        "participants".to_string(),
        serde_json::Value::Array(
            participants
                .iter()
                .map(|p| serde_json::Value::String(p.clone()))
                .collect(),
        ),
    );

    Ok(PageGist {
        url: url.to_string(),
        title: subject,
        source_type: source_type.clone(),
        domain: domain.to_string(),
        author: data.sender,
        description: Some(description),
        content_excerpt: thread_text,
        published_date: None,
        image_url: None,
        extra: serde_json::Value::Object(extra),
    })
}
