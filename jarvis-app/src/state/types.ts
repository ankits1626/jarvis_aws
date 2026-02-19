/**
 * Type definitions for JarvisApp state management
 * 
 * These types define the application state, actions, and event payloads
 * for the React frontend. They match the Rust backend types where applicable.
 */

/**
 * Recording state enum
 * - idle: No recording in progress, ready to start
 * - recording: Currently recording audio
 * - processing: Transitioning between states (starting/stopping)
 */
export type RecordingState = "idle" | "recording" | "processing";

/**
 * Recording metadata matching Rust RecordingMetadata struct
 * 
 * This interface matches the Rust type in src-tauri/src/files.rs
 */
export interface RecordingMetadata {
  /** Filename of the recording (e.g., "20240315_143022.pcm") */
  filename: string;
  
  /** Size of the recording file in bytes */
  size_bytes: number;
  
  /** Unix timestamp (seconds since epoch) when the recording was created */
  created_at: number;
  
  /** Duration of the recording in seconds, calculated from file size */
  duration_seconds: number;
}

/**
 * Transcription segment matching Rust TranscriptionSegment struct
 * 
 * This interface matches the Rust type in src-tauri/src/transcription/provider.rs
 */
export interface TranscriptionSegment {
  /** Transcribed text */
  text: string;
  
  /** Start time in milliseconds */
  start_ms: number;
  
  /** End time in milliseconds */
  end_ms: number;
  
  /** false = Vosk partial (gray text), true = Whisper final (normal text) */
  is_final: boolean;
}

/**
 * Transcription status matching Rust TranscriptionStatus enum
 * 
 * This type matches the Rust enum in src-tauri/src/transcription/provider.rs
 * - idle: Not currently transcribing
 * - active: Currently transcribing
 * - error: An error occurred
 * - disabled: Transcription is disabled (models not available)
 */
export type TranscriptionStatus = "idle" | "active" | "error" | "disabled";

/**
 * Application state interface
 * 
 * Represents the complete state of the application, managed by the reducer
 */
export interface AppState {
  /** Current recording state (idle, recording, or processing) */
  recordingState: RecordingState;
  
  /** Filename of the current recording (null if not recording) */
  currentRecording: string | null;
  
  /** List of all available recordings */
  recordings: RecordingMetadata[];
  
  /** Filename of the currently selected recording for playback (null if none) */
  selectedRecording: string | null;
  
  /** Current error message (null if no error) */
  error: string | null;
  
  /** Elapsed time in seconds for the current recording */
  elapsedTime: number;
  
  /** Whether the permission dialog should be shown */
  showPermissionDialog: boolean;
  
  /** Current transcription status */
  transcriptionStatus: TranscriptionStatus;
  
  /** Accumulated transcript segments */
  transcript: TranscriptionSegment[];
  
  /** Current transcription error message (null if no error) */
  transcriptionError: string | null;
}

/**
 * Application action union type
 * 
 * Defines all possible actions that can be dispatched to the reducer
 */
export type AppAction =
  | { type: "START_RECORDING" }
  | { type: "RECORDING_STARTED"; filename: string }
  | { type: "STOP_RECORDING" }
  | { type: "RECORDING_STOPPED" }
  | { type: "SET_RECORDINGS"; recordings: RecordingMetadata[] }
  | { type: "SELECT_RECORDING"; filename: string }
  | { type: "DESELECT_RECORDING" }
  | { type: "REMOVE_RECORDING"; filename: string }
  | { type: "SET_ERROR"; error: string }
  | { type: "CLEAR_ERROR" }
  | { type: "SHOW_PERMISSION_DIALOG" }
  | { type: "HIDE_PERMISSION_DIALOG" }
  | { type: "TICK_TIMER" }
  | { type: "RESET_TIMER" }
  | { type: "TRANSCRIPTION_STARTED" }
  | { type: "TRANSCRIPTION_UPDATE"; segment: TranscriptionSegment }
  | { type: "TRANSCRIPTION_STOPPED"; transcript: TranscriptionSegment[] }
  | { type: "TRANSCRIPTION_ERROR"; message: string }
  | { type: "CLEAR_TRANSCRIPT" };

/**
 * Event payload types for Tauri events
 * 
 * These types define the structure of event payloads emitted by the Rust backend
 */

/** Payload for recording-started event */
export interface RecordingStartedEvent {
  filename: string;
}

/** Payload for error events (permission-error, sidecar-error) */
export interface ErrorEvent {
  message: string;
}

/** Payload for shortcut-triggered event */
export interface ShortcutEvent {
  action: "start" | "stop";
}

/** Payload for sidecar-crashed event */
export interface CrashedEvent {
  code: number | null;
}

/**
 * Transcription event payload types
 * 
 * These types define the structure of transcription event payloads emitted by the Rust backend
 */

/** Payload for transcription-update event */
export interface TranscriptionUpdateEvent {
  segment: TranscriptionSegment;
}

