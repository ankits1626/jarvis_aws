// YouTube scraper - fetches and extracts metadata from YouTube video pages

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeGist {
    pub url: String,
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub description: String,
    pub duration_seconds: u32,
}

/// Scrape YouTube video metadata from a video URL
/// 
/// Fetches the YouTube page HTML and extracts metadata fields.
/// Returns a YouTubeGist with video information or an error message.
pub async fn scrape_youtube_gist(url: &str) -> Result<YouTubeGist, String> {
    // Create HTTP client with 10-second timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // Fetch page HTML with timeout
    let html = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch YouTube page: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    
    // Extract video ID from URL
    let video_id = extract_video_id(url)?;
    
    // Extract title from <title> tag
    let title = extract_title(&html);
    
    // Extract ytInitialPlayerResponse JSON
    let player_response = extract_player_response(&html)?;
    
    // Parse JSON fields
    let channel = extract_channel(&player_response);
    let description = extract_description(&player_response);
    let duration_seconds = extract_duration(&player_response);
    
    Ok(YouTubeGist {
        url: url.to_string(),
        video_id,
        title,
        channel,
        description,
        duration_seconds,
    })
}

/// Extract video ID from YouTube URL using regex
fn extract_video_id(url: &str) -> Result<String, String> {
    static VIDEO_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:youtube\.com/watch\?(?:[^&]*&)*v=|youtu\.be/)([a-zA-Z0-9_-]{11})").unwrap()
    });
    
    VIDEO_ID_REGEX
        .captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| format!("Failed to extract video ID from URL: {}", url))
}

