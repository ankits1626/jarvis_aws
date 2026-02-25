// Claude Chrome Extension conversation extractor — uses macOS Accessibility API
// Extracts conversations from the Claude side panel using AXUIElement tree traversal.
// Uses depth-based message separation to distinguish user prompts from Claude responses.

#[cfg(target_os = "macos")]
use crate::browser::accessibility::{AccessibilityReader, TextBlock, WebArea};

use super::PageGist;
use crate::browser::tabs::{extract_domain, SourceType};

#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
struct ConversationData {
    full_text: String,
    message_count: u32,
    first_prompt: String,
}

/// Find the Claude web area from a list of web areas
#[cfg(target_os = "macos")]
fn find_claude_web_area(web_areas: &[WebArea]) -> Result<&WebArea, String> {
    web_areas
        .iter()
        .find(|wa| wa.title.contains("Claude"))
        .ok_or_else(|| {
            "No Claude conversation found. Open the Claude Chrome Extension side panel first."
                .to_string()
        })
}

/// Check if text contains plan indicators that signal start of Claude's response
#[cfg(target_os = "macos")]
fn is_plan_indicator(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("steps")
        || lower.contains("created a plan")
        || lower.contains("done")
        || lower.contains("extract page text")
}

/// Get the active tab URL from Chrome using AppleScript
#[cfg(target_os = "macos")]
async fn get_active_tab_url() -> Result<String, String> {
    let script = r#"tell application "Google Chrome" to get URL of active tab of front window"#;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute AppleScript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to get active tab URL: {}", stderr));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(url)
}

/// Check if the current message text ends mid-sentence (not at a sentence/paragraph boundary).
/// Used to determine whether the next text block should be joined inline.
#[cfg(target_os = "macos")]
fn ends_mid_sentence(text: &str) -> bool {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    let last_char = trimmed.chars().last().unwrap();
    // If text ends with sentence-ending punctuation or colon, it's a boundary
    !matches!(last_char, '.' | '!' | '?' | ':' | '\n')
}

/// Reconstruct conversation from text blocks using depth-based message separation.
///
/// Formatting improvements:
/// - Skips duplicate heading text (AXStaticText inside AXHeading)
/// - Joins inline fragments (bold/emphasis) without line breaks
/// - Adds paragraph breaks before headings and between sections
#[cfg(target_os = "macos")]
fn reconstruct_conversation(text_blocks: Vec<TextBlock>) -> Result<ConversationData, String> {
    if text_blocks.is_empty() {
        return Err("Claude conversation is empty".to_string());
    }

    let mut conversation_parts = Vec::new();
    let mut current_author = "You";
    let mut current_message = String::new();
    let mut message_count = 0u32;
    let mut first_prompt = String::new();
    let mut last_depth: usize = 0;
    let mut last_was_heading = false;

    for block in text_blocks {
        // Stop processing after "[input: Reply to Claude]" marker
        if block.text.contains("[input: Reply to Claude]") {
            break;
        }

        // Skip duplicate heading text: AXStaticText nodes inside AXHeading
        // already have their text captured by the parent AXHeading node
        if block.role == "AXStaticText" && block.parent_role.as_deref() == Some("AXHeading") {
            continue;
        }

        // Detect message boundary: depth decrease from >6 to <6 or plan indicator at shallow depth
        let is_boundary = (last_depth > 6 && block.depth < 6)
            || (block.depth < 6 && is_plan_indicator(&block.text));

        if is_boundary && !current_message.is_empty() {
            // Flush current message
            conversation_parts.push(format!(
                "--- {} ---\n{}",
                current_author,
                current_message.trim()
            ));

            if message_count == 0 {
                first_prompt = current_message.trim().chars().take(200).collect();
            }

            message_count += 1;
            current_message.clear();
            last_was_heading = false;

            current_author = if current_author == "You" {
                "Claude"
            } else {
                "You"
            };
        }

        if !block.text.trim().is_empty() {
            if current_message.is_empty() {
                // First block in message — no separator needed
            } else if block.role == "AXHeading" {
                // Blank line before headings
                current_message.push_str("\n\n");
            } else if last_was_heading {
                // Newline after heading (body text follows)
                current_message.push('\n');
            } else if block.parent_role.as_deref() == Some("AXGroup")
                && (block.depth as isize - last_depth as isize).abs() == 1
                && ends_mid_sentence(&current_message)
            {
                // Inline fragment: bold/emphasis text at depth +/- 1
                // when the previous text didn't end a sentence.
                // Join without any separator to reconstruct the original sentence.
            } else {
                // Default: newline between blocks
                current_message.push('\n');
            }

            current_message.push_str(&block.text);
            last_was_heading = block.role == "AXHeading";
        }

        last_depth = block.depth;
    }

    // Flush final message
    if !current_message.is_empty() {
        conversation_parts.push(format!(
            "--- {} ---\n{}",
            current_author,
            current_message.trim()
        ));

        if message_count == 0 {
            first_prompt = current_message.trim().chars().take(200).collect();
        }

        message_count += 1;
    }

    let mut full_text = conversation_parts.join("\n\n");

    // Truncate at 50,000 characters
    if full_text.len() > 50000 {
        full_text.truncate(50000);
        full_text.push_str("\n\n[conversation truncated]");
    }

    Ok(ConversationData {
        full_text,
        message_count,
        first_prompt,
    })
}

