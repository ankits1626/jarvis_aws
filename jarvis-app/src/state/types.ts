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
  
  /** Transcription engine: "whisper-rs" (whisper.cpp), "whisperkit" (Apple Neural Engine), or "mlx-omni" (MLX multimodal) */
  transcription_engine: "whisper-rs" | "whisperkit" | "mlx-omni";
  
  /** WhisperKit model name (e.g., "openai_whisper-large-v3_turbo") */
  whisperkit_model: string;

  /** MLX Omni model catalog ID (e.g., "qwen-omni-3b-8bit") for post-recording transcription */
  mlx_omni_model: string;

  /** Audio window duration in seconds (1.0 to 10.0) for batch transcription */
  window_duration: number;
}

/** Intelligence settings matching Rust IntelligenceSettings struct */
export interface IntelligenceSettings {
  /** Provider type: "mlx", "intelligencekit", or "api" */
  provider: string;
  
  /** Active model catalog ID (e.g., "qwen3-8b-4bit") */
  active_model: string;
  
  /** Python executable path (e.g., "python3" or absolute path) */
  python_path: string;
}

/** Co-Pilot settings matching Rust CoPilotSettings struct */
export interface CoPilotSettings {
  /** Whether Co-Pilot starts automatically when recording begins */
  enabled: boolean;
  
  /** Seconds between agent cycles (30-120) */
  cycle_interval: number;
  
  /** Seconds of overlap between consecutive audio chunks (0-15) */
  audio_overlap: number;
  
  /** Whether to write prompt/response logs to disk */
  agent_logging: boolean;
}

/** Main settings structure matching Rust Settings struct */
export interface Settings {
  /** Transcription-specific settings */
  transcription: TranscriptionSettings;
  
  /** Intelligence-specific settings */
  intelligence: IntelligenceSettings;
  
  /** Co-Pilot agent settings */
  copilot: CoPilotSettings;
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

/** LLM model information matching Rust LlmModelInfo struct */
export interface LlmModelInfo {
  /** Model catalog ID (e.g., "qwen3-8b-4bit") */
  id: string;
  
  /** Human-readable display name (e.g., "Qwen 3 8B (Q4)") */
  display_name: string;
  
  /** HuggingFace repo ID (e.g., "mlx-community/Qwen3-8B-4bit") */
  repo_id: string;
  
  /** Short description of the model */
  description: string;
  
  /** Estimated download size (e.g., "~5 GB") */
  size_estimate: string;
  
  /** Quality tier: "basic", "good", "great", or "best" */
  quality_tier: string;
  
  /** Current status of the model */
  status: ModelStatus;
  
  /** Model capabilities (e.g., ["text"], ["audio", "text"]) */
  capabilities: string[];
}

/** Payload for llm-model-download-progress event */
export interface LlmModelProgressEvent {
  /** Model catalog ID */
  model_id: string;
  
  /** Download progress (0.0 to 100.0) */
  progress: number;
  
  /** Downloaded size in MB */
  downloaded_mb: number;
}

/** Payload for llm-model-download-complete event */
export interface LlmModelDownloadCompleteEvent {
  /** Model catalog ID */
  model_id: string;
}

/** Payload for llm-model-download-error event */
export interface LlmModelDownloadErrorEvent {
  /** Model catalog ID */
  model_id: string;
  
  /** Error message */
  error: string;
}

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

/** Claude panel detection status matching Rust ClaudePanelStatus struct */
export interface ClaudePanelStatus {
  detected: boolean;
  active_tab_url: string | null;
  needs_accessibility: boolean;
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

/**
 * Gems types
 * 
 * These types define the structure for the persistent knowledge base (gems)
 */

/** Full gem representation matching Rust Gem struct */
export interface Gem {
  /** Unique identifier (UUID v4) */
  id: string;
  
  /** Source classification (YouTube, Article, Email, Chat, etc.) */
  source_type: string;
  
  /** Original URL (unique constraint) */
  source_url: string;
  
  /** Domain extracted from URL (e.g., "youtube.com", "medium.com") */
  domain: string;
  
  /** Page/video/article title */
  title: string;
  
  /** Author/channel name (optional) */
  author: string | null;
  
  /** Short description or summary (optional) */
  description: string | null;
  
  /** Full extracted content (optional) */
  content: string | null;
  
  /** Source-specific metadata (JSON, e.g., video duration, email thread ID) */
  source_meta: Record<string, unknown>;
  
  /** ISO 8601 timestamp when gem was captured */
  captured_at: string;
  
