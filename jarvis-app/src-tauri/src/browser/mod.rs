// Browser observation module for detecting and scraping YouTube videos

pub mod observer;
pub mod youtube;

pub use observer::BrowserObserver;
pub use youtube::{scrape_youtube_gist, YouTubeGist};
