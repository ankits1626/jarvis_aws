use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Main settings structure containing all application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub transcription: TranscriptionSettings,
    #[serde(default)]
    pub browser: BrowserSettings,
}

/// Transcription-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSettings {
    pub vad_enabled: bool,
    pub vad_threshold: f32,
    pub vosk_enabled: bool,
    pub whisper_enabled: bool,
    pub whisper_model: String,
    #[serde(default = "default_engine")]
    pub transcription_engine: String,
    #[serde(default = "default_whisperkit_model")]
    pub whisperkit_model: String,
    #[serde(default = "default_window_duration")]
    pub window_duration: f32,
}

/// Browser observer settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    pub observer_enabled: bool,
}

fn default_engine() -> String {
    "whisper-rs".to_string()
}

fn default_whisperkit_model() -> String {
    "openai_whisper-large-v3_turbo".to_string()
}

fn default_window_duration() -> f32 {
    3.0
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            vad_enabled: true,
            vad_threshold: 0.3,
            vosk_enabled: true,
            whisper_enabled: true,
            whisper_model: "ggml-base.en.bin".to_string(),
            transcription_engine: default_engine(),
            whisperkit_model: default_whisperkit_model(),
            window_duration: default_window_duration(),
        }
    }
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            observer_enabled: true,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            transcription: TranscriptionSettings::default(),
            browser: BrowserSettings::default(),
        }
    }
}

/// Manages settings persistence and provides thread-safe access
pub struct SettingsManager {
    settings_path: PathBuf,
    current_settings: Arc<RwLock<Settings>>,
}

impl SettingsManager {
    /// Creates a new SettingsManager and loads settings from disk
    /// 
    /// If the settings file doesn't exist, creates it with default values.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The settings directory cannot be created
    /// - The settings file cannot be read or written
    /// - The settings file contains invalid JSON
    pub fn new() -> Result<Self, String> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| "Failed to get home directory".to_string())?;
        
        let jarvis_dir = home_dir.join(".jarvis");
        let settings_path = jarvis_dir.join("settings.json");
        
        Self::new_with_path(settings_path)
    }
    
    /// Creates a new SettingsManager with a custom settings path
    /// 
    /// This is primarily used for testing but is also used internally by new().
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The settings directory cannot be created
    /// - The settings file cannot be read or written
    /// - The settings file contains invalid JSON
    pub(crate) fn new_with_path(settings_path: PathBuf) -> Result<Self, String> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = settings_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create settings directory: {}", e))?;
            }
        }
        
        let manager = Self {
            settings_path: settings_path.clone(),
            current_settings: Arc::new(RwLock::new(Self::default_settings())),
        };
        
        // Load settings from file or create with defaults
        let settings = if settings_path.exists() {
            manager.load_from_file()?
        } else {
            let defaults = Self::default_settings();
            manager.save_to_file(&defaults)?;
            defaults
        };
        
        // Update in-memory settings
        *manager.current_settings.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))? = settings;
        
        Ok(manager)
    }
    
    /// Returns a clone of the current settings
    pub fn get(&self) -> Settings {
        self.current_settings.read()
            .expect("Failed to acquire read lock")
            .clone()
    }
    
    /// Updates settings (validates, persists to disk, then updates in-memory)
    /// 
    /// This method follows the critical ordering:
    /// 1. Validate settings
    /// 2. Persist to disk (FIRST)
    /// 3. Update in-memory state (ONLY if persist succeeded)
    /// 
    /// This ensures in-memory state never becomes stale if disk write fails.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - Validation fails
    /// - Disk write fails
    /// 
    /// If an error occurs, in-memory state remains unchanged.
    pub fn update(&self, settings: Settings) -> Result<(), String> {
        // Step 1: Validate
        Self::validate(&settings)?;
        
        // Step 2: Persist to disk FIRST
        self.save_to_file(&settings)?;
        
        // Step 3: Update in-memory state ONLY if save succeeded
        *self.current_settings.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))? = settings;
        
        Ok(())
    }
    
    /// Validates settings schema and constraints
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - vad_threshold is not in range [0.0, 1.0]
    /// - whisper_model is an empty string
    /// - transcription_engine is not "whisper-rs" or "whisperkit"
    fn validate(settings: &Settings) -> Result<(), String> {
        // Validate VAD threshold range
        if settings.transcription.vad_threshold < 0.0 || settings.transcription.vad_threshold > 1.0 {
            return Err(format!(
                "VAD threshold must be between 0.0 and 1.0, got {}",
                settings.transcription.vad_threshold
            ));
        }
        
        // Validate whisper_model is non-empty
        if settings.transcription.whisper_model.trim().is_empty() {
            return Err("Whisper model name cannot be empty".to_string());
        }
        
        // Validate window_duration range (1-10 seconds)
        if settings.transcription.window_duration < 1.0 || settings.transcription.window_duration > 10.0 {
            return Err(format!(
                "Window duration must be between 1.0 and 10.0 seconds, got {}",
                settings.transcription.window_duration
            ));
        }

        // Validate transcription_engine
        let engine = settings.transcription.transcription_engine.as_str();
        if engine != "whisper-rs" && engine != "whisperkit" {
            return Err(format!(
                "Transcription engine must be 'whisper-rs' or 'whisperkit', got '{}'",
                engine
            ));
        }
        
        Ok(())
    }
    
    /// Returns default settings
    fn default_settings() -> Settings {
        Settings::default()
    }
    
    /// Loads settings from disk
    /// 
    /// If the file contains invalid JSON, logs an error and returns defaults
    /// to ensure graceful degradation.
    fn load_from_file(&self) -> Result<Settings, String> {
        let contents = std::fs::read_to_string(&self.settings_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;
        
        match serde_json::from_str(&contents) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                eprintln!("Failed to parse settings JSON: {}. Using defaults.", e);
                Ok(Self::default_settings())
            }
        }
    }
    
    /// Saves settings to disk atomically
    /// 
    /// Uses a temporary file and atomic rename to prevent partial writes.
    fn save_to_file(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        
        // Write to temporary file
        let temp_path = self.settings_path.with_extension("json.tmp");
        std::fs::write(&temp_path, json)
            .map_err(|e| format!("Failed to write temporary settings file: {}", e))?;
        
        // Atomic rename
        std::fs::rename(&temp_path, &self.settings_path)
            .map_err(|e| format!("Failed to rename settings file: {}", e))?;
        
        Ok(())
    }
}
