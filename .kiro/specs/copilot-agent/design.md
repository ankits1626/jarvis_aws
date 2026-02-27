# Design Document: Co-Pilot Agent — Live Recording Intelligence

## Overview

The Co-Pilot Agent is a live recording intelligence system that runs alongside audio recording and produces high-quality, actionable output in real-time. Unlike the live transcription system (which uses Whisper/WhisperKit for word-level accuracy), the Co-Pilot bypasses transcript quality issues by feeding raw audio chunks directly to Qwen Omni (a multimodal model that understands audio natively), then aggregating results across cycles to maintain running understanding of the entire conversation.

The system provides:
- Rolling summary of conversation content
- Suggested questions to ask next
- Key concept alerts (technical terms, names, topics)
- Decisions and action items tracking
- Open questions identification

The Co-Pilot operates on a configurable cycle (default: 60 seconds), extracting audio chunks with overlap to bridge sentence boundaries, calling the `IntelProvider::copilot_analyze()` trait method for analysis, and emitting real-time updates to the frontend. The agent uses self-compressing aggregation where each cycle's output becomes the context for the next cycle, preventing unbounded memory growth.

### Key Design Goals

1. **Real-time intelligence**: Provide actionable insights during the conversation, not just after
2. **High quality**: Use multimodal LLM (Qwen Omni) to bypass live transcript quality issues
3. **User control**: Optional feature with start/stop controls, configurable cycle intervals
4. **Resource efficiency**: Share IntelProvider with gem enrichment, skip cycles when provider is busy
5. **Auditability**: Full prompt/response logging for quality tuning and debugging

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Jarvis App (Tauri)                          │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (React)                                                │
│  ├─ Recording Screen                                             │
│  │  └─ Co-Pilot Toggle (start/stop agent)                       │
│  └─ Right Panel                                                  │
│     ├─ Transcript Tab (existing)                                 │
│     └─ Co-Pilot Tab (NEW)                                        │
│        └─ CoPilotPanel Component                                 │
│           ├─ Summary Section                                     │
│           ├─ Decisions & Action Items                            │
│           ├─ Suggested Questions                                 │
│           └─ Key Concepts                                        │
├─────────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                                  │
│  ├─ Commands                                                     │
│  │  ├─ start_copilot()                                           │
│  │  ├─ stop_copilot()                                            │
│  │  ├─ get_copilot_state()                                       │
│  │  └─ dismiss_copilot_question(index)                           │
│  ├─ Agents Module (NEW)                                          │
│  │  └─ CoPilotAgent                                              │
│  │     ├─ Cycle loop (tokio background task)                     │
│  │     ├─ Audio chunk extraction                                 │
│  │     ├─ IntelProvider integration (copilot_analyze)             │
│  │     ├─ State aggregation                                      │
│  │     └─ Agent logging                                          │
│  ├─ Recording Module (existing)                                  │
│  │  └─ RecordingManager (writes PCM file)                        │
│  ├─ Intelligence Module (existing, extended)                     │
│  │  ├─ IntelProvider trait (+ copilot_analyze method)            │
│  │  └─ MlxProvider (implements copilot_analyze via sidecar)      │
│  ├─ Settings Module (extended)                                   │
│  │  └─ CopilotSettings                                           │
│  └─ Gems Module (extended)                                       │
│     └─ Save copilot data to source_meta.copilot                  │
├─────────────────────────────────────────────────────────────────┤
│  Python MLX Sidecar (server.py)                                 │
│  ├─ copilot-analyze command (NEW)                                │
│  │  ├─ Load audio file (WAV)                                     │
│  │  ├─ Construct analysis prompt with context                    │
│  │  └─ Return structured JSON (summary, questions, concepts)     │
│  └─ Existing commands (generate-tags, summarize, transcribe)     │
└─────────────────────────────────────────────────────────────────┘
```


### Agent Lifecycle Flow

```
User clicks Co-Pilot toggle ON
    ↓
Frontend calls start_copilot()
    ↓
Backend checks: recording active?
    ├─ NO → Return error
    └─ YES → Continue
    ↓
Create CoPilotAgent instance
    ↓
Spawn tokio background task for cycle loop
    ↓
┌─────────────────────────────────────────┐
│ Cycle Loop (every 60s by default)      │
├─────────────────────────────────────────┤
│ 1. Extract audio chunk from PCM file   │
│    - Read last (60s + 5s overlap)      │
│    - Convert to WAV format              │
│    - Write to temp file                 │
│                                         │
│ 2. Call provider.copilot_analyze()     │
│    - Timeout: 120s                      │
│    - If timeout → skip cycle            │
│                                         │
│ 3. Send copilot-analyze command        │
│    - audio_path: temp WAV file          │
│    - context: previous cycle's summary  │
│    - Timeout: 120s                      │
│                                         │
│ 4. Parse JSON response                  │
│    - Extract summary, questions, etc.   │
│    - Handle partial/malformed JSON      │
│                                         │
│ 5. Update agent state                   │
│    - Merge new data with existing       │
│    - Deduplicate concepts               │
│    - Update cycle metadata              │
│                                         │
│ 6. Emit copilot-updated event           │
│    - Send full state to frontend        │
│                                         │
│ 7. Append to agent log (if enabled)    │
│    - Write prompt + response            │
│    - Write cycle metadata               │
│                                         │
│ 8. Clean up temp WAV file               │
│                                         │
│ 9. Sleep until next cycle               │
│    - Interval measured from cycle end   │
└─────────────────────────────────────────┘
    ↓
Recording stops OR user toggles OFF
    ↓
Send stop signal to cycle loop
    ↓
Wait for in-flight inference (up to 120s)
    ↓
Write final summary to agent log
    ↓
Return final state
    ↓
