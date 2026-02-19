pub mod manager;
pub mod model_manager;

#[cfg(test)]
mod tests;

pub use manager::{BrowserSettings, Settings, SettingsManager, TranscriptionSettings};
pub use model_manager::{ModelInfo, ModelManager, ModelStatus};