  /** AI-generated enrichment metadata (tags, summary, provider info) */
  ai_enrichment: {
    /** AI-generated topic tags (1-5 tags) */
    tags: string[];
    
    /** AI-generated one-sentence summary */
    summary: string;
    
    /** Provider that generated the enrichment (e.g., "mlx", "intelligencekit") */
    provider: string;

    /** Model used for enrichment (e.g., "qwen3-8b-4bit"), only for MLX provider */
    model?: string;

    /** ISO 8601 timestamp when enrichment was generated */
    enriched_at: string;
  } | null;
  
  /** MLX Omni-generated transcript (separate from Whisper real-time transcript) */
  transcript: string | null;
  
  /** Language detected by MLX Omni during transcription (ISO 639-1 code) */
  transcript_language: string | null;
}

/** Lightweight gem for list/search results matching Rust GemPreview struct */
export interface GemPreview {
  /** Unique identifier (UUID v4) */
  id: string;
  
  /** Source classification (YouTube, Article, Email, Chat, etc.) */
  source_type: string;
  
  /** Original URL (unique constraint) */
  source_url: string;
  
  /** Domain extracted from URL (e.g., "youtube.com", "medium.com") */
  domain: string;
  
  /** Page/video/article title */
  title: string;
  
  /** Author/channel name (optional) */
  author: string | null;
  
  /** Short description or summary (optional) */
  description: string | null;
  
  /** Content truncated to 200 characters */
  content_preview: string | null;
  
  /** ISO 8601 timestamp when gem was captured */
  captured_at: string;
  
  /** AI-generated topic tags (1-5 tags, extracted from ai_enrichment) */
  tags: string[] | null;
  
  /** AI-generated one-sentence summary (extracted from ai_enrichment) */
  summary: string | null;

  /** Source of enrichment, e.g. "mlx / qwen3-8b-4bit" (extracted from ai_enrichment) */
  enrichment_source: string | null;
  
  /** Language detected by MLX Omni during transcription (ISO 639-1 code) */
  transcript_language: string | null;
}

/**
 * Transcript types
 * 
 * These types define the structure for recording transcription results
 */

/** Transcript result matching Rust TranscriptResult struct */
export interface TranscriptResult {
  /** Detected language code (e.g., "en", "es", "zh") */
  language: string;
  
  /** Full transcript text */
  transcript: string;
}

/** Recording transcription state for UI management */
export interface RecordingTranscriptionState {
  /** Whether this recording is currently being transcribed */
  transcribing: boolean;
  
  /** Transcript result if transcription completed successfully */
  transcript?: TranscriptResult;
  
  /** Error message if transcription failed */
  transcriptError?: string;
  
  /** Whether this recording has an associated gem */
  hasGem: boolean;
  
  /** Whether gem is currently being saved */
  savingGem: boolean;
  
  /** Whether gem was successfully saved (for success indicator) */
  gemSaved: boolean;
  
  /** Error message if gem save failed */
  gemError?: string;
}

/**
 * IntelligenceKit types
 * 
 * These types define the structure for AI enrichment availability and status
 */

/** AI enrichment availability status matching Rust AvailabilityResult struct */
export interface AvailabilityResult {
  /** Whether AI enrichment is available on this system */
  available: boolean;
  
  /** Reason why AI enrichment is unavailable (undefined if available) */
  reason?: string;
}

/** MLX dependencies diagnostic information matching Rust MlxDiagnostics struct */
export interface MlxDiagnostics {
  /** Whether Python was found at the configured path */
  python_found: boolean;

  /** Python version string if found (e.g., "Python 3.11.5") */
  python_version?: string;

  /** Error message if Python check failed */
  python_error?: string;

  /** Venv status: "not_created", "ready", or "needs_update" */
  venv_status: string;

  /** Path to the venv Python binary if venv exists */
  venv_python_path?: string;
}

/** Progress event during MLX venv setup */
export interface MlxVenvProgressEvent {
  phase: 'creating_venv' | 'installing_deps' | 'validating';
  message: string;
}

/**
 * Co-Pilot Agent types
 * 
 * These types define the structure for the live recording intelligence system
 */

/** Co-Pilot status type */
export type CoPilotStatus = "idle" | "starting" | "processing" | "stopped" | "error";

/** Suggested question from Co-Pilot matching Rust SuggestedQuestion struct */
export interface SuggestedQuestion {
  /** The question text */
  question: string;
  
  /** Reason why this question is relevant */
  reason: string;
  
  /** Cycle number when this question was added */
  cycle_added: number;
  
  /** Whether the user has dismissed this question */
  dismissed: boolean;
}

/** Key concept from Co-Pilot matching Rust KeyConcept struct */
export interface KeyConcept {
  /** The concept term */
  term: string;
  