/** Payload for transcription-stopped event */
export interface TranscriptionStoppedEvent {
  transcript: TranscriptionSegment[];
}

/** Payload for transcription-error event */
export interface TranscriptionErrorEvent {
  message: string;
}

/**
 * Settings types
 * 
 * These types define the structure of application settings and model management
 */

/** Transcription settings matching Rust TranscriptionSettings struct */
export interface TranscriptionSettings {
  /** Whether VAD (Voice Activity Detection) is enabled */
  vad_enabled: boolean;
  
  /** VAD threshold (0.0 to 1.0) */
  vad_threshold: number;
  
  /** Whether Vosk is enabled for instant partials */
  vosk_enabled: boolean;
  
  /** Whether Whisper is enabled (always true, not user-configurable) */
  whisper_enabled: boolean;
  
  /** Whisper model filename (e.g., "ggml-base.en.bin") */
  whisper_model: string;
  
  /** Transcription engine: "whisper-rs" (whisper.cpp) or "whisperkit" (Apple Neural Engine) */
  transcription_engine: "whisper-rs" | "whisperkit";
  
  /** WhisperKit model name (e.g., "openai_whisper-large-v3_turbo") */
  whisperkit_model: string;

  /** Audio window duration in seconds (1.0 to 10.0) for batch transcription */
  window_duration: number;
}

/** Main settings structure matching Rust Settings struct */
export interface Settings {
  /** Transcription-specific settings */
  transcription: TranscriptionSettings;
}

/** Model status enum matching Rust ModelStatus */
export type ModelStatus = 
  | { type: "downloaded"; size_bytes: number }
  | { type: "downloading"; progress: number }
  | { type: "not_downloaded" }
  | { type: "error"; message: string };

/** Model information matching Rust ModelInfo struct */
export interface ModelInfo {
  /** Model filename (e.g., "ggml-base.en.bin") */
  filename: string;

  /** Human-readable display name (e.g., "Large V3 Turbo Q5") */
  display_name: string;

  /** Short description of the model */
  description: string;

  /** Estimated download size (e.g., "547 MB") */
  size_estimate: string;

  /** Quality tier: "basic", "good", "great", or "best" */
  quality_tier: string;

  /** Current status of the model */
  status: ModelStatus;
}

/** Payload for model-download-progress event */
export interface ModelProgressEvent {
  /** Model filename */
  model_name: string;
  
  /** Download progress (0.0 to 100.0) */
  progress: number;
}

/** Payload for model-download-complete event */
export interface ModelDownloadCompleteEvent {
  /** Model filename */
  model_name: string;
}

/** Payload for model-download-error event */
export interface ModelDownloadErrorEvent {
  /** Model filename */
  model_name: string;
  
  /** Error message */
  error: string;
}

/** Payload for settings-changed event */
export type SettingsChangedEvent = Settings;

/** WhisperKit availability status matching Rust WhisperKitStatus struct */
export interface WhisperKitStatus {
  /** Whether WhisperKit is available on this system */
  available: boolean;
  
  /** Reason why WhisperKit is unavailable (null if available) */
  reason?: string;
}

/**
 * YouTube Browser Vision types
 * 
 * These types define the structure for YouTube video detection and metadata
 */

/** YouTube video metadata (gist) matching Rust YouTubeGist struct */
export interface YouTubeGist {
  /** Full YouTube URL */
  url: string;
  
  /** 11-character video ID */
  video_id: string;
  
  /** Video title */
  title: string;
  
  /** Channel name */
  channel: string;
  
  /** Video description */
  description: string;
  
  /** Video duration in seconds */
  duration_seconds: number;
}

/**
 * Browser Tool types
 *
 * These types define the structure for the Browser Tool tab listing and gist extraction
 */

/** Source type classification matching Rust SourceType enum */
export type SourceType = 'YouTube' | 'Article' | 'Code' | 'Docs' | 'Email' | 'Chat' | 'QA' | 'News' | 'Research' | 'Social' | 'Other';

/** Browser tab info matching Rust BrowserTab struct */
export interface BrowserTab {
  url: string;
  title: string;
  source_type: SourceType;
  domain: string;
}

/** Page gist (extracted metadata) matching Rust PageGist struct */
export interface PageGist {
  url: string;
  title: string;
  source_type: SourceType;
  domain: string;
  author: string | null;
  description: string | null;
  content_excerpt: string | null;
  published_date: string | null;
  image_url: string | null;
  extra: Record<string, unknown>;
}

/** Payload for youtube-video-detected event */
export interface YouTubeDetectedEvent {
  /** Full YouTube URL */
  url: string;
  
  /** 11-character video ID */
  video_id: string;
  
  /** Video title (from oEmbed API, optional) */
  title?: string;
  
  /** Channel/author name (from oEmbed API, optional) */
  author_name?: string;
}
