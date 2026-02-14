use std::fmt::{self, Display, Formatter};

/// Application error types for JarvisApp
#[derive(Debug)]
pub enum AppError {
    /// Failed to spawn the JarvisListen sidecar process
    SidecarSpawnFailed(String),
    
    /// The JarvisListen sidecar process crashed during recording
    SidecarCrashed(String),
    
    /// File I/O operation failed (read, write, delete)
    FileIOError(String),
    
    /// Permission denied (Screen Recording or Microphone access)
    PermissionDenied(String),
    
    /// Platform is not supported for recording
    PlatformNotSupported,
    
    /// Recording file is invalid or corrupted
    InvalidRecording(String),
    
    /// Attempted to start recording while already recording
    ConcurrentRecording,
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            AppError::SidecarSpawnFailed(msg) => {
                write!(f, "Failed to start audio capture: {}", msg)
            }
            AppError::SidecarCrashed(msg) => {
                write!(f, "Audio capture crashed unexpectedly: {}", msg)
            }
            AppError::FileIOError(msg) => {
                write!(f, "File operation failed: {}", msg)
            }
            AppError::PermissionDenied(msg) => {
                write!(f, "Permission denied: {}", msg)
            }
            AppError::PlatformNotSupported => {
                write!(f, "Recording is not yet supported on this platform. Currently only macOS is supported.")
            }
            AppError::InvalidRecording(msg) => {
                write!(f, "Invalid recording: {}", msg)
            }
            AppError::ConcurrentRecording => {
                write!(f, "A recording is already in progress")
            }
        }
    }
}

impl std::error::Error for AppError {}
