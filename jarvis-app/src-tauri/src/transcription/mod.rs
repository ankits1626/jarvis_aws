// Transcription module for real-time speech-to-text
// Implements a hybrid pipeline: Silero VAD → Vosk (partials) → Whisper (finals)

pub mod provider;
pub mod audio_buffer;
pub mod vad;
pub mod vosk_provider;
pub mod whisper_provider;
pub mod whisperkit_provider;
pub mod hybrid_provider;
pub mod audio_router;
pub mod manager;

// Re-export commonly used types (only implemented modules)
pub use provider::{TranscriptionProvider, TranscriptionSegment, TranscriptionConfig, TranscriptionStatus};
pub use audio_buffer::AudioBuffer;
pub use vad::SileroVad;
pub use vosk_provider::VoskProvider;
pub use whisper_provider::WhisperProvider;
pub use whisperkit_provider::WhisperKitProvider;
pub use hybrid_provider::HybridProvider;
pub use audio_router::AudioRouter;
pub use manager::TranscriptionManager;
