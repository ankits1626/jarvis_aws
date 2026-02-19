// Chrome browser adapter — macOS AppleScript implementation

use super::{BrowserAdapter, RawTab};
use std::process::Command;

/// Chrome adapter using AppleScript (macOS only)
pub struct ChromeAppleScriptAdapter;

impl ChromeAppleScriptAdapter {
    /// AppleScript that gets ALL tabs from ALL Chrome windows.
    /// Returns lines in format: URL|||Title
    const LIST_TABS_SCRIPT: &'static str = r#"
tell application "Google Chrome"
    set tabInfo to ""
    set windowCount to count of windows
    repeat with w from 1 to windowCount
        set tabCount to count of tabs in window w
        repeat with t from 1 to tabCount
            set tabURL to URL of tab t of window w
            set tabTitle to title of tab t of window w
            set tabInfo to tabInfo & tabURL & "|||" & tabTitle & linefeed
        end repeat
    end repeat
    return tabInfo
end tell
"#;

    /// AppleScript that gets ALL tabs with window/tab indices.
    /// Returns lines in format: windowIndex|||tabIndex|||URL
    /// Used by get_tab_html() to find a tab's position before executing JS.
    const LIST_TAB_INDICES_SCRIPT: &'static str = r#"
tell application "Google Chrome"
    set tabInfo to ""
    set windowCount to count of windows
    repeat with w from 1 to windowCount
        set tabCount to count of tabs in window w
        repeat with t from 1 to tabCount
            set tabURL to URL of tab t of window w
            set tabInfo to tabInfo & w & "|||" & t & "|||" & tabURL & linefeed
        end repeat
    end repeat
    return tabInfo
end tell
"#;

    /// Max HTML size we'll accept from a tab (5MB)
    const MAX_HTML_SIZE: usize = 5 * 1024 * 1024;
}

impl BrowserAdapter for ChromeAppleScriptAdapter {
    fn name(&self) -> &str {
        "Chrome (macOS)"
    }

    fn is_available(&self) -> bool {
        // Check if Chrome is running via AppleScript
        let output = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events" to (name of processes) contains "Google Chrome""#)
            .output();

        match output {
            Ok(out) => {
                let result = String::from_utf8_lossy(&out.stdout).trim().to_string();
                result == "true"
            }
            Err(_) => false,
        }
    }

    async fn list_tabs(&self) -> Result<Vec<RawTab>, String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(Self::LIST_TABS_SCRIPT)
            .output()
            .map_err(|e| format!("Failed to execute AppleScript: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("AppleScript failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tabs: Vec<RawTab> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, "|||").collect();
                if parts.len() == 2 {
                    Some(RawTab {
                        url: parts[0].trim().to_string(),
                        title: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(tabs)
    }

    async fn get_tab_html(&self, url: &str) -> Result<String, String> {
        let html = self
            .execute_js_in_tab(url, "document.documentElement.outerHTML")
            .await?;

        if html.len() > Self::MAX_HTML_SIZE {
            return Err(format!(
                "Page HTML too large ({:.1} MB) - this may not be a normal article page",
                html.len() as f64 / (1024.0 * 1024.0)
            ));
        }

        Ok(html)
    }

    async fn execute_js_in_tab(&self, url: &str, js_code: &str) -> Result<String, String> {
        // Step 1: List all tabs with window/tab indices
        let output = Command::new("osascript")
            .arg("-e")
            .arg(Self::LIST_TAB_INDICES_SCRIPT)
            .output()
            .map_err(|e| format!("Failed to list Chrome tabs: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to list Chrome tabs - check Chrome permissions: {}",
                stderr.trim()
            ));
        }

        // Step 2: Find matching tab in Rust (safe string comparison, no AppleScript injection)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut window_idx: Option<usize> = None;
        let mut tab_idx: Option<usize> = None;

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, "|||").collect();
            if parts.len() == 3 && parts[2].trim() == url {
                window_idx = parts[0].trim().parse().ok();
                tab_idx = parts[1].trim().parse().ok();
                break;
            }
        }

        let (w, t) = match (window_idx, tab_idx) {
            (Some(w), Some(t)) => (w, t),
            _ => {
                return Err(
                    "Chrome tab not found for this URL - the tab may have been closed".to_string(),
                )
            }
        };

        // Step 3: Execute JavaScript using numeric indices only (no URL interpolation)
        // Escape for AppleScript string: backslashes first, then double quotes
        let escaped_js = js_code.replace('\\', "\\\\").replace('"', "\\\"");
        let applescript = format!(
            r#"tell application "Google Chrome" to execute tab {} of window {} javascript "{}""#,
            t, w, escaped_js
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .output()
            .map_err(|e| format!("Failed to execute JavaScript in Chrome tab: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("JavaScript through AppleScript is turned off") {
                return Err(
                    "Chrome requires JavaScript from AppleScript to be enabled. \
                     Go to Chrome menu: View → Developer → Allow JavaScript from Apple Events"
                        .to_string(),
                );
            }
            return Err(format!(
                "Failed to execute JavaScript in Chrome tab: {}",
                stderr.trim()
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_applescript_output() {
        let output = "https://github.com|||GitHub\nhttps://youtube.com/watch?v=abc|||My Video\n";
        let tabs: Vec<RawTab> = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, "|||").collect();
                if parts.len() == 2 {
                    Some(RawTab {
                        url: parts[0].trim().to_string(),
                        title: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].url, "https://github.com");
        assert_eq!(tabs[0].title, "GitHub");
        assert_eq!(tabs[1].url, "https://youtube.com/watch?v=abc");
        assert_eq!(tabs[1].title, "My Video");
    }

    #[test]
    fn test_parse_empty_output() {
        let output = "\n\n";
        let tabs: Vec<RawTab> = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, "|||").collect();
                if parts.len() == 2 {
                    Some(RawTab {
                        url: parts[0].trim().to_string(),
                        title: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(tabs.len(), 0);
    }

    #[test]
    fn test_parse_title_with_delimiter() {
        // Title contains ||| — splitn(2, ...) should handle this
        let output = "https://example.com|||Page with ||| in title\n";
        let tabs: Vec<RawTab> = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, "|||").collect();
                if parts.len() == 2 {
                    Some(RawTab {
                        url: parts[0].trim().to_string(),
                        title: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].title, "Page with ||| in title");
    }
}
