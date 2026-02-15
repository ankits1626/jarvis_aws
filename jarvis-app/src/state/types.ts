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
