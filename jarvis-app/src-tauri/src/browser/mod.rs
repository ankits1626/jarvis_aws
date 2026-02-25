// Browser observation module for detecting and scraping YouTube videos

#[cfg(target_os = "macos")]
pub mod accessibility;
#[cfg(all(test, target_os = "macos"))]
mod accessibility_tests;
pub mod adapters;
pub mod extractors;
pub mod observer;
pub mod tabs;
pub mod youtube;

pub use observer::BrowserObserver;
pub use youtube::{fetch_oembed_metadata, scrape_youtube_gist, QuickMetadata, YouTubeGist};