/// Extract page title from non-Claude web areas
#[cfg(target_os = "macos")]
fn extract_page_title(web_areas: &[WebArea]) -> Result<String, String> {
    web_areas
        .iter()
        .find(|wa| !wa.title.contains("Claude"))
        .map(|wa| wa.title.clone())
        .ok_or_else(|| "No active tab found".to_string())
}

/// Build PageGist from conversation data
#[cfg(target_os = "macos")]
fn build_page_gist(
    page_url: String,
    page_title: String,
    conversation_data: ConversationData,
    claude_version: String,
) -> PageGist {
    let domain = extract_domain(&page_url);
    let title = format!("Claude: {}", page_title);

    let mut extra = serde_json::Map::new();
    extra.insert(
        "page_url".to_string(),
        serde_json::Value::String(page_url.clone()),
    );
    extra.insert(
        "page_title".to_string(),
        serde_json::Value::String(page_title),
    );
    extra.insert(
        "message_count".to_string(),
        serde_json::Value::Number(conversation_data.message_count.into()),
    );
    extra.insert(
        "extraction_method".to_string(),
        serde_json::Value::String("accessibility_api".to_string()),
    );
    extra.insert(
        "claude_extension_version".to_string(),
        serde_json::Value::String(claude_version),
    );

    PageGist {
        url: page_url,
        title,
        source_type: SourceType::Chat,
        domain,
        author: Some("Claude Extension".to_string()),
        description: Some(conversation_data.first_prompt),
        content_excerpt: Some(conversation_data.full_text),
        published_date: None,
        image_url: None,
        extra: serde_json::Value::Object(extra),
    }
}

/// Extract Claude conversation from Chrome Extension side panel
#[cfg(target_os = "macos")]
pub async fn extract() -> Result<PageGist, String> {
    // Check accessibility permission
    if !AccessibilityReader::check_permission() {
        return Err("Accessibility permission not granted. Please enable accessibility access for this app in System Settings > Privacy & Security > Accessibility.".to_string());
    }

    // Find Chrome process
    let pid = AccessibilityReader::find_chrome_pid()
        .map_err(|e| {
            eprintln!("[ClaudeExtractor] {}", e);
            e
        })?;

    // Find all web areas and extract all needed data BEFORE any await
    let (claude_version, page_title, text_blocks) = {
        let web_areas = AccessibilityReader::find_web_areas(pid)
            .map_err(|e| {
                eprintln!("[ClaudeExtractor] Failed to find web areas: {}", e);
                e
            })?;

        // Find Claude web area
        let claude_web_area = find_claude_web_area(&web_areas)
            .map_err(|e| {
                eprintln!("[ClaudeExtractor] {}", e);
                e
            })?;

        // Extract text content from Claude web area
        let text_blocks = AccessibilityReader::extract_text_content(claude_web_area.element)
            .map_err(|e| {
                eprintln!("[ClaudeExtractor] Failed to extract text content: {}", e);
                e
            })?;

        // Extract page title from non-Claude web areas
        let page_title = extract_page_title(&web_areas)
            .map_err(|e| {
                eprintln!("[ClaudeExtractor] {}", e);
                e
            })?;

        // Clone the Claude version title
        let claude_version = claude_web_area.title.clone();

        // Return all extracted data (web_areas is dropped here)
        (claude_version, page_title, text_blocks)
    };

    // Reconstruct conversation
    let conversation_data = reconstruct_conversation(text_blocks)
        .map_err(|e| {
            eprintln!("[ClaudeExtractor] {}", e);
            e
        })?;

    // Get active tab URL using AppleScript (after all non-Send data is dropped)
    let page_url = get_active_tab_url().await
        .map_err(|e| {
            eprintln!("[ClaudeExtractor] Failed to get active tab URL: {}", e);
            e
        })?;

    // Build and return PageGist
    Ok(build_page_gist(
        page_url,
        page_title,
        conversation_data,
        claude_version,
    ))
}

/// Non-macOS stub implementation
#[cfg(not(target_os = "macos"))]
pub async fn extract() -> Result<PageGist, String> {
    Err("Claude conversation capture is only available on macOS".to_string())
}
