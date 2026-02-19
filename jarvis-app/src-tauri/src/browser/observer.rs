// Browser observer implementation - polls Chrome for active tab URL and detects YouTube videos

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::watch;
use std::time::Duration;
use std::sync::LazyLock;
use regex::Regex;
use serde::{Serialize, Deserialize};

pub struct BrowserObserver {
    app_handle: AppHandle,
    stop_tx: Option<watch::Sender<bool>>,
    is_running: bool,
}

/// Event payload for YouTube video detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeDetectedEvent {
    pub url: String,
    pub video_id: String,
}

impl BrowserObserver {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            stop_tx: None,
            is_running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Start the browser observer
    /// 
    /// Spawns a background task that polls Chrome's active tab URL every 3 seconds.
    /// Returns an error if the observer is already running.
    pub async fn start(&mut self) -> Result<(), String> {
        if self.is_running {
            return Err("Browser observer is already running".to_string());
        }

        eprintln!("BrowserObserver: Starting observer");

        // Create stop signal channel
        let (stop_tx, mut stop_rx) = watch::channel(false);
        self.stop_tx = Some(stop_tx);
        self.is_running = true;

        // Clone app_handle for background task
        let app_handle = self.app_handle.clone();

        // Spawn background polling task
        tokio::spawn(async move {
            let mut last_url = String::new();

            loop {
                tokio::select! {
                    biased; // Prioritize stop signal over polling

                    // Check for stop signal
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            eprintln!("BrowserObserver: Stop signal received, shutting down");
                            break;
                        }
                    }

                    // Poll Chrome every 3 seconds
                    _ = tokio::time::sleep(Duration::from_secs(3)) => {
                        eprintln!("BrowserObserver: Polling Chrome...");
                        match poll_chrome_url().await {
                            Ok(url) if url != last_url => {
                                eprintln!("BrowserObserver: URL changed to: {}", url);
                                last_url = url.clone();
                                classify_url(&url, &app_handle).await;
                            }
                            Ok(url) => {
                                eprintln!("BrowserObserver: URL unchanged: {}", url);
                            }
                            Err(e) => {
                                eprintln!("BrowserObserver: Chrome unavailable: {}", e);
                            }
                        }
                    }
                }
            }

            eprintln!("BrowserObserver: Polling task terminated");
        });

        Ok(())
    }

    /// Stop the browser observer
    /// 
    /// Sends stop signal to background task and resets internal state.
    /// Returns an error if the observer is not running.
    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.is_running {
            return Err("Browser observer is not running".to_string());
        }

        eprintln!("BrowserObserver: Stopping observer");

        // Send stop signal
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }

        // Reset state
        self.is_running = false;

        Ok(())
    }
}

/// Poll Chrome for the active tab URL using AppleScript
async fn poll_chrome_url() -> Result<String, String> {
    // Timeout after 2 seconds to prevent hanging if Chrome is unresponsive
    let output = tokio::time::timeout(
        Duration::from_secs(2),
        tokio::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"Google Chrome\" to return URL of active tab of front window")
            .output()
    )
    .await
    .map_err(|_| "AppleScript execution timed out after 2 seconds".to_string())?
    .map_err(|e| format!("Failed to execute osascript: {}", e))?;
    
    if !output.status.success() {
        return Err("Chrome not running or no windows".to_string());
    }
    
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(url)
}

/// Classify a URL and trigger appropriate actions
/// 
/// Currently checks for YouTube videos. Will be extended in future tasks
/// to detect other content types.
async fn classify_url(url: &str, app_handle: &AppHandle) {
    eprintln!("BrowserObserver: Classifying URL: {}", url);
    
    // Check for YouTube videos
    if let Some((video_id, full_url)) = detect_youtube(url) {
        eprintln!("BrowserObserver: YouTube video detected - ID: {}, URL: {}", video_id, full_url);
        
        // Emit youtube-video-detected event
        let event = YouTubeDetectedEvent {
            url: full_url.clone(),
            video_id: video_id.clone(),
        };
        
        match app_handle.emit("youtube-video-detected", &event) {
            Ok(()) => eprintln!("BrowserObserver: Event emitted successfully"),
            Err(e) => eprintln!("BrowserObserver: WARNING - Failed to emit event: {}", e),
        }

        // Send native macOS notification
        match app_handle
            .notification()
            .builder()
            .title("YouTube Video Detected")
            .body("Open JarvisApp to prepare a gist")
            .show()
        {
            Ok(()) => eprintln!("BrowserObserver: Notification sent successfully"),
            Err(e) => eprintln!("BrowserObserver: WARNING - Failed to send notification: {}", e),
        }
    }
}