Frontend displays final Co-Pilot data
```

### Concurrency Model

The Co-Pilot agent shares the intelligence provider with gem enrichment. Concurrency is managed through:

1. **Provider abstraction**: The Co-Pilot calls `IntelProvider::copilot_analyze()` — the same trait used for tags/summary/transcription. It never references `MlxProvider` directly.
2. **Internal mutex**: The `MlxProvider` implementation manages its own mutex internally. The Co-Pilot doesn't need to know about sidecar locking.
3. **Timeout-based skipping**: If the provider call takes too long (120s timeout), the Co-Pilot skips the cycle.
4. **Non-blocking recording**: Co-Pilot reads PCM file directly, doesn't block recording pipeline.

This design (Option A from requirements) avoids complex scheduling because:
- Real-time transcription uses whisper-rs/WhisperKit (separate from the IntelProvider)
- Co-Pilot and enrichment are both async operations that can wait
- Skipping a Co-Pilot cycle is acceptable (next cycle will catch up)
- Swapping to an API provider requires only implementing `copilot_analyze` on the new provider — no agent code changes


## Components and Interfaces

### 1. CoPilotAgent (Rust)

**Location**: `jarvis-app/src-tauri/src/agents/copilot.rs`

**Responsibilities**:
- Manage agent lifecycle (start/stop)
- Run cycle loop on background tokio task
- Extract audio chunks from recording file
- Call IntelProvider::copilot_analyze() for analysis
- Aggregate results across cycles
- Emit Tauri events for frontend updates
- Write agent logs (if enabled)

**Data Structures**:

```rust
pub struct CoPilotAgent {
    app_handle: AppHandle,
    state: Arc<TokioMutex<CoPilotState>>,
    cycle_task: Option<tokio::task::JoinHandle<()>>,
    stop_tx: Option<watch::Sender<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoPilotState {
    pub running_summary: String,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub action_items: Vec<String>,
    pub open_questions: Vec<String>,
    pub suggested_questions: Vec<SuggestedQuestion>,
    pub key_concepts: Vec<KeyConcept>,
    pub cycle_metadata: CycleMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedQuestion {
    pub question: String,
    pub reason: String,
    pub cycle_added: u32,
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConcept {
    pub term: String,
    pub context: String,
    pub cycle_added: u32,
    pub mention_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetadata {
    pub cycle_number: u32,
    pub last_updated_at: String,  // ISO 8601
    pub processing: bool,
    pub failed_cycles: u32,
    pub total_audio_seconds: u64,
}

impl Default for CoPilotState {
    fn default() -> Self {
        Self {
            running_summary: String::new(),
            key_points: Vec::new(),
            decisions: Vec::new(),
            action_items: Vec::new(),
            open_questions: Vec::new(),
            suggested_questions: Vec::new(),
            key_concepts: Vec::new(),
            cycle_metadata: CycleMetadata {
                cycle_number: 0,
                last_updated_at: String::new(),
                processing: false,
                failed_cycles: 0,
                total_audio_seconds: 0,
            },
        }
    }
}
```

**Methods**:

```rust
impl CoPilotAgent {
    /// Create a new CoPilotAgent instance
    pub fn new(app_handle: AppHandle) -> Self;
    
    /// Start the agent cycle loop
    /// Returns error if no recording is active or agent already running
    pub async fn start(
        &mut self,
        recording_filepath: PathBuf,
        settings: CopilotSettings,
        provider: Arc<dyn IntelProvider>,
    ) -> Result<(), String>;
    
    /// Stop the agent gracefully
    /// Waits for in-flight inference to complete (up to 120s timeout)
    /// Returns final state
    pub async fn stop(&mut self) -> Result<CoPilotState, String>;
    
    /// Get current agent state
    pub async fn get_state(&self) -> CoPilotState;
    
    /// Dismiss a suggested question by index
    pub async fn dismiss_question(&self, index: usize) -> Result<(), String>;
    
    /// Extract audio chunk from recording file
    /// Returns path to temporary WAV file
    async fn extract_audio_chunk(
        &self,
        recording_filepath: &Path,
        cycle_interval: u64,
        audio_overlap: u64,
    ) -> Result<PathBuf, String>;
    
    /// Run one cycle of analysis
    async fn run_cycle(
        &self,
        recording_filepath: &Path,
        settings: &CopilotSettings,
        provider: &Arc<dyn IntelProvider>,
    ) -> Result<CycleResult, String>;
    
    /// Update state with cycle results
    async fn update_state(&self, result: CycleResult);
    
    /// Write cycle to agent log
    async fn log_cycle(
        &self,
        log_path: &Path,
        cycle_number: u32,
        prompt: &str,
        response: &str,
        inference_time: f64,
        status: &str,
    ) -> Result<(), String>;
}
```


### 2. Audio Chunk Extraction

**Algorithm**:

```rust
async fn extract_audio_chunk(
    recording_filepath: &Path,
    cycle_interval: u64,
    audio_overlap: u64,
) -> Result<PathBuf, String> {
    // Calculate chunk duration in seconds
    let chunk_duration = cycle_interval + audio_overlap;
    
    // Calculate byte size for chunk
    // PCM format: 16kHz, 16-bit (2 bytes), mono
    let bytes_per_second = 16000 * 2;
    let chunk_size_bytes = chunk_duration * bytes_per_second;
    
    // Open recording file
    let file = tokio::fs::File::open(recording_filepath).await?;
    let file_size = file.metadata().await?.len();
    
    // Handle case where file is shorter than chunk size (first cycle)
    let read_size = std::cmp::min(chunk_size_bytes, file_size);
    let start_offset = file_size.saturating_sub(read_size);
    
    // Read chunk from end of file
    file.seek(SeekFrom::Start(start_offset)).await?;
    let mut chunk_data = vec![0u8; read_size as usize];
    file.read_exact(&mut chunk_data).await?;
    
    // Generate unique temp filename
    let temp_path = std::env::temp_dir()
        .join(format!("jarvis_copilot_chunk_{}.wav", uuid::Uuid::new_v4()));
    
    // Convert PCM to WAV using existing convert_to_wav logic
    // Reuse WAV header generation from recording module
    let wav_data = convert_pcm_to_wav(&chunk_data, 16000, 1)?;
    
    // Write to temp file
    tokio::fs::write(&temp_path, wav_data).await?;
    
    Ok(temp_path)
}
```

**WAV Conversion**:

The system reuses the existing `convert_to_wav` logic from the recording module. The WAV header format is:

```
RIFF header (12 bytes)
  - "RIFF" magic (4 bytes)
  - File size - 8 (4 bytes, little-endian)
  - "WAVE" magic (4 bytes)

fmt chunk (24 bytes)
  - "fmt " magic (4 bytes)
  - Chunk size: 16 (4 bytes)
  - Audio format: 1 (PCM) (2 bytes)
  - Channels: 1 (mono) (2 bytes)
  - Sample rate: 16000 (4 bytes)
  - Byte rate: 32000 (4 bytes)
  - Block align: 2 (2 bytes)
  - Bits per sample: 16 (2 bytes)

data chunk (8 bytes + PCM data)
  - "data" magic (4 bytes)
  - Data size (4 bytes)
  - PCM samples (variable)
```

### 3. IntelProvider Trait Extension

**Location**: `jarvis-app/src-tauri/src/intelligence/provider.rs`

The `IntelProvider` trait is extended with a `copilot_analyze` method so the Co-Pilot agent calls the same abstraction used for tag generation, summarization, and transcription. This is the single point of change if the backend switches to an API provider.

**New types**:

```rust
/// Result of a Co-Pilot analysis cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoPilotCycleResult {
    pub new_content: String,
    pub updated_summary: String,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub action_items: Vec<String>,
    pub open_questions: Vec<String>,
    pub suggested_questions: Vec<CoPilotQuestion>,
    pub key_concepts: Vec<CoPilotConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoPilotQuestion {
    pub question: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoPilotConcept {
    pub term: String,
    pub context: String,
}
```

**Trait extension**:

```rust
#[async_trait]
pub trait IntelProvider: Send + Sync {
    // ... existing methods (check_availability, generate_tags, summarize, generate_transcript) ...

    /// Analyze an audio chunk with running context for Co-Pilot
    ///
    /// Processes an audio file alongside a text summary of the conversation so far,
    /// and returns structured analysis (summary, questions, concepts, etc.).
    /// Default implementation returns error for providers that don't support audio analysis.
    ///
    /// # Arguments
    ///
    /// * `audio_path` - Path to the audio chunk file (.wav format)
    /// * `context` - Running context (previous cycle's summary, empty for first cycle)
    ///
    /// # Returns
    ///
    /// * `Ok(CoPilotCycleResult)` - Structured analysis of the audio chunk
    /// * `Err(String)` - Error message if analysis fails or is not supported
    async fn copilot_analyze(
        &self,
        _audio_path: &std::path::Path,
        _context: &str,
    ) -> Result<CoPilotCycleResult, String> {
        Err("Co-Pilot analysis not supported by this provider".to_string())
    }
}
```

The `MlxProvider` implements this method by sending the `copilot-analyze` NDJSON command to its sidecar process. The `IntelligenceKitProvider` and `NoOpProvider` use the default (unsupported) implementation.

### 4. MLX Sidecar — copilot-analyze Command

The `MlxProvider::copilot_analyze` implementation sends a `copilot-analyze` NDJSON command to the sidecar. This is the MLX-specific implementation detail hidden behind the `IntelProvider` trait.

**New Command**: `copilot-analyze`

**Request Format**:
```json
{
  "command": "copilot-analyze",
  "audio_path": "/tmp/jarvis_copilot_chunk_abc123.wav",
  "context": "Previous summary text or empty string for first cycle"
}
```

**Response Format**:
```json
{
  "type": "response",
  "command": "copilot-analyze",
  "new_content": "What was discussed in this audio segment",
  "updated_summary": "Full conversation summary incorporating new content",
  "key_points": ["Point 1", "Point 2"],
  "decisions": ["Decision 1"],
  "action_items": ["Action 1", "Action 2"],
  "open_questions": ["Question 1"],
  "suggested_questions": [
    {"question": "Can you clarify X?", "reason": "Mentioned but not explained"},
    {"question": "What about Y?", "reason": "Related topic not covered"}
  ],
  "key_concepts": [
    {"term": "Technical Term", "context": "Brief explanation"},
    {"term": "Person Name", "context": "Role or relevance"}
  ]
}
```

**Error Response**:
```json
{
  "type": "error",
  "command": "copilot-analyze",
  "error": "Error message"
}
```

**Python Implementation** (in `server.py`):

```python
def copilot_analyze(self, audio_path: str, context: str) -> Dict[str, Any]:
    """Analyze audio chunk with running context."""
    if self.model is None:
        return {
            "type": "error",
            "command": "copilot-analyze",
            "error": "No model loaded"
        }
    
    # Check capabilities
    if "audio" not in self.capabilities:
        return {
            "type": "error",
            "command": "copilot-analyze",
            "error": "Model does not support audio analysis"
        }
    
    try:
        # Load audio file (reuse existing audio loading logic)
        audio, sr = librosa.load(audio_path, sr=16000, mono=True)
        
        # Construct prompt
        if context:
            prompt_text = f"""Previous conversation summary:
{context}

Analyze the new audio segment and provide:
1. What new content was discussed
2. Updated summary of the entire conversation so far
3. Key points mentioned
4. Any decisions made
5. Action items identified
6. Open questions raised
7. Suggested questions to ask next (with reasons)
8. Key concepts (technical terms, names, topics) with brief context

Respond in JSON format with these exact fields:
{{"new_content": "...", "updated_summary": "...", "key_points": [...], "decisions": [...], "action_items": [...], "open_questions": [...], "suggested_questions": [{{"question": "...", "reason": "..."}}], "key_concepts": [{{"term": "...", "context": "..."}}]}}"""
        else:
            prompt_text = """This is the start of a conversation. Analyze the audio and provide:
1. What was discussed
2. Summary of the conversation
3. Key points mentioned
4. Any decisions made
5. Action items identified
6. Open questions raised
7. Suggested questions to ask next (with reasons)
8. Key concepts (technical terms, names, topics) with brief context

Respond in JSON format with these exact fields:
{{"new_content": "...", "updated_summary": "...", "key_points": [...], "decisions": [...], "action_items": [...], "open_questions": [...], "suggested_questions": [{{"question": "...", "reason": "..."}}], "key_concepts": [{{"term": "...", "context": "..."}}]}}"""
        
        # Build messages with audio
        messages = [
            {"role": "user", "content": prompt_text, "audio": audio}
        ]
        token_ids = self.tokenizer.apply_chat_template(messages, add_generation_prompt=True)
        
        # Generate response
        response = mlx_omni_generate(
            self.model,
            self.tokenizer,
            prompt=token_ids,
            max_tokens=2000,
            prefill_step_size=32768,
            verbose=False
        )
        
        # Parse JSON response
        response_text = response.strip()
        
        # Strip markdown code fence if present
        if response_text.startswith("```"):
            lines = response_text.split("\n")
            lines = [l for l in lines if not l.strip().startswith("```")]
            response_text = "\n".join(lines).strip()
        
        try:
            parsed = json.loads(response_text)
            
            # Validate required fields, provide defaults for missing
            result = {
                "type": "response",
                "command": "copilot-analyze",
                "new_content": parsed.get("new_content", ""),
                "updated_summary": parsed.get("updated_summary", ""),
                "key_points": parsed.get("key_points", []),
                "decisions": parsed.get("decisions", []),
                "action_items": parsed.get("action_items", []),
                "open_questions": parsed.get("open_questions", []),
                "suggested_questions": parsed.get("suggested_questions", []),
                "key_concepts": parsed.get("key_concepts", [])
            }
            
            return result
            
        except json.JSONDecodeError:
            # Partial JSON - return what we can parse
            return {
                "type": "response",
                "command": "copilot-analyze",
                "new_content": response_text,
                "updated_summary": response_text,
                "key_points": [],
                "decisions": [],
                "action_items": [],
                "open_questions": [],
                "suggested_questions": [],
                "key_concepts": []
            }
    
    except Exception as e:
        return {
            "type": "error",
            "command": "copilot-analyze",
            "error": str(e)
        }
```


### 5. State Aggregation

**Merge Strategy**:

When a new cycle completes, the system merges results with existing state:

```rust
async fn update_state(&self, result: CycleResult) {
    let mut state = self.state.lock().await;
    
    // Replace summary with latest
    state.running_summary = result.updated_summary;
    
    // Append new items, deduplicate
    for point in result.key_points {
        if !state.key_points.contains(&point) {
            state.key_points.push(point);
        }
    }
    
    for decision in result.decisions {
        if !state.decisions.contains(&decision) {
            state.decisions.push(decision);
        }
    }
    
    for item in result.action_items {
        if !state.action_items.contains(&item) {
            state.action_items.push(item);
        }
    }
    
    for question in result.open_questions {
        if !state.open_questions.contains(&question) {
            state.open_questions.push(question);
        }
    }
    
    // Replace suggested questions (keep max 5, preserve dismissed state)
    let mut new_questions = Vec::new();
    for new_q in result.suggested_questions {
        // Check if this question was previously dismissed
        let was_dismissed = state.suggested_questions.iter()
            .any(|old_q| old_q.question == new_q.question && old_q.dismissed);
        
        new_questions.push(SuggestedQuestion {
            question: new_q.question,
            reason: new_q.reason,
            cycle_added: state.cycle_metadata.cycle_number + 1,
            dismissed: was_dismissed,
        });
    }
    state.suggested_questions = new_questions.into_iter().take(5).collect();
    
    // Merge key concepts (increment mention_count for existing)
    for new_concept in result.key_concepts {
        if let Some(existing) = state.key_concepts.iter_mut()
            .find(|c| c.term.eq_ignore_ascii_case(&new_concept.term)) {
            existing.mention_count += 1;
            // Update context with latest
            existing.context = new_concept.context;
        } else {
            state.key_concepts.push(KeyConcept {
                term: new_concept.term,
                context: new_concept.context,
                cycle_added: state.cycle_metadata.cycle_number + 1,
                mention_count: 1,
            });
        }
    }
    
    // Update metadata
    state.cycle_metadata.cycle_number += 1;
    state.cycle_metadata.last_updated_at = chrono::Utc::now().to_rfc3339();
    state.cycle_metadata.processing = false;
    state.cycle_metadata.total_audio_seconds += result.audio_duration_seconds;
}
```

### 6. CopilotSettings

**Location**: `jarvis-app/src-tauri/src/settings/manager.rs` (extension)

**Data Structure**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotSettings {
    /// Whether Co-Pilot starts automatically when recording begins
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Seconds between agent cycles (30-120)
    #[serde(default = "default_cycle_interval")]
    pub cycle_interval: u64,
    
    /// Seconds of overlap between consecutive audio chunks (0-15)
    #[serde(default = "default_audio_overlap")]
    pub audio_overlap: u64,
    
    /// Whether to write prompt/response logs to disk
    #[serde(default = "default_agent_logging")]
    pub agent_logging: bool,
}

fn default_enabled() -> bool { false }
fn default_cycle_interval() -> u64 { 60 }
fn default_audio_overlap() -> u64 { 5 }
fn default_agent_logging() -> bool { true }

impl Default for CopilotSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            cycle_interval: 60,
            audio_overlap: 5,
            agent_logging: true,
        }
    }
}

// Add to main Settings struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub transcription: TranscriptionSettings,
    #[serde(default)]
    pub browser: BrowserSettings,
    #[serde(default)]
    pub intelligence: IntelligenceSettings,
    #[serde(default)]  // Backward compatibility
    pub copilot: CopilotSettings,
}
```

**Validation**:

```rust
impl SettingsManager {
    fn validate(settings: &Settings) -> Result<(), String> {
        // Existing validations...
        
        // Copilot settings validation
        if settings.copilot.cycle_interval < 30 || settings.copilot.cycle_interval > 120 {
            return Err(format!(
                "cycle_interval must be between 30 and 120 seconds, got {}",
                settings.copilot.cycle_interval
            ));
        }
        
        if settings.copilot.audio_overlap > 15 {
            return Err(format!(
                "audio_overlap must be between 0 and 15 seconds, got {}",
                settings.copilot.audio_overlap
            ));
        }
        
        if settings.copilot.audio_overlap >= settings.copilot.cycle_interval {
            return Err(format!(
                "audio_overlap ({}) must be less than cycle_interval ({})",
                settings.copilot.audio_overlap,
                settings.copilot.cycle_interval
            ));
        }
        
        Ok(())
    }
}
```

### 7. Tauri Commands

**Location**: `jarvis-app/src-tauri/src/commands.rs` (additions)

```rust
#[tauri::command]
async fn start_copilot(
    copilot_agent: State<'_, Arc<TokioMutex<Option<CoPilotAgent>>>>,
    recording_manager: State<'_, Mutex<RecordingManager>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    // Check if recording is active
    let recording_filepath = {
        let manager = recording_manager.lock()
            .map_err(|e| format!("Failed to acquire recording lock: {}", e))?;

        if !manager.is_recording() {
            return Err("No recording in progress".to_string());
        }

        manager.current_filepath()
            .ok_or_else(|| "No recording filepath found".to_string())?
            .clone()
    };

    // Get copilot settings
    let settings = settings_manager.get().copilot;

    // Check if agent already running
    let mut agent_guard = copilot_agent.lock().await;
    if agent_guard.is_some() {
        return Err("Co-Pilot agent is already running".to_string());
    }

    // Create and start agent — pass the provider trait object, not MlxProvider directly
    let provider = intel_provider.inner().clone();
    let mut agent = CoPilotAgent::new(app_handle);
    agent.start(recording_filepath, settings, provider).await?;

    *agent_guard = Some(agent);

    Ok(())
}

#[tauri::command]
async fn stop_copilot(
    copilot_agent: State<'_, Arc<TokioMutex<Option<CoPilotAgent>>>>,
) -> Result<CoPilotState, String> {
    let mut agent_guard = copilot_agent.lock().await;
    
    let mut agent = agent_guard.take()
        .ok_or_else(|| "No Co-Pilot agent running".to_string())?;
    
    let final_state = agent.stop().await?;
    
    Ok(final_state)
}

#[tauri::command]
async fn get_copilot_state(
    copilot_agent: State<'_, Arc<TokioMutex<Option<CoPilotAgent>>>>,
) -> Result<CoPilotState, String> {
    let agent_guard = copilot_agent.lock().await;
    
    let agent = agent_guard.as_ref()
        .ok_or_else(|| "No Co-Pilot agent running".to_string())?;
    
    Ok(agent.get_state().await)
}

#[tauri::command]
async fn dismiss_copilot_question(
    index: usize,
    copilot_agent: State<'_, Arc<TokioMutex<Option<CoPilotAgent>>>>,
) -> Result<(), String> {
    let agent_guard = copilot_agent.lock().await;
    
    let agent = agent_guard.as_ref()
        .ok_or_else(|| "No Co-Pilot agent running".to_string())?;
    
    agent.dismiss_question(index).await
}
```


### 8. Agent Logging

**Log File Location**: `~/Library/Application Support/com.jarvis.app/agent_logs/YYYYMMDD_HHMMSS_copilot.md`

**Log Format**:

```markdown
# Co-Pilot Agent Log — 2024-03-15 14:30:22

**Recording:** 20240315_143022.pcm
**Settings:** cycle_interval=60s, audio_overlap=5s
**Model:** Qwen 2.5 Omni 3B (8-bit)

---

## Cycle 1 — 00:00 → 01:05

**Audio chunk:** 0:00–1:05 (65s, includes 5s overlap)
**Inference time:** 8.3s
**Status:** success

### Prompt
Previous conversation summary:
(empty - first cycle)

Analyze the new audio segment and provide:
1. What new content was discussed
2. Updated summary of the entire conversation so far
...

### Response
```json
{
  "new_content": "Discussion about implementing a Co-Pilot feature...",
  "updated_summary": "Team is planning to add a Co-Pilot agent...",
  "key_points": ["Real-time analysis", "Multimodal LLM"],
  ...
}
```

---

## Cycle 2 — 01:00 → 02:05

**Audio chunk:** 1:00–2:05 (65s, includes 5s overlap)
**Inference time:** 7.9s
**Status:** success

### Prompt
Previous conversation summary:
Team is planning to add a Co-Pilot agent...

Analyze the new audio segment and provide:
...

### Response
```json
{
  "new_content": "Discussed technical architecture...",
  "updated_summary": "Team is planning to add a Co-Pilot agent. Architecture will use...",
  ...
}
```

---

## Summary

| Metric | Value |
|---|---|
| Total cycles | 5 |
| Successful | 5 |
| Skipped | 0 |
| Errors | 0 |
| Avg inference time | 8.1s |
| Total recording duration | 5m 22s |
```

**Implementation**:

```rust
async fn log_cycle(
    log_path: &Path,
    cycle_number: u32,
    audio_start: &str,
    audio_end: &str,
    audio_duration: u64,
    audio_overlap: u64,
    prompt: &str,
    response: &str,
    inference_time: f64,
    status: &str,
) -> Result<(), String> {
    use tokio::fs::OpenOptions;
    use tokio::io::AsyncWriteExt;
    
    let entry = format!(
        r#"---

## Cycle {} — {} → {}

**Audio chunk:** {}–{} ({}s, includes {}s overlap)
**Inference time:** {:.1}s
**Status:** {}

### Prompt
{}

### Response
```json
{}
```

"#,
        cycle_number,
        audio_start,
        audio_end,
        audio_start,
        audio_end,
        audio_duration,
        audio_overlap,
        inference_time,
        status,
        prompt,
        response
    );
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    
    file.write_all(entry.as_bytes())
        .await
        .map_err(|e| format!("Failed to write to log file: {}", e))?;
    
    Ok(())
}

async fn write_log_header(
    log_path: &Path,
    recording_filename: &str,
    settings: &CopilotSettings,
    model_name: &str,
) -> Result<(), String> {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    
    let header = format!(
        r#"# Co-Pilot Agent Log — {}

**Recording:** {}
**Settings:** cycle_interval={}s, audio_overlap={}s
**Model:** {}

"#,
        timestamp,
        recording_filename,
        settings.cycle_interval,
        settings.audio_overlap,
        model_name
    );
    
    let mut file = File::create(log_path)
        .await
        .map_err(|e| format!("Failed to create log file: {}", e))?;
    
    file.write_all(header.as_bytes())
        .await
        .map_err(|e| format!("Failed to write log header: {}", e))?;
    
    Ok(())
}

async fn write_log_summary(
    log_path: &Path,
    total_cycles: u32,
    successful_cycles: u32,
    skipped_cycles: u32,
    error_cycles: u32,
    avg_inference_time: f64,
    total_duration: &str,
) -> Result<(), String> {
    use tokio::fs::OpenOptions;
    use tokio::io::AsyncWriteExt;
    
    let summary = format!(
        r#"---

## Summary

| Metric | Value |
|---|---|
| Total cycles | {} |
| Successful | {} |
| Skipped | {} |
| Errors | {} |
| Avg inference time | {:.1}s |
| Total recording duration | {} |
"#,
        total_cycles,
        successful_cycles,
        skipped_cycles,
        error_cycles,
        avg_inference_time,
        total_duration
    );
    
    let mut file = OpenOptions::new()
        .append(true)
        .open(log_path)
        .await
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    
    file.write_all(summary.as_bytes())
        .await
        .map_err(|e| format!("Failed to write log summary: {}", e))?;
    
    Ok(())
}
```

### 9. Gem Integration

**Saving Co-Pilot Data**:

When a recording with active Co-Pilot data is saved as a gem, the final state is included in `source_meta.copilot`:

```rust
// In gem creation logic (when saving recording as gem)
let copilot_data = if let Some(agent_guard) = app_handle.try_state::<Arc<TokioMutex<Option<CoPilotAgent>>>>() {
    let agent = agent_guard.lock().await;
    if let Some(agent) = agent.as_ref() {
        let state = agent.get_state().await;
        Some(serde_json::json!({
            "summary": state.running_summary,
            "key_points": state.key_points,
            "decisions": state.decisions,
            "action_items": state.action_items,
            "open_questions": state.open_questions,
            "key_concepts": state.key_concepts.iter().map(|c| {
                serde_json::json!({
                    "term": c.term,
                    "context": c.context
                })
            }).collect::<Vec<_>>(),
            "total_cycles": state.cycle_metadata.cycle_number,
            "total_audio_analyzed_seconds": state.cycle_metadata.total_audio_seconds
        }))
    } else {
        None
    }
} else {
    None
};

// Add to gem's source_meta
let mut source_meta = serde_json::json!({
    "recording_filename": filename,
    "duration_seconds": duration,
    // ... other metadata
});

if let Some(copilot) = copilot_data {
    source_meta["copilot"] = copilot;
}
```

**GemDetailPanel Display**:

```typescript
// In GemDetailPanel component
interface CoPilotData {
  summary: string;
  key_points: string[];
  decisions: string[];
  action_items: string[];
  open_questions: string[];
  key_concepts: Array<{ term: string; context: string }>;
  total_cycles: number;
  total_audio_analyzed_seconds: number;
}

function GemDetailPanel({ gem }: { gem: Gem }) {
  const copilotData = gem.source_meta?.copilot as CoPilotData | undefined;
  
  return (
    <div className="gem-detail">
      {/* Existing gem details */}
      
      {copilotData && (
        <div className="copilot-section">
          <h3>Co-Pilot Analysis</h3>
          
          <div className="summary">
            <h4>Summary</h4>
            <p>{copilotData.summary}</p>
          </div>
          
          {copilotData.key_points.length > 0 && (
            <div className="key-points">
              <h4>Key Points</h4>
              <ul>
                {copilotData.key_points.map((point, i) => (
                  <li key={i}>{point}</li>
                ))}
              </ul>
            </div>
          )}
          
          {copilotData.decisions.length > 0 && (
            <div className="decisions">
              <h4>Decisions</h4>
              <ul>
                {copilotData.decisions.map((decision, i) => (
                  <li key={i}>✓ {decision}</li>
                ))}
              </ul>
            </div>
          )}
          
          {copilotData.action_items.length > 0 && (
            <div className="action-items">
              <h4>Action Items</h4>
              <ul>
                {copilotData.action_items.map((item, i) => (
                  <li key={i}>{item}</li>
                ))}
              </ul>
            </div>
          )}
          
          {copilotData.key_concepts.length > 0 && (
            <div className="key-concepts">
              <h4>Key Concepts</h4>
              <div className="concepts-grid">
                {copilotData.key_concepts.map((concept, i) => (
                  <div key={i} className="concept-chip" title={concept.context}>
                    {concept.term}
                  </div>
                ))}
              </div>
            </div>
          )}
          
          <div className="copilot-meta">
            <small>
              Analyzed {copilotData.total_cycles} cycles 
              ({Math.floor(copilotData.total_audio_analyzed_seconds / 60)}m 
              {copilotData.total_audio_analyzed_seconds % 60}s of audio)
            </small>
          </div>
        </div>
      )}
      
      {/* Existing AI enrichment section (if present) */}
    </div>
  );
}
```


## Data Models

### Frontend TypeScript Types

**Location**: `jarvis-app/src/state/types.ts` (additions)

```typescript
/**
 * Co-Pilot agent state
 */
export interface CoPilotState {
  running_summary: string;
  key_points: string[];
  decisions: string[];
  action_items: string[];
  open_questions: string[];
  suggested_questions: SuggestedQuestion[];
  key_concepts: KeyConcept[];
  cycle_metadata: CycleMetadata;
}

export interface SuggestedQuestion {
  question: string;
  reason: string;
  cycle_added: number;
  dismissed: boolean;
}

export interface KeyConcept {
  term: string;
  context: string;
  cycle_added: number;
  mention_count: number;
}

export interface CycleMetadata {
  cycle_number: number;
  last_updated_at: string;  // ISO 8601
  processing: boolean;
  failed_cycles: number;
  total_audio_seconds: number;
}

/**
 * Co-Pilot status
 */
export type CoPilotStatus = 
  | "starting"
  | "active"
  | "processing"
  | "paused"
  | "stopped"
  | "error";
```

### App.tsx State Management

**Pattern**: The Co-Pilot state follows the existing pattern in App.tsx using `useState` hooks and Tauri event listeners.

**State additions to App.tsx**:

```typescript
function App() {
  // ... existing state ...
  
  // Co-Pilot state (added)
  const [copilotEnabled, setCopilotEnabled] = useState(false);
  const [copilotStatus, setCopilotStatus] = useState<CoPilotStatus>('stopped');
  const [copilotState, setCopilotState] = useState<CoPilotState | null>(null);
  const [copilotError, setCopilotError] = useState<string | null>(null);
  
  // ... rest of component ...
}
```

**Event listeners** (add to App.tsx):

```typescript
// Listen for copilot-updated events
useTauriEvent<CoPilotState>(
  'copilot-updated',
  useCallback((state) => {
    setCopilotState(state);
    setCopilotStatus('active');
  }, [])
);

// Listen for copilot-status events
useTauriEvent<{ status: CoPilotStatus; message?: string }>(
  'copilot-status',
  useCallback((event) => {
    setCopilotStatus(event.status);
  }, [])
);

// Listen for copilot-error events
useTauriEvent<{ cycle: number; error: string }>(
  'copilot-error',
  useCallback((event) => {
    setCopilotError(`Cycle ${event.cycle}: ${event.error}`);
  }, [])
);
```

**Props passed to components**:

```typescript
<RightPanel
  // ... existing props ...
  copilotEnabled={copilotEnabled}
  copilotStatus={copilotStatus}
  copilotState={copilotState}
  copilotError={copilotError}
  onDismissCopilotQuestion={handleDismissCopilotQuestion}
/>
```

## Algorithms

### Cycle Loop Algorithm

```rust
async fn run_cycle_loop(
    recording_filepath: PathBuf,
    settings: CopilotSettings,
    state: Arc<TokioMutex<CoPilotState>>,
    provider: Arc<dyn IntelProvider>,
    app_handle: AppHandle,
    mut stop_rx: watch::Receiver<bool>,
) {
    let mut cycle_number = 0;
    let mut consecutive_failures = 0;
    let mut inference_times = Vec::new();
    
    // Create log file if logging enabled
    let log_path = if settings.agent_logging {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let log_dir = dirs::home_dir()
            .unwrap()
            .join("Library/Application Support/com.jarvis.app/agent_logs");
        
        tokio::fs::create_dir_all(&log_dir).await.ok();
        
        let log_path = log_dir.join(format!("{}_copilot.md", timestamp));
        
        // Write header
        let recording_filename = recording_filepath.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        write_log_header(&log_path, recording_filename, &settings, "Qwen 2.5 Omni 3B").await.ok();
        
        Some(log_path)
    } else {
        None
    };
    
    loop {
        // Check for stop signal
        if *stop_rx.borrow() {
            break;
        }
        
        cycle_number += 1;
        
        // Mark as processing
        {
            let mut state_guard = state.lock().await;
            state_guard.cycle_metadata.processing = true;
        }
        
        // Emit status event
        app_handle.emit("copilot-status", json!({
            "status": "processing"
        })).ok();
        
        let cycle_start = std::time::Instant::now();
        
        // Run cycle
        match run_single_cycle(
            &recording_filepath,
            &settings,
            &state,
            &provider,
            cycle_number,
        ).await {
            Ok(result) => {
                let inference_time = cycle_start.elapsed().as_secs_f64();
                inference_times.push(inference_time);
                
                // Update state
                update_state(&state, result).await;
                
                // Emit updated event
                let current_state = state.lock().await.clone();
                app_handle.emit("copilot-updated", current_state).ok();
                
                // Log cycle
                if let Some(ref log_path) = log_path {
                    // Log cycle details
                    log_cycle(log_path, cycle_number, /* ... */).await.ok();
                }
                
                // Reset failure counter
                consecutive_failures = 0;
            }
            Err(e) => {
                eprintln!("Co-Pilot cycle {} failed: {}", cycle_number, e);
                
                consecutive_failures += 1;
                
                // Update failed cycles counter
                {
                    let mut state_guard = state.lock().await;
                    state_guard.cycle_metadata.failed_cycles += 1;
                    state_guard.cycle_metadata.processing = false;
                }
                
                // Emit error event
                app_handle.emit("copilot-error", json!({
                    "cycle": cycle_number,
                    "error": e
                })).ok();
                
                // Check failure threshold
                if consecutive_failures >= 3 {
                    eprintln!("Co-Pilot: 3 consecutive failures, pausing agent");
                    app_handle.emit("copilot-status", json!({
                        "status": "paused",
                        "message": "3 consecutive failures"
                    })).ok();
                    break;
                }
            }
        }
        
        // Sleep until next cycle (interval measured from cycle end)
        tokio::time::sleep(tokio::time::Duration::from_secs(settings.cycle_interval)).await;
    }
    
    // Write log summary
    if let Some(ref log_path) = log_path {
        let (failed_cycles, total_duration) = {
            let state_guard = state.lock().await;
            let secs = state_guard.cycle_metadata.total_audio_seconds;
            (
                state_guard.cycle_metadata.failed_cycles,
                format!("{}m {}s", secs / 60, secs % 60)
            )
        };
        
        let successful = cycle_number - failed_cycles;
        let avg_time = if !inference_times.is_empty() {
            inference_times.iter().sum::<f64>() / inference_times.len() as f64
        } else {
            0.0
        };
        
        write_log_summary(
            log_path,
            cycle_number,
            successful,
            0, // skipped cycles
            failed_cycles,
            avg_time,
            &total_duration,
        ).await.ok();
    }
}

async fn run_single_cycle(
    recording_filepath: &Path,
    settings: &CopilotSettings,
    state: &Arc<TokioMutex<CoPilotState>>,
    provider: &Arc<dyn IntelProvider>,
    cycle_number: u32,
) -> Result<CycleResult, String> {
    // 1. Extract audio chunk
    let temp_wav_path = extract_audio_chunk(
        recording_filepath,
        settings.cycle_interval,
        settings.audio_overlap,
    ).await?;

    // 2. Get running context
    let context = {
        let state_guard = state.lock().await;
        state_guard.running_summary.clone()
    };

    // 3. Call provider.copilot_analyze() with timeout
    //    The provider manages its own concurrency internally
    //    (e.g., MlxProvider uses a mutex for sidecar access)
    let response_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(120),
        provider.copilot_analyze(&temp_wav_path, &context)
    ).await;

    // Clean up temp file after inference (success or failure)
    tokio::fs::remove_file(&temp_wav_path).await.ok();

    let cycle_result = response_result
        .map_err(|_| "Inference timeout (120s)".to_string())?
        .map_err(|e| format!("Provider error: {}", e))?;

    // 4. Convert CoPilotCycleResult to internal CycleResult
    let result = CycleResult {
        updated_summary: cycle_result.updated_summary,
        new_content: cycle_result.new_content,
        key_points: cycle_result.key_points,
        decisions: cycle_result.decisions,
        action_items: cycle_result.action_items,
        open_questions: cycle_result.open_questions,
        suggested_questions: cycle_result.suggested_questions,
        key_concepts: cycle_result.key_concepts,
        audio_duration_seconds: settings.cycle_interval + settings.audio_overlap,
    };

    Ok(result)
}
```


## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

After reviewing all testable acceptance criteria, I've identified the following properties. Note that many UI-related requirements (Requirements 7, 8, 9) are not testable via property-based testing as they involve rendering and user interaction.

### Property 1: Audio Chunk Byte Calculation

*For any* cycle interval and audio overlap values within valid ranges (30-120s and 0-15s respectively), the calculated byte size for audio chunk extraction SHALL equal `(cycle_interval + audio_overlap) × 16000 × 2`.

**Validates: Requirements 1.2**

### Property 2: Unique Temporary File Names

*For any* sequence of audio chunk extractions, each temporary WAV file SHALL have a unique filename (using UUID v4), preventing file collisions.

**Validates: Requirements 1.4**

### Property 3: Temporary File Cleanup

*For any* completed cycle (successful or failed), the temporary WAV file created for that cycle SHALL be deleted from the filesystem.

**Validates: Requirements 1.5**

### Property 4: NDJSON Protocol Round Trip

*For any* valid call to `IntelProvider::copilot_analyze()`, the returned `CoPilotCycleResult` SHALL contain all required fields. For MlxProvider specifically, the underlying NDJSON response SHALL be valid and parseable.

**Validates: Requirements 2.5**

### Property 5: Graceful JSON Parsing

*For any* provider response containing partial or malformed data, the `copilot_analyze` implementation SHALL extract whatever fields are successfully parseable and provide defaults for missing fields, without crashing.

**Validates: Requirements 2.7**

### Property 6: Start Requires Active Recording

*For any* call to `start_copilot` when no recording is active, the command SHALL return an error and SHALL NOT create a CoPilotAgent instance.

**Validates: Requirements 3.4**

### Property 7: Concurrent Instance Prevention

*For any* call to `start_copilot` when a Co-Pilot agent is already running, the command SHALL return an error without creating a second instance.

**Validates: Requirements 3.8**

### Property 8: Recording Stop Triggers Agent Stop

*For any* active Co-Pilot agent, when the recording stops, the agent SHALL automatically stop its cycle loop and clean up resources.

**Validates: Requirements 3.6**

### Property 9: Context Propagation Between Cycles

*For any* cycle N > 1, the running context passed to `provider.copilot_analyze()` SHALL be the `updated_summary` field returned by cycle N-1.

**Validates: Requirements 4.3**

### Property 10: Failure Skips Cycle

*For any* cycle where inference fails or times out, the system SHALL increment the `failed_cycles` counter, keep the existing running context unchanged, and proceed to the next cycle without stopping the agent.

**Validates: Requirements 4.5**

### Property 11: State Update Deduplication

*For any* new cycle result containing key_points, decisions, action_items, or open_questions, when merging with existing state, duplicate items (case-sensitive string match) SHALL NOT be added to the arrays.

**Validates: Requirements 5.5**

### Property 12: State JSON Serialization Round Trip

*For any* CoPilotState instance, serializing to JSON and deserializing back SHALL produce an equivalent state object.

**Validates: Requirements 5.6**

### Property 13: State Reset on New Recording

*For any* new recording start, if a previous Co-Pilot state exists, the state SHALL be reset to default (empty) values.

**Validates: Requirements 5.7**

### Property 14: Event Emission on State Change

*For any* successful cycle that produces new data (non-empty response), a `copilot-updated` event SHALL be emitted with the full current state.

**Validates: Requirements 6.1, 6.4**

### Property 15: Status Event on State Transition

*For any* agent status change (starting, active, processing, paused, stopped, error), a `copilot-status` event SHALL be emitted with the new status.

**Validates: Requirements 6.2**

### Property 16: Error Event on Cycle Failure

*For any* cycle that fails (inference error, timeout, MLX busy), a `copilot-error` event SHALL be emitted with the cycle number and error message.

**Validates: Requirements 6.3**

### Property 17: Gem Co-Pilot Data Persistence

*For any* recording with active Co-Pilot data that is saved as a gem, the gem's `source_meta.copilot` field SHALL contain the final agent state with all required fields (summary, key_points, decisions, action_items, open_questions, key_concepts, total_cycles, total_audio_analyzed_seconds).

**Validates: Requirements 10.1, 10.2**

### Property 18: Provider Concurrency Serialization

*For any* two concurrent operations attempting to use the IntelProvider (Co-Pilot cycle and gem enrichment), only one SHALL execute at a time, with the second waiting or timing out. The provider implementation manages its own concurrency (e.g., MlxProvider's internal mutex).

**Validates: Requirements 11.2**

### Property 19: Settings Backward Compatibility

*For any* existing `settings.json` file that does not contain a `copilot` key, loading the settings SHALL succeed and SHALL use the default CopilotSettings values without error.

**Validates: Requirements 12.2**

### Property 20: Agent Logging Conditional Creation

*For any* agent start with `copilot.agent_logging` enabled, a markdown log file SHALL be created in the agent_logs directory; when disabled, no log file SHALL be created.

**Validates: Requirements 13.1, 13.6**

### Property 21: Log File Append-Only

*For any* cycle completion, if logging is enabled, the cycle entry SHALL be appended to the existing log file without modifying previous entries.

**Validates: Requirements 13.3**

### Property 22: Log Excludes Binary Data

*For any* agent log file, the content SHALL NOT contain raw audio binary data, only file path references to temporary WAV files.

**Validates: Requirements 13.7**

### Property 23: Log Directory Auto-Creation

*For any* first use of agent logging, if the `agent_logs/` directory does not exist, it SHALL be created automatically.

**Validates: Requirements 13.8**


## Error Handling

### Audio Extraction Errors

1. **File Not Found**: Recording file doesn't exist → Return error, skip cycle
2. **File Too Short**: Recording shorter than chunk size → Extract all available audio
3. **Read Error**: I/O error reading PCM file → Return error, skip cycle
4. **WAV Conversion Error**: PCM to WAV conversion fails → Return error, skip cycle

### Provider Errors (IntelProvider::copilot_analyze)

1. **Provider Busy**: Provider can't process request (e.g., MlxProvider mutex held) → Skip cycle, log warning
2. **Inference Timeout**: `copilot_analyze` doesn't complete within 120s → Skip cycle, increment failed_cycles
3. **Not Supported**: Provider returns "not supported" error → Return error on start, don't create agent
4. **Model Not Loaded**: Underlying model not available → Return error, pause agent
5. **Audio Not Supported**: Model doesn't support audio → Return error on start, don't create agent
6. **Malformed JSON**: Response is partial/invalid JSON → Parse what's available, use defaults for missing fields
7. **Provider Crash**: Underlying process terminates unexpectedly → Return error, pause agent

### Agent Lifecycle Errors

1. **No Recording Active**: start_copilot called without recording → Return error immediately
2. **Already Running**: start_copilot called when agent exists → Return error immediately
3. **Not Running**: stop_copilot or get_state called when no agent → Return error immediately
4. **Three Consecutive Failures**: Cycles fail 3 times in a row → Emit error event, pause agent

### Settings Errors

1. **Invalid Cycle Interval**: Value outside 30-120s range → Validation error, settings not persisted
2. **Invalid Audio Overlap**: Value outside 0-15s range → Validation error, settings not persisted
3. **Overlap >= Interval**: Overlap not less than interval → Validation error, settings not persisted

### Logging Errors

1. **Directory Creation Failed**: Can't create agent_logs directory → Log warning, disable logging for this session
2. **File Write Failed**: Can't write to log file → Log warning, continue without logging
3. **Disk Full**: No space for log file → Log warning, disable logging for this session

### Recovery Strategies

**Cycle Failures**:
- Skip the failed cycle
- Keep existing state unchanged
- Continue to next cycle
- If 3 consecutive failures → pause agent, require user restart

**Provider Busy**:
- Agent wraps `provider.copilot_analyze()` call in 120s timeout
- If timeout → skip cycle (acceptable, next cycle will catch up)
- Don't block recording or transcription

**Temporary File Cleanup**:
- Clean up temp file after inference completes (success or failure)
- Clean up temp file before early returns (e.g., provider timeout)
- If cleanup fails → log warning, continue (OS will clean temp dir eventually)

## Testing Strategy

### Unit Tests

Unit tests focus on specific examples, edge cases, and error conditions:

- **Audio Chunk Extraction**:
  - Test byte calculation with various intervals and overlaps
  - Test handling of short recording files (first cycle)
  - Test WAV header generation
  - Test unique filename generation

- **State Aggregation**:
  - Test deduplication of key_points, decisions, action_items
  - Test concept merging with mention_count increment
  - Test suggested questions replacement (max 5, preserve dismissed)
  - Test state reset on new recording

- **Settings Validation**:
  - Test cycle_interval range validation (30-120)
  - Test audio_overlap range validation (0-15)
  - Test overlap < interval validation
  - Test backward compatibility (missing copilot key)

- **Agent Logging**:
  - Test log file creation when enabled
  - Test no log file when disabled
  - Test log header format
  - Test cycle entry format
  - Test summary format
  - Test directory auto-creation

- **Error Handling**:
  - Test start_copilot without active recording
  - Test concurrent start_copilot calls
  - Test graceful JSON parsing with malformed input
  - Test cycle failure handling
  - Test three consecutive failures threshold

### Property-Based Tests

Property tests verify universal properties across all inputs (minimum 100 iterations per test):

- **Property 1: Audio Chunk Byte Calculation**
  - Generate random cycle intervals (30-120) and overlaps (0-15)
  - Verify byte calculation formula
  - Tag: **Feature: copilot-agent, Property 1: Audio Chunk Byte Calculation**

- **Property 2: Unique Temporary File Names**
  - Generate multiple chunk extractions
  - Verify all filenames are unique (UUID collision check)
  - Tag: **Feature: copilot-agent, Property 2: Unique Temporary File Names**

- **Property 3: Temporary File Cleanup**
  - Run cycles and verify temp files are deleted
  - Tag: **Feature: copilot-agent, Property 3: Temporary File Cleanup**

- **Property 4: NDJSON Protocol Round Trip**
  - Generate random copilot-analyze commands
  - Verify responses are valid NDJSON
  - Tag: **Feature: copilot-agent, Property 4: NDJSON Protocol Round Trip**

- **Property 5: Graceful JSON Parsing**
  - Generate partial/malformed JSON responses
  - Verify parser extracts available fields without crashing
  - Tag: **Feature: copilot-agent, Property 5: Graceful JSON Parsing**

- **Property 9: Context Propagation Between Cycles**
  - Generate random cycle sequences
  - Verify context flows correctly between cycles
  - Tag: **Feature: copilot-agent, Property 9: Context Propagation Between Cycles**

- **Property 11: State Update Deduplication**
  - Generate random cycle results with duplicates
  - Verify deduplication works correctly
  - Tag: **Feature: copilot-agent, Property 11: State Update Deduplication**

- **Property 12: State JSON Serialization Round Trip**
  - Generate random CoPilotState instances
  - Verify JSON round-trip preserves data
  - Tag: **Feature: copilot-agent, Property 12: State JSON Serialization Round Trip**

- **Property 19: Settings Backward Compatibility**
  - Generate settings files without copilot key
  - Verify loading succeeds with defaults
  - Tag: **Feature: copilot-agent, Property 19: Settings Backward Compatibility**

### Integration Tests

- Test full cycle loop from start to stop
- Test agent interaction with recording lifecycle
- Test IntelProvider::copilot_analyze() end-to-end (MlxProvider with sidecar)
- Test gem integration (save recording with copilot data)
- Test concurrent MLX access (copilot + enrichment)
- Test agent logging full workflow

### Manual Testing

- Test UI toggle responsiveness
- Test Co-Pilot tab switching and notification badge
- Test suggested questions display and dismiss
- Test key concepts display
- Test agent log readability
- Test settings UI for copilot configuration
- Test gem detail panel copilot section display


## Frontend Components

### 1. CoPilotPanel Component

**Location**: `jarvis-app/src/components/CoPilotPanel.tsx`

**Structure**:

```typescript
interface CoPilotPanelProps {
  state: CoPilotState | null;
  status: CoPilotStatus;
  onDismissQuestion: (index: number) => void;
}

export function CoPilotPanel({ state, status, onDismissQuestion }: CoPilotPanelProps) {
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  
  const handleQuestionClick = async (question: string, index: number) => {
    await navigator.clipboard.writeText(question);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
  };
  
  if (!state) {
    return (
      <div className="copilot-placeholder">
        <p>Co-Pilot is listening... first analysis in ~60 seconds</p>
      </div>
    );
  }
  
  return (
    <div className="copilot-panel">
      {/* Summary Section */}
      <section className="copilot-section summary-section">
        <h3>Summary</h3>
        <p className="summary-text">{state.running_summary}</p>
        
        {state.key_points.length > 0 && (
          <div className="key-points">
            <h4>Key Points</h4>
            <ul>
              {state.key_points.map((point, i) => (
                <li key={i}>{point}</li>
              ))}
            </ul>
          </div>
        )}
        
        {state.open_questions.length > 0 && (
          <div className="open-questions">
            <h4>Open Questions</h4>
            <ul className="warning-list">
              {state.open_questions.map((question, i) => (
                <li key={i}>⚠️ {question}</li>
              ))}
            </ul>
          </div>
        )}
      </section>
      
      {/* Decisions & Action Items Section */}
      {(state.decisions.length > 0 || state.action_items.length > 0) && (
        <section className="copilot-section decisions-section">
          <h3>Decisions & Action Items</h3>
          
          {state.decisions.length > 0 && (
            <div className="decisions">
              <h4>Decisions</h4>
              <ul className="checklist">
                {state.decisions.map((decision, i) => (
                  <li key={i}>✓ {decision}</li>
                ))}
              </ul>
            </div>
          )}
          
          {state.action_items.length > 0 && (
            <div className="action-items">
              <h4>Action Items</h4>
              <ul>
                {state.action_items.map((item, i) => (
                  <li key={i}>{item}</li>
                ))}
              </ul>
            </div>
          )}
        </section>
      )}
      
      {/* Suggested Questions Section */}
      {state.suggested_questions.filter(q => !q.dismissed).length > 0 && (
        <section className="copilot-section questions-section">
          <h3>Suggested Questions</h3>
          <div className="questions-grid">
            {state.suggested_questions
              .filter(q => !q.dismissed)
              .map((q, i) => (
                <div key={i} className="question-card">
                  <div 
                    className="question-text"
                    onClick={() => handleQuestionClick(q.question, i)}
                  >
                    {q.question}
                    {copiedIndex === i && <span className="copied-indicator">Copied!</span>}
                  </div>
                  <div className="question-reason">{q.reason}</div>
                  <button
                    className="dismiss-button"
                    onClick={() => onDismissQuestion(i)}
                    aria-label="Dismiss question"
                  >
                    ×
                  </button>
                </div>
              ))}
          </div>
        </section>
      )}
      
      {/* Key Concepts Section */}
      {state.key_concepts.length > 0 && (
        <section className="copilot-section concepts-section">
          <h3>Key Concepts</h3>
          <div className="concepts-grid">
            {state.key_concepts.map((concept, i) => (
              <div 
                key={i} 
                className="concept-chip"
                title={concept.context}
              >
                <span className="concept-term">{concept.term}</span>
                {concept.mention_count > 1 && (
                  <span className="mention-count">{concept.mention_count}</span>
                )}
              </div>
            ))}
          </div>
        </section>
      )}
      
      {/* Status Footer */}
      <footer className="copilot-footer">
        <div className="cycle-info">
          Cycle {state.cycle_metadata.cycle_number}
          {state.cycle_metadata.last_updated_at && (
            <span className="last-update">
              {' • '}
              {formatTimeAgo(state.cycle_metadata.last_updated_at)}
            </span>
          )}
        </div>
        <div className={`status-indicator ${status}`}>
          {status === 'processing' && <span className="pulse-dot" />}
          {status}
        </div>
      </footer>
    </div>
  );
}

function formatTimeAgo(isoTimestamp: string): string {
  const now = new Date();
  const then = new Date(isoTimestamp);
  const seconds = Math.floor((now.getTime() - then.getTime()) / 1000);
  
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}
```

### 2. Right Panel Tab Integration

**Location**: `jarvis-app/src/components/RightPanel.tsx` (modifications to existing `activeNav === 'record'` branch)

The existing RightPanel.tsx is a props-driven component that renders different content based on `activeNav`. It has 6 branches: record, recordings, gems, youtube, browser, settings. The Co-Pilot tab integration should be added inside the existing `if (activeNav === 'record')` branch, not as a top-level rewrite.

**Props interface additions**:

```typescript
interface RightPanelProps {
  // ... existing props ...
  copilotEnabled: boolean;
  copilotStatus: CoPilotStatus;
  copilotState: CoPilotState | null;
  copilotError: string | null;
  onDismissCopilotQuestion: (index: number) => void;
}
```

**Modification to the existing `activeNav === 'record'` branch**:

The current code in RightPanel.tsx has this structure:

```typescript
if (activeNav === 'record') {
  const isRecording = recordingState === 'recording';
  const hasTranscript = transcript.length > 0;
  const recordingCompleted = !isRecording && hasTranscript;

  if (isRecording || recordingCompleted) {
    return (
      <div className="right-panel" style={style}>
        <TranscriptDisplay ... />
      </div>
    );
  }

  return (
    <div className="right-panel" style={style}>
      <div className="right-panel-placeholder">
        Start recording to see live transcript
      </div>
    </div>
  );
}
```

Replace this entire `if (activeNav === 'record')` block with:

```typescript
// Record nav: show live transcript OR Co-Pilot tabs when recording
if (activeNav === 'record') {
  const isRecording = recordingState === 'recording';
  const hasTranscript = transcript.length > 0;
  const recordingCompleted = !isRecording && hasTranscript;
  
  // Show tabs when Co-Pilot is enabled during recording
  if (copilotEnabled && isRecording) {
    return (
      <div className="right-panel" style={style}>
        <RecordTabsView
          transcript={transcript}
          transcriptionStatus={transcriptionStatus}
          transcriptionError={transcriptionError}
          currentRecording={currentRecording}
          copilotState={copilotState}
          copilotStatus={copilotStatus}
          onDismissCopilotQuestion={onDismissCopilotQuestion}
        />
      </div>
    );
  }
  
  // Show transcript only (existing behavior)
  if (isRecording || recordingCompleted) {
    return (
      <div className="right-panel" style={style}>
        <TranscriptDisplay
          transcript={transcript}
          status={transcriptionStatus}
          error={transcriptionError}
          recordingFilename={currentRecording}
        />
      </div>
    );
  }

  return (
    <div className="right-panel" style={style}>
      <div className="right-panel-placeholder">
        Start recording to see live transcript
      </div>
    </div>
  );
}
```

**New component: RecordTabsView** (add to RightPanel.tsx file, not a separate file):

```typescript
interface RecordTabsViewProps {
  transcript: TranscriptionSegment[];
  transcriptionStatus: 'idle' | 'active' | 'error' | 'disabled';
  transcriptionError: string | null;
  currentRecording: string | null;
  copilotState: CoPilotState | null;
  copilotStatus: CoPilotStatus;
  onDismissCopilotQuestion: (index: number) => void;
}

function RecordTabsView({
  transcript,
  transcriptionStatus,
  transcriptionError,
  currentRecording,
  copilotState,
  copilotStatus,
  onDismissCopilotQuestion,
}: RecordTabsViewProps) {
  const [activeTab, setActiveTab] = useState<'transcript' | 'copilot'>('transcript');
  const [hasNewCopilotData, setHasNewCopilotData] = useState(false);
  
  // Show notification badge when copilot data arrives while on transcript tab
  useEffect(() => {
    if (copilotState && activeTab === 'transcript') {
      setHasNewCopilotData(true);
    }
  }, [copilotState, activeTab]);
  
  // Clear notification badge when switching to copilot tab
  useEffect(() => {
    if (activeTab === 'copilot') {
      setHasNewCopilotData(false);
    }
  }, [activeTab]);
  
  return (
    <>
      <div className="tab-buttons">
        <button
          className={`tab-button ${activeTab === 'transcript' ? 'active' : ''}`}
          onClick={() => setActiveTab('transcript')}
        >
          Transcript
        </button>
        <button
          className={`tab-button ${activeTab === 'copilot' ? 'active' : ''}`}
          onClick={() => setActiveTab('copilot')}
        >
          Co-Pilot
          {hasNewCopilotData && <span className="notification-dot" />}
        </button>
      </div>
      
      <div className="tab-content">
        {activeTab === 'transcript' && (
          <TranscriptDisplay
            transcript={transcript}
            status={transcriptionStatus}
            error={transcriptionError}
            recordingFilename={currentRecording}
          />
        )}
        {activeTab === 'copilot' && (
          <CoPilotPanel
            state={copilotState}
            status={copilotStatus}
            onDismissQuestion={onDismissCopilotQuestion}
          />
        )}
      </div>
    </>
  );
}
```

### 3. Co-Pilot Toggle

**Location**: `jarvis-app/src/App.tsx` (additions to the `activeNav === 'record'` section)

**State additions** (add near other useState declarations):

```typescript
// Co-Pilot state
const [copilotEnabled, setCopilotEnabled] = useState(false);
const [copilotStatus, setCopilotStatus] = useState<CoPilotStatus>('stopped');
const [copilotState, setCopilotState] = useState<CoPilotState | null>(null);
const [copilotError, setCopilotError] = useState<string | null>(null);
```

**Event listeners** (add with other useTauriEvent calls):

```typescript
// Listen for Co-Pilot events
useTauriEvent<CoPilotState>(
  'copilot-updated',
  useCallback((state) => {
    setCopilotState(state);
    setCopilotStatus('active');
  }, [])
);

useTauriEvent<{ status: CoPilotStatus; message?: string }>(
  'copilot-status',
  useCallback((event) => {
    setCopilotStatus(event.status);
  }, [])
);

useTauriEvent<{ cycle: number; error: string }>(
  'copilot-error',
  useCallback((event) => {
    setCopilotError(`Cycle ${event.cycle}: ${event.error}`);
  }, [])
);
```

**Handler functions** (add with other handler functions):

```typescript
const handleCopilotToggle = async () => {
  if (!copilotEnabled) {
    // Start Co-Pilot
    try {
      await invoke('start_copilot');
      setCopilotEnabled(true);
      setCopilotStatus('starting');
    } catch (error) {
      console.error('Failed to start Co-Pilot:', error);
      setToastError(`Failed to start Co-Pilot: ${error}`);
    }
  } else {
    // Stop Co-Pilot
    try {
      const finalState = await invoke<CoPilotState>('stop_copilot');
      setCopilotEnabled(false);
      setCopilotStatus('stopped');
      setCopilotState(finalState);
    } catch (error) {
      console.error('Failed to stop Co-Pilot:', error);
      setToastError(`Failed to stop Co-Pilot: ${error}`);
    }
  }
};

const handleDismissCopilotQuestion = async (index: number) => {
  try {
    await invoke('dismiss_copilot_question', { index });
    // Update local state
    if (copilotState) {
      setCopilotState({
        ...copilotState,
        suggested_questions: copilotState.suggested_questions.map((q, i) =>
          i === index ? { ...q, dismissed: true } : q
        ),
      });
    }
  } catch (error) {
    console.error('Failed to dismiss question:', error);
  }
};

// Reset Co-Pilot when recording stops
useEffect(() => {
  if (state.recordingState !== 'recording' && copilotEnabled) {
    setCopilotEnabled(false);
    setCopilotStatus('stopped');
  }
}, [state.recordingState, copilotEnabled]);
```

**UI modification** (add to the `activeNav === 'record'` section, after the button-container div):

```typescript
{activeNav === 'record' && (
  <>
    {/* Status Display */}
    <div className="status">
      {/* ... existing status display ... */}
    </div>

    {/* Record Button */}
    <div className="button-container">
      {/* ... existing record button ... */}
    </div>

    {/* Co-Pilot Toggle (NEW - shown when recording is active) */}
    {state.recordingState === 'recording' && (
      <div className="copilot-toggle">
        <label>
          <input
            type="checkbox"
            checked={copilotEnabled}
            onChange={handleCopilotToggle}
          />
          <span className="toggle-label">Co-Pilot</span>
        </label>
      </div>
    )}

    {/* Error Display */}
    {/* ... existing error display ... */}
  </>
)}
```

**Props to RightPanel** (update the RightPanel component call):

```typescript
<RightPanel
  // ... existing props ...
  copilotEnabled={copilotEnabled}
  copilotStatus={copilotStatus}
  copilotState={copilotState}
  copilotError={copilotError}
  onDismissCopilotQuestion={handleDismissCopilotQuestion}
/>
```

### 4. CSS Styling

**Location**: `jarvis-app/src/App.css` (additions)

```css
/* Co-Pilot Panel */
.copilot-panel {
  padding: var(--space-4);
  overflow-y: auto;
}

.copilot-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.copilot-section {
  margin-bottom: var(--space-6);
  padding-bottom: var(--space-6);
  border-bottom: 1px solid var(--border-subtle);
}

.copilot-section:last-of-type {
  border-bottom: none;
}

.copilot-section h3 {
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  margin-bottom: var(--space-2);
  color: var(--text-primary);
}

.copilot-section h4 {
  font-size: var(--text-base);
  font-weight: var(--font-medium);
  margin-top: var(--space-4);
  margin-bottom: var(--space-1);
  color: var(--text-secondary);
}

.summary-text {
  font-size: var(--text-base);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
}

.key-points ul,
.open-questions ul,
.decisions ul,
.action-items ul {
  list-style: none;
  padding-left: 0;
}

.key-points li,
.open-questions li,
.decisions li,
.action-items li {
  padding: var(--space-1) 0;
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.warning-list li {
  color: var(--warning);
}

.checklist li::before {
  content: '✓ ';
  color: var(--success);
  font-weight: var(--font-semibold);
}

/* Suggested Questions */
.questions-grid {
  display: grid;
  gap: var(--space-2);
}

.question-card {
  position: relative;
  padding: var(--space-3);
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-default);
  cursor: pointer;
  transition: all var(--duration-normal) var(--ease-out);
}

.question-card:hover {
  border-color: var(--accent-primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.question-text {
  font-size: var(--text-base);
  font-weight: var(--font-medium);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
  position: relative;
}

.copied-indicator {
  position: absolute;
  right: 0;
  top: 0;
  font-size: var(--text-sm);
  color: var(--success);
  font-weight: var(--font-normal);
}

.question-reason {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.dismiss-button {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--duration-normal) var(--ease-out);
}

.dismiss-button:hover {
  background: var(--bg-elevated);
  color: var(--text-primary);
}

/* Key Concepts */
.concepts-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.concept-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  background: var(--accent-subtle);
  color: var(--accent-primary);
  border: 1px solid var(--accent-border);
  border-radius: var(--radius-lg);
  font-size: var(--text-sm);
  cursor: help;
}

.mention-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 var(--space-1);
  background: var(--accent-primary);
  color: white;
  border-radius: var(--radius-lg);
  font-size: var(--text-sm);
  font-weight: var(--font-semibold);
}

/* Status Footer */
.copilot-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: var(--space-4);
  margin-top: var(--space-4);
  border-top: 1px solid var(--border-subtle);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-weight: var(--font-medium);
}

.status-indicator.processing {
  color: var(--accent-primary);
}

.pulse-dot {
  width: 8px;
  height: 8px;
  background: var(--accent-primary);
  border-radius: 50%;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

/* Tab Buttons */
.tab-buttons {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-2);
  border-bottom: 1px solid var(--border-subtle);
}

.tab-button {
  position: relative;
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-normal) var(--ease-out);
}

.tab-button.active {
  color: var(--accent-primary);
  border-bottom-color: var(--accent-primary);
}

.tab-button:hover:not(.active) {
  color: var(--text-primary);
}

.notification-dot {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 8px;
  height: 8px;
  background: var(--accent-primary);
  border-radius: 50%;
}

/* Co-Pilot Toggle */
.copilot-toggle {
  margin-top: var(--space-4);
}

.copilot-toggle label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
}

.copilot-toggle input[type="checkbox"] {
  width: 40px;
  height: 24px;
  appearance: none;
  background: var(--bg-elevated);
  border-radius: 12px;
  position: relative;
  cursor: pointer;
  transition: background var(--duration-normal) var(--ease-out);
}

.copilot-toggle input[type="checkbox"]:checked {
  background: var(--accent-primary);
}

.copilot-toggle input[type="checkbox"]::before {
  content: '';
  position: absolute;
  width: 20px;
  height: 20px;
  background: var(--text-primary);
  border-radius: 50%;
  top: 2px;
  left: 2px;
  transition: transform var(--duration-normal) var(--ease-out);
}

.copilot-toggle input[type="checkbox"]:checked::before {
  transform: translateX(16px);
}

.copilot-toggle input[type="checkbox"]:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle-label {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
```

## Technical Constraints

1. **Provider-agnostic inference**: The Co-Pilot agent calls `IntelProvider::copilot_analyze()` — the same trait used for tags, summary, and transcription. For V1, only `MlxProvider` implements this (local MLX sidecar). Future API providers implement the same method — single point of change.
2. **Qwen Omni required (V1)**: The Co-Pilot requires a multimodal model with audio understanding. The provider checks model capabilities before starting and returns an error if the active model does not support audio.
3. **Single CSS file**: All new CSS goes in `App.css` using existing design tokens. No new CSS files.
4. **Existing sidecar protocol**: The `copilot-analyze` command follows the existing NDJSON protocol. No protocol changes.
5. **PCM format**: Audio is 16kHz, 16-bit signed, mono. WAV conversion uses the same header logic as existing `convert_to_wav`.
6. **Rust module structure**: New agent code goes in `src-tauri/src/agents/` as a new module. New Tauri commands registered in `lib.rs`.
7. **No recording changes**: The recording and transcription pipeline are NOT modified. The Co-Pilot is additive only.
8. **Settings backward compatibility**: Existing `settings.json` without `copilot` key must deserialize correctly via `#[serde(default)]`.
9. **Temporary files**: Audio chunks written to system temp dir must be cleaned up after each cycle.
10. **Agent log size**: Each log file is ~50–200KB depending on meeting length. No automatic cleanup — user manages disk space.
11. **Memory**: Qwen 2.5 Omni 3B (8-bit) needs ~5.3 GB. The agent and gem enrichment share the same loaded model instance.
12. **Concurrency**: Real-time transcription uses whisper-rs/WhisperKit (separate from IntelProvider). Co-Pilot and enrichment share the `IntelProvider` instance. Provider implementations handle their own concurrency (e.g., MlxProvider's internal mutex).

## Out of Scope

1. Speaker diarization — the agent does not track who said what
2. Multiple concurrent agents — only one Co-Pilot instance at a time
3. Custom prompt configuration — the analysis prompt is hardcoded for V1
4. Agent memory across recordings — each recording starts with fresh state
5. Offline concept lookup — "Key Concepts" are flagged but not auto-defined
6. Adaptive cycle intervals based on speech density — deferred to future polish phase
7. Agent log browsing UI — logs exist as markdown files but no in-app viewer
8. Cloud API provider implementation — architecture supports it via `IntelProvider` trait, but V1 only implements `MlxProvider`
9. Co-Pilot for existing recordings — only works during live recording
10. Export Co-Pilot data separately from gems — only available via gem source_meta

