// Browser adapter trait — browser-agnostic interface for tab retrieval

pub mod chrome;

use serde::{Deserialize, Serialize};

/// Raw tab info from a browser — browser-agnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTab {
    pub url: String,
    pub title: String,
}

/// Trait for browser-specific tab retrieval.
/// Implement this for each browser (Chrome, Safari, Arc, Firefox...).
pub trait BrowserAdapter {
    /// Human-readable browser name
    fn name(&self) -> &str;
    /// Check if this browser is running/available
    fn is_available(&self) -> bool;
    /// Get all open tabs across all windows
    fn list_tabs(&self) -> impl std::future::Future<Output = Result<Vec<RawTab>, String>> + Send;
    /// Get the full rendered HTML from a specific tab by URL.
    /// Uses DOM extraction (e.g. JavaScript execution) to get the actual page content,
    /// including authenticated/paywalled content the user can see.
    fn get_tab_html(&self, url: &str) -> impl std::future::Future<Output = Result<String, String>> + Send;
    /// Execute arbitrary JavaScript in a specific tab by URL and return the result string.
    /// The JS code should return a string (use JSON.stringify for objects).
    fn execute_js_in_tab(&self, url: &str, js_code: &str) -> impl std::future::Future<Output = Result<String, String>> + Send;
}