  /** Context where this concept appeared */
  context: string;
  
  /** Cycle number when this concept was first added */
  cycle_added: number;
  
  /** Number of times this concept has been mentioned */
  mention_count: number;
}

/** Cycle metadata matching Rust CycleMetadata struct */
export interface CycleMetadata {
  /** Current cycle number */
  cycle_number: number;
  
  /** ISO 8601 timestamp of last update */
  last_updated_at: string;
  
  /** Whether a cycle is currently processing */
  processing: boolean;
  
  /** Number of failed cycles */
  failed_cycles: number;
  
  /** Total audio analyzed in seconds */
  total_audio_seconds: number;
}

/** Co-Pilot state matching Rust CoPilotState struct */
export interface CoPilotState {
  /** Rolling summary of the conversation */
  running_summary: string;
  
  /** Key points extracted from the conversation */
  key_points: string[];
  
  /** Decisions made during the conversation */
  decisions: string[];
  
  /** Action items identified */
  action_items: string[];
  
  /** Open questions that need answers */
  open_questions: string[];
  
  /** Suggested questions for the user (max 5) */
  suggested_questions: SuggestedQuestion[];
  
  /** Key concepts mentioned in the conversation */
  key_concepts: KeyConcept[];
  
  /** Cycle metadata */
  cycle_metadata: CycleMetadata;
}

/** Payload for copilot-updated event */
export interface CoPilotUpdatedEvent {
  state: CoPilotState;
}

/** Payload for copilot-status event */
export interface CoPilotStatusEvent {
  status: CoPilotStatus;
}

/** Payload for copilot-error event */
export interface CoPilotErrorEvent {
  message: string;
}

/**
 * Co-Pilot Card Stack UX types
 * 
 * These types define the structure for the card-based UX redesign
 */

/** Card type classification */
export type CoPilotCardType = 'insight' | 'decision' | 'action_item' | 'question' | 'summary_update';

/** Running status for the card stack component */
export type CoPilotRunningStatus = 'idle' | 'recording' | 'processing' | 'complete';

/** Individual card in the card stack */
export interface CoPilotCard {
  /** Unique identifier: "cycle-{N}-{type}-{index}" */
  id: string;
  
  /** Card type classification */
  type: CoPilotCardType;
  
  /** Short title extracted from body (max 60 chars) */
  title: string;
  
  /** Full insight text */
  body: string;
  
  /** Cycle number that produced this card */
  cycle: number;
  
  /** Unix timestamp when card was created */
  timestamp: number;
  
  /** Whether card is currently expanded */
  isExpanded: boolean;
  
  /** Whether card is new (true until auto-collapse or user interaction) */
  isNew: boolean;
}

/** Final summary card displayed when recording stops */
export interface FinalSummaryCard {
  /** Final running summary text */
  summary: string;
  
  /** Accumulated key points */
  keyTakeaways: string[];
  
  /** Accumulated action items */
  actionItems: string[];
  
  /** Accumulated decisions */
  decisions: string[];
  
  /** Accumulated open questions */
  openQuestions: string[];
}

/** Component state for CoPilotCardStack */
export interface CoPilotCardStackState {
  /** All cards created from CoPilotState updates */
  cards: CoPilotCard[];
  
  /** Current running status */
  runningStatus: CoPilotRunningStatus;
  
  /** Current cycle number */
  currentCycle: number;
  
  /** Seconds until next cycle */
  nextCycleIn: number;
  
  /** Total audio analyzed in seconds */
  totalAudioAnalyzed: number;
  
  /** Final summary card (null until recording stops) */
  finalSummaryCard: FinalSummaryCard | null;
}

/**
 * Knowledge File types
 * 
 * These types define the structure for gem knowledge files
 */

/** Knowledge subfile metadata matching Rust KnowledgeSubfile struct */
export interface KnowledgeSubfile {
  /** Filename (e.g., "gem.md", "content.md", "enrichment.md") */
  filename: string;
  
  /** Whether the file exists on disk */
  exists: boolean;
  
  /** File size in bytes */
  size_bytes: number;
  
  /** ISO 8601 timestamp of last modification (null if file doesn't exist) */
  last_modified: string | null;
}

/** Knowledge entry matching Rust KnowledgeEntry struct */
export interface KnowledgeEntry {
  /** Gem ID this knowledge belongs to */
  gem_id: string;
  
  /** Assembled gem.md content */
  assembled: string;
  
  /** Metadata for all known subfiles */
  subfiles: KnowledgeSubfile[];
  
  /** Knowledge schema version */
  version: number;
  
  /** ISO 8601 timestamp when knowledge was last assembled */
  last_assembled: string;
}