/// Extract title from HTML <title> tag and strip " - YouTube" suffix
fn extract_title(html: &str) -> String {
    static TITLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<title>([^<]+)</title>").unwrap()
    });
    
    TITLE_REGEX
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| {
            let title = m.as_str();
            // Strip " - YouTube" suffix if present
            title.strip_suffix(" - YouTube").unwrap_or(title).to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Extract ytInitialPlayerResponse JSON from HTML using brace-counting
fn extract_player_response(html: &str) -> Result<String, String> {
    let start_marker = "var ytInitialPlayerResponse = ";
    let start_pos = html
        .find(start_marker)
        .ok_or_else(|| "Failed to find ytInitialPlayerResponse in page HTML".to_string())?;
    
    let json_start = start_pos + start_marker.len();
    let remaining = &html[json_start..];
    
    // Count braces to find the matching closing brace
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut json_end = 0;
    
    for (i, ch) in remaining.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => brace_count += 1,
            '}' if !in_string => {
                brace_count -= 1;
                if brace_count == 0 {
                    json_end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    
    if json_end == 0 {
        return Err("Failed to find closing brace for ytInitialPlayerResponse".to_string());
    }
    
    Ok(remaining[..json_end].to_string())
}

/// Extract channel name from JSON
fn extract_channel(json: &str) -> String {
    static CHANNEL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""ownerChannelName":"((?:[^"\\]|\\.)*)""#).unwrap()
    });
    
    CHANNEL_REGEX
        .captures(json)
        .and_then(|caps| caps.get(1))
        .map(|m| unescape_json(m.as_str()))
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Extract description from JSON
fn extract_description(json: &str) -> String {
    static DESCRIPTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""shortDescription":"((?:[^"\\]|\\.)*)""#).unwrap()
    });
    
    DESCRIPTION_REGEX
        .captures(json)
        .and_then(|caps| caps.get(1))
        .map(|m| unescape_json(m.as_str()))
        .unwrap_or_else(|| String::new())
}

/// Extract duration in seconds from JSON
fn extract_duration(json: &str) -> u32 {
    static DURATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""lengthSeconds":"(\d+)""#).unwrap()
    });
    
    DURATION_REGEX
        .captures(json)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0)
}

/// Unescape JSON string escape sequences
/// 
/// Handles: \n, \t, \", \\, \/, \r, \b, \f
fn unescape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next_ch) = chars.next() {
                match next_ch {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    'b' => result.push('\u{0008}'), // backspace
                    'f' => result.push('\u{000C}'), // form feed
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    '/' => result.push('/'),
                    _ => {
                        // Unknown escape sequence - keep as-is
                        result.push('\\');
                        result.push(next_ch);
                    }
                }
            } else {
                // Trailing backslash
                result.push('\\');
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id_standard_url() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let result = extract_video_id(url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "dQw4w9WgXcQ");
    }

    #[test]
    fn test_extract_video_id_short_url() {
        let url = "https://youtu.be/dQw4w9WgXcQ";
        let result = extract_video_id(url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "dQw4w9WgXcQ");
    }

    #[test]
    fn test_extract_video_id_with_params() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s";
        let result = extract_video_id(url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "dQw4w9WgXcQ");
    }

    #[test]
    fn test_extract_video_id_invalid_url() {
        let url = "https://www.google.com";
        let result = extract_video_id(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to extract video ID"));
    }

    #[test]
    fn test_extract_title_with_suffix() {
        let html = "<title>Test Video - YouTube</title>";
        let title = extract_title(html);
        assert_eq!(title, "Test Video");
    }

    #[test]
    fn test_extract_title_without_suffix() {
        let html = "<title>Test Video</title>";
        let title = extract_title(html);
        assert_eq!(title, "Test Video");
    }

    #[test]
    fn test_extract_title_missing() {
        let html = "<html><body>No title</body></html>";
        let title = extract_title(html);
        assert_eq!(title, "Unknown");
    }

    #[test]
    fn test_extract_player_response_simple() {
        let html = r#"var ytInitialPlayerResponse = {"test":"value"};</script>"#;
        let result = extract_player_response(html);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"test":"value"}"#);
    }

    #[test]
    fn test_extract_player_response_nested() {
        let html = r#"var ytInitialPlayerResponse = {"outer":{"inner":"value"}};</script>"#;
        let result = extract_player_response(html);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"outer":{"inner":"value"}}"#);
    }

    #[test]
    fn test_extract_player_response_with_escaped_quotes() {
        let html = r#"var ytInitialPlayerResponse = {"text":"He said \"hello\""};</script>"#;
        let result = extract_player_response(html);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r#"{"text":"He said \"hello\""}"#);
    }

    #[test]
    fn test_extract_player_response_missing() {
        let html = "<html><body>No player response</body></html>";
        let result = extract_player_response(html);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to find ytInitialPlayerResponse"));
    }

    #[test]
    fn test_extract_player_response_unclosed_braces() {
        let html = r#"var ytInitialPlayerResponse = {"test":"value"#;
        let result = extract_player_response(html);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to find closing brace"));
    }

    #[test]
    fn test_extract_channel_present() {
        let json = r#"{"ownerChannelName":"Test Channel"}"#;
        let channel = extract_channel(json);
        assert_eq!(channel, "Test Channel");
    }

    #[test]
    fn test_extract_channel_missing() {
        let json = r#"{"other":"field"}"#;
        let channel = extract_channel(json);
        assert_eq!(channel, "Unknown");
    }

    #[test]
    fn test_extract_description_present() {
        let json = r#"{"shortDescription":"Test description"}"#;
        let description = extract_description(json);
        assert_eq!(description, "Test description");
    }

    #[test]
    fn test_extract_description_missing() {
        let json = r#"{"other":"field"}"#;
        let description = extract_description(json);
        assert_eq!(description, "");
    }

    #[test]
    fn test_extract_duration_present() {
        let json = r#"{"lengthSeconds":"123"}"#;
        let duration = extract_duration(json);
        assert_eq!(duration, 123);
    }

    #[test]
    fn test_extract_duration_missing() {
        let json = r#"{"other":"field"}"#;
        let duration = extract_duration(json);
        assert_eq!(duration, 0);
    }

    #[test]
    fn test_extract_duration_invalid() {
        let json = r#"{"lengthSeconds":"invalid"}"#;
        let duration = extract_duration(json);
        assert_eq!(duration, 0);
    }

    #[test]
    fn test_unescape_json_newline() {
        let input = r"Line 1\nLine 2";
        let output = unescape_json(input);
        assert_eq!(output, "Line 1\nLine 2");
    }

    #[test]
    fn test_unescape_json_tab() {
        let input = r"Col1\tCol2";
        let output = unescape_json(input);
        assert_eq!(output, "Col1\tCol2");
    }

    #[test]
    fn test_unescape_json_quote() {
        let input = r#"He said \"hello\""#;
        let output = unescape_json(input);
        assert_eq!(output, r#"He said "hello""#);
    }

    #[test]
    fn test_unescape_json_backslash() {
        let input = r"Path\\to\\file";
        let output = unescape_json(input);
        assert_eq!(output, r"Path\to\file");
    }

    #[test]
    fn test_unescape_json_slash() {
        let input = r"https:\/\/example.com";
        let output = unescape_json(input);
        assert_eq!(output, "https://example.com");
    }

    #[test]
    fn test_unescape_json_multiple() {
        let input = r#"Line 1\nHe said \"hello\"\tEnd"#;
        let output = unescape_json(input);
        assert_eq!(output, "Line 1\nHe said \"hello\"\tEnd");
    }

    #[test]
    fn test_unescape_json_no_escapes() {
        let input = "Plain text";
        let output = unescape_json(input);
        assert_eq!(output, "Plain text");
    }

    #[test]
    fn test_unescape_json_trailing_backslash() {
        let input = r"Text\";
        let output = unescape_json(input);
        assert_eq!(output, r"Text\");
    }

    #[test]
    fn test_unescape_json_unknown_escape() {
        let input = r"Text\x";
        let output = unescape_json(input);
        assert_eq!(output, r"Text\x");
    }
}

// Property-based tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: jarvis-browser-vision, Property 7: YouTube URL detection and extraction
    // For any URL matching youtube.com/watch?v= or youtu.be/, the system should correctly
    // detect it as a YouTube video and extract the 11-character video ID
    proptest! {
        #[test]
        fn prop_youtube_url_detection_and_extraction(video_id in "[a-zA-Z0-9_-]{11}") {
            // Test youtube.com format
            let url1 = format!("https://www.youtube.com/watch?v={}", &video_id);
            let result1 = extract_video_id(&url1);
            prop_assert!(result1.is_ok(), "Failed to extract video ID from youtube.com URL");
            prop_assert_eq!(&result1.unwrap(), &video_id);

            // Test youtu.be format
            let url2 = format!("https://youtu.be/{}", &video_id);
            let result2 = extract_video_id(&url2);
            prop_assert!(result2.is_ok(), "Failed to extract video ID from youtu.be URL");
            prop_assert_eq!(&result2.unwrap(), &video_id);

            // Test with additional query parameters
            let url3 = format!("https://www.youtube.com/watch?v={}&t=42s", &video_id);
            let result3 = extract_video_id(&url3);
            prop_assert!(result3.is_ok(), "Failed to extract video ID with query params");
            prop_assert_eq!(&result3.unwrap(), &video_id);
        }
    }

    // Feature: jarvis-browser-vision, Property 9: Non-YouTube URLs don't emit events
    // For any non-YouTube URL, the system should not detect it as a YouTube video
    proptest! {
        #[test]
        fn prop_non_youtube_urls_not_detected(
            domain in "[a-z]{3,10}",
            tld in "(com|org|net|io)",
            path in "[a-z0-9/]{5,20}"
        ) {
            // Skip if domain is "youtube" or "youtu"
            prop_assume!(domain != "youtube" && domain != "youtu");
            
            let url = format!("https://{}.{}/{}", domain, tld, path);
            let result = extract_video_id(&url);
            prop_assert!(result.is_err(), "Non-YouTube URL was incorrectly detected: {}", url);
        }
    }

    // Feature: jarvis-browser-vision, Property 14: Scraper uses fallback values for missing fields
    // For any YouTube page HTML, when specific metadata fields cannot be extracted,
    // the scraper should use fallback values rather than failing
    proptest! {
        #[test]
        fn prop_missing_channel_uses_fallback(
            title in ".*",
            description in ".*"
        ) {
            // JSON without ownerChannelName field
            let json = format!(
                r#"{{"videoDetails":{{"title":"{}","shortDescription":"{}"}}}}"#,
                title.replace('"', r#"\""#),
                description.replace('"', r#"\""#)
            );
            
            let channel = extract_channel(&json);
            prop_assert_eq!(channel, "Unknown", "Missing channel should use 'Unknown' fallback");
        }

        #[test]
        fn prop_missing_description_uses_fallback(
            title in ".*",
            channel in ".*"
        ) {
            // JSON without shortDescription field
            let json = format!(
                r#"{{"videoDetails":{{"title":"{}","ownerChannelName":"{}"}}}}"#,
                title.replace('"', r#"\""#),
                channel.replace('"', r#"\""#)
            );
            
            let description = extract_description(&json);
            prop_assert_eq!(description, "", "Missing description should use empty string fallback");
        }

        #[test]
        fn prop_missing_duration_uses_fallback(
            title in ".*",
            channel in ".*"
        ) {
            // JSON without lengthSeconds field
            let json = format!(
                r#"{{"videoDetails":{{"title":"{}","ownerChannelName":"{}"}}}}"#,
                title.replace('"', r#"\""#),
                channel.replace('"', r#"\""#)
            );
            
            let duration = extract_duration(&json);
            prop_assert_eq!(duration, 0, "Missing duration should use 0 fallback");
        }

        #[test]
        fn prop_invalid_duration_uses_fallback(
            invalid_duration in "[a-zA-Z]{3,10}"
        ) {
            // JSON with non-numeric lengthSeconds
            let json = format!(r#"{{"lengthSeconds":"{}"}}"#, invalid_duration);
            
            let duration = extract_duration(&json);
            prop_assert_eq!(duration, 0, "Invalid duration should use 0 fallback");
        }
    }

    // Feature: jarvis-browser-vision, Property 12: YouTube gist contains all required fields
    // For any successfully extracted metadata, all fields should have valid values
    proptest! {
        #[test]
        fn prop_extracted_fields_are_valid(
            title in "[a-zA-Z0-9 ]{1,100}",
            channel in "[a-zA-Z0-9 ]{1,50}",
            description in "[a-zA-Z0-9 ]{0,200}",
            duration in 1u32..86400u32  // 1 second to 24 hours
        ) {
            let json = format!(
                r#"{{"videoDetails":{{"title":"{}","ownerChannelName":"{}","shortDescription":"{}","lengthSeconds":"{}"}}}}"#,
                title, channel, description, duration
            );
            
            let extracted_title = extract_title(&format!("<title>{} - YouTube</title>", title));
            let extracted_channel = extract_channel(&json);
            let extracted_description = extract_description(&json);
            let extracted_duration = extract_duration(&json);
            
            prop_assert_eq!(extracted_title, title);
            prop_assert_eq!(extracted_channel, channel);
            prop_assert_eq!(extracted_description, description);
            prop_assert_eq!(extracted_duration, duration);
        }
    }

    // Test unescape_json with various escape sequences
    proptest! {
        #[test]
        fn prop_unescape_json_handles_escapes(
            text1 in "[a-zA-Z0-9 ]{0,20}",
            text2 in "[a-zA-Z0-9 ]{0,20}"
        ) {
            // Test newline escape
            let input = format!(r"{}\n{}", text1, text2);
            let output = unescape_json(&input);
            prop_assert!(output.contains('\n'), "Newline escape should be unescaped");
            
            // Test tab escape
            let input = format!(r"{}\t{}", text1, text2);
            let output = unescape_json(&input);
            prop_assert!(output.contains('\t'), "Tab escape should be unescaped");
            
            // Test quote escape
            let input = format!(r#"{}\"{}\"#, text1, text2);
            let output = unescape_json(&input);
            prop_assert!(output.contains('"'), "Quote escape should be unescaped");
        }
    }
}