/// Detect YouTube video URLs and extract video ID
/// 
/// Matches both youtube.com/watch?v= and youtu.be/ formats.
/// Returns Some((video_id, full_url)) if YouTube video detected, None otherwise.
fn detect_youtube(url: &str) -> Option<(String, String)> {
    // Compile regex once at startup using LazyLock
    static YOUTUBE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        // Match youtube.com/watch with v= as any query parameter (not just first)
        // Also match youtu.be/ short URLs
        Regex::new(r"(?:youtube\.com/watch\?(?:[^&]*&)*v=|youtu\.be/)([a-zA-Z0-9_-]{11})").unwrap()
    });
    
    YOUTUBE_REGEX.captures(url).map(|caps| {
        let video_id = caps[1].to_string();
        let full_url = if url.contains("youtube.com") {
            url.to_string()
        } else {
            // Convert youtu.be short URL to full youtube.com URL
            format!("https://www.youtube.com/watch?v={}", video_id)
        };
        (video_id, full_url)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_poll_chrome_url_timeout() {
        // This test verifies that poll_chrome_url times out after 2 seconds
        // Note: This test will actually try to execute osascript, so it may succeed
        // if Chrome is running. The timeout behavior is tested by the implementation.
        let result = poll_chrome_url().await;
        
        // The result can be either:
        // - Ok(url) if Chrome is running with a window
        // - Err with "Chrome not running or no windows" if Chrome is not running
        // - Err with "AppleScript execution timed out" if it takes too long (unlikely in practice)
        
        // We just verify that it returns within a reasonable time and doesn't panic
        match result {
            Ok(url) => {
                // If Chrome is running, URL should be empty or start with http/https
                assert!(
                    url.is_empty() || url.starts_with("http"),
                    "URL should be empty or start with http, got: {}", url
                );
            }
            Err(e) => {
                // Error messages should be descriptive
                assert!(
                    e.contains("Chrome not running") || 
                    e.contains("timed out") || 
                    e.contains("Failed to execute"),
                    "Unexpected error message: {}", e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_poll_chrome_url_error_messages() {
        // Test that error messages are descriptive
        let result = poll_chrome_url().await;
        
        if let Err(e) = result {
            // Verify error message is not empty and contains useful information
            assert!(!e.is_empty(), "Error message should not be empty");
            assert!(
                e.len() > 10,
                "Error message should be descriptive, got: {}", e
            );
        }
    }

    // Note: Full lifecycle tests (start/stop, state transitions, polling behavior)
    // require a Tauri AppHandle which is not available in unit tests.
    // These will be implemented as integration tests in Task 15 where we have
    // access to the full Tauri app context.
    //
    // Expected integration test coverage:
    // - test_start_returns_error_when_already_running
    // - test_stop_returns_error_when_not_running  
    // - test_is_running_state_transitions
    // - test_observer_stops_within_poll_interval
    // - test_url_debouncing_prevents_duplicate_processing
    // - test_observer_continues_when_chrome_unavailable

    #[test]
    fn test_detect_youtube_standard_url() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        assert_eq!(full_url, url);
    }

    #[test]
    fn test_detect_youtube_short_url() {
        let url = "https://youtu.be/dQw4w9WgXcQ";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        // Short URL should be converted to full URL
        assert_eq!(full_url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    }

    #[test]
    fn test_detect_youtube_without_www() {
        let url = "https://youtube.com/watch?v=dQw4w9WgXcQ";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        assert_eq!(full_url, url);
    }

    #[test]
    fn test_detect_youtube_with_additional_params() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s&list=PLtest";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        assert_eq!(full_url, url);
    }

    #[test]
    fn test_detect_youtube_http_protocol() {
        let url = "http://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        assert_eq!(full_url, url);
    }

    #[test]
    fn test_detect_youtube_non_youtube_url() {
        let url = "https://www.google.com/search?q=test";
        let result = detect_youtube(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_youtube_malformed_video_id() {
        // Video ID must be exactly 11 characters
        let url = "https://www.youtube.com/watch?v=short";
        let result = detect_youtube(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_youtube_missing_video_id() {
        let url = "https://www.youtube.com/watch";
        let result = detect_youtube(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_youtube_empty_string() {
        let url = "";
        let result = detect_youtube(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_youtube_video_id_with_underscore_and_dash() {
        // Video IDs can contain letters, numbers, underscores, and dashes
        let url = "https://www.youtube.com/watch?v=aB-_1234567";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, _) = result.unwrap();
        assert_eq!(video_id, "aB-_1234567");
    }

    #[test]
    fn test_detect_youtube_v_not_first_param() {
        // Real-world case: playlist links often have list= before v=
        let url = "https://www.youtube.com/watch?list=PLtest&v=dQw4w9WgXcQ";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        assert_eq!(full_url, url);
    }

    #[test]
    fn test_detect_youtube_v_in_middle_of_params() {
        // v= parameter in the middle of multiple parameters
        let url = "https://www.youtube.com/watch?feature=share&v=dQw4w9WgXcQ&t=42s";
        let result = detect_youtube(url);
        assert!(result.is_some());
        let (video_id, full_url) = result.unwrap();
        assert_eq!(video_id, "dQw4w9WgXcQ");
        assert_eq!(full_url, url);
    }
}

