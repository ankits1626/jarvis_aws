// Browser observation module for detecting and scraping YouTube videos

pub mod adapters;
pub mod extractors;
pub mod observer;
pub mod tabs;
pub mod youtube;

pub use observer::BrowserObserver;
pub use youtube::{fetch_oembed_metadata, scrape_youtube_gist, QuickMetadata, YouTubeGist};
