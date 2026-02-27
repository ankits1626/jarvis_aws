# Design: MLX Omni Transcription (Local, Private)

## Overview

This feature adds local multimodal audio transcription to Jarvis using MLX models (Qwen2.5-Omni). It extends the existing MLX sidecar (`mlx-server/server.py`) with a new `generate-transcript` command that processes audio files and returns accurate multilingual transcripts. Transcripts are stored as new fields on the Gem data model alongside existing AI-generated tags and summaries.

The design reuses the existing MLX infrastructure:
- Same `MlxProvider` and sidecar process (no new process spawned)
- Same `IntelProvider` trait pattern with new `generate_transcript()` method
- Same venv and model management infrastructure
- Integrates with existing gem enrichment flow

Key capabilities:
- Transcribes audio in the original spoken language (no translation)
- Supports multilingual transcription (Hindi, English, etc.) with native script output
- Processes full audio files post-recording for high accuracy
- Whisper continues to provide real-time partials during recording (unchanged)
- Fully local and private (no data leaves the machine)

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Jarvis App (Tauri)                      │
│                                                             │
│  ┌──────────────┐      ┌─────────────────┐                │
│  │ Recording    │      │ Gem Enrichment  │                │
│  │ Pipeline     │──────▶│ Flow            │                │
│  └──────────────┘      └────────┬────────┘                │
│                                  │                          │
│                                  ▼                          │
│                         ┌────────────────┐                 │
│                         │ IntelProvider  │                 │
│                         │ (trait)        │                 │
│                         └────────┬───────┘                 │
│                                  │                          │
│                                  ▼                          │
│                         ┌────────────────┐                 │
│                         │  MlxProvider   │                 │
│                         └────────┬───────┘                 │
│                                  │ NDJSON                   │
│                                  │ stdin/stdout             │
└──────────────────────────────────┼──────────────────────────┘
                                   │
                                   ▼
                          ┌─────────────────┐
                          │  mlx-server     │
                          │  (Python)       │
                          │                 │
                          │  Commands:      │
                          │  - check-avail  │
                          │  - load-model   │
                          │  - gen-tags     │
                          │  - summarize    │
                          │  - gen-trans ◄──┼── NEW
                          └─────────────────┘
```


### Component Interaction Flow

1. **Recording Completion**: User stops recording → audio saved to `~/.jarvis/recordings/YYYYMMDD_HHMMSS.pcm`
2. **Gem Creation**: Recording metadata converted to Gem → saved to SQLite
3. **Enrichment Trigger**: `enrich_gem()` called with gem ID
4. **Sequential Enrichment**: Three operations run sequentially (tags, summary, transcript)
5. **Transcript Generation**: 
   - `MlxProvider.generate_transcript(audio_path)` called
   - Sends `{"command":"generate-transcript","audio_path":"..."}` to sidecar
   - Sidecar loads audio, processes through multimodal model
   - Returns `{"type":"response","command":"generate-transcript","language":"Hindi","transcript":"..."}`
6. **Storage**: Transcript and language stored in gem's `transcript` and `transcript_language` fields
7. **Frontend Update**: Gem serialized with transcript fields → displayed in UI

### Data Flow

```
Recording File (.pcm)
    │
    ├──▶ Whisper (real-time) ──▶ content field (Whisper transcript)
    │
    └──▶ MLX Omni (post-recording) ──▶ transcript field (accurate multilingual)
```

Both transcripts coexist:
- `content`: Whisper's real-time transcript (may have errors, English-biased)
- `transcript`: MLX Omni's accurate multilingual transcript (post-processing)

## Components and Interfaces

### 1. IntelProvider Trait Extension

**File**: `src-tauri/src/intelligence/provider.rs`

Add new method to the trait:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResult {
    pub language: String,
    pub transcript: String,
}

#[async_trait]
pub trait IntelProvider: Send + Sync {
    // ... existing methods ...
    
    /// Generate transcript from audio file
    /// 
    /// Default implementation returns error for providers that don't support transcription.
    async fn generate_transcript(&self, audio_path: &Path) -> Result<TranscriptResult, String> {
        Err("Transcript generation not supported by this provider".to_string())
    }
}
```


### 2. MlxProvider Implementation

**File**: `src-tauri/src/intelligence/mlx_provider.rs`

Add `generate_transcript()` implementation:

```rust
impl MlxProvider {
    /// Generate transcript from audio file
    async fn generate_transcript_internal(&self, audio_path: &Path) -> Result<TranscriptResult, String> {
        let cmd = NdjsonCommand {
            command: "generate-transcript".to_string(),
            audio_path: Some(audio_path.to_string_lossy().to_string()),
            model_path: None,
            content: None,
            repo_id: None,
            destination: None,
        };

        // Use 120s timeout for transcript generation (longer than tags/summary)
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            self.send_command(cmd)
        )
        .await
        .map_err(|_| "Transcript generation timeout (120s)".to_string())??;

        if response.response_type == "error" {
            return Err(response.error.unwrap_or_else(|| "Transcript generation failed".to_string()));
        }

        let language = response.language.ok_or("No language in response")?;
        let transcript = response.transcript.ok_or("No transcript in response")?;

        Ok(TranscriptResult { language, transcript })
    }
}

#[async_trait]
impl IntelProvider for MlxProvider {
    // ... existing methods ...
    
    async fn generate_transcript(&self, audio_path: &Path) -> Result<TranscriptResult, String> {
        self.generate_transcript_internal(audio_path).await
    }
}
```

Update `NdjsonCommand` struct:

```rust
#[derive(Serialize)]
struct NdjsonCommand {
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_path: Option<String>,  // NEW: for generate-transcript
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,  // NEW: for load-model
    // ... existing fields ...
}
```

**Important**: When calling `load-model`, the Rust side must look up the model in the catalog and pass its capabilities:

```rust
async fn load_model_internal(&self, model_path: PathBuf) -> Result<(), String> {
    // Look up model capabilities from catalog
    let model_id = model_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid model path")?;
    
    let capabilities = Self::catalog_entry(model_id)
        .map(|entry| entry.capabilities.iter().map(|s| s.to_string()).collect())
        .unwrap_or_else(|| vec!["text".to_string()]);  // Default to text-only
    
    let cmd = NdjsonCommand {
        command: "load-model".to_string(),
        model_path: Some(model_path.to_string_lossy().to_string()),
        capabilities: Some(capabilities),  // Pass capabilities from catalog
        audio_path: None,
        content: None,
        // ... other fields ...
    };
    
    // Send command and wait for response...
}
```

Update `NdjsonResponse` struct:

```rust
#[derive(Deserialize, Debug)]
struct NdjsonResponse {
    // ... existing fields ...
    #[serde(default)]
    language: Option<String>,  // NEW: for generate-transcript
    #[serde(default)]
    transcript: Option<String>,  // NEW: for generate-transcript
    #[serde(default)]
    capabilities: Option<Vec<String>>,  // NEW: for load-model response
}
```


### 3. MLX Sidecar Extension

**File**: `src-tauri/sidecars/mlx-server/server.py`

#### Runtime Patches

Add patch application at startup (before model loading):

```python
import mlx_lm_omni
from packaging import version

def apply_runtime_patches():
    """Apply runtime patches for mlx-lm-omni v0.1.3 bugs.
    
    These patches fix 5 of the 6 critical bugs in mlx-lm-omni <= 0.1.3 via runtime monkey-patching:
    1. AudioTower reshape bug (causes failure on audio > 15s)
    2. AudioMel precision loss (float16 → float32)
    3. ExtendedQuantizedEmbedding kwargs compatibility
    4. Model attribute delegation to tokenizer
    5. 7B model conv weight layout detection (auto-detect and fix PyTorch layout)
    
    The remaining bug is handled differently:
    6. Prefill chunking bug - Fixed at call-site via prefill_step_size=32768 parameter in generate_transcript()
       (avoids patching generate() internals, cleaner separation of concerns)
    
    Patches are version-gated and automatically disabled for versions > 0.1.3.
    
    Design Note: Bug #6 (prefill chunking) is intentionally NOT patched here because:
    - It requires patching the generate() function's internal logic
    - The fix is cleaner when applied at the call-site via the prefill_step_size parameter
    - This avoids deep monkey-patching of core generation logic
    - The parameter-based approach is more maintainable and explicit
    """
    patches_applied = []
    
    # Only apply patches for versions <= 0.1.3
    if version.parse(mlx_lm_omni.__version__) > version.parse("0.1.3"):
        print("MLX: mlx-lm-omni version > 0.1.3, skipping patches", file=sys.stderr, flush=True)
        return
    
    try:
        # Patch 1: AudioTower.__call__ - move reshape after transformer loop
        from mlx_lm_omni.models.qwen_omni import AudioTower
        original_call = AudioTower.__call__
        
        def patched_call(self, audio_features):
            # Process each audio chunk independently through transformer
            # (implementation details omitted for brevity)
            pass
        
        AudioTower.__call__ = patched_call
        patches_applied.append("AudioTower.__call__")
        
        # Patch 2: AudioMel - use float32 for precision
        from mlx_lm_omni.audio_processing import AudioMel
        # Patch mel_filters, waveform, window to use float32
        patches_applied.append("AudioMel.float32")
        
        # Patch 3: ExtendedQuantizedEmbedding - accept **kwargs
        from mlx_lm_omni.models.qwen_omni import ExtendedQuantizedEmbedding
        original_to_quantized = ExtendedQuantizedEmbedding.to_quantized
        
        def patched_to_quantized(cls, module, **kwargs):
            return original_to_quantized(cls, module)
        
        ExtendedQuantizedEmbedding.to_quantized = classmethod(patched_to_quantized)
        patches_applied.append("ExtendedQuantizedEmbedding.to_quantized")
        
        # Patch 4: Model class - add __getattr__ and chat_template
        from mlx_lm_omni.models.qwen_omni import Model
        
        def __getattr__(self, name):
            return getattr(self.tokenizer, name)
        
        @property
        def chat_template(self):
            return self.tokenizer.chat_template
        
        Model.__getattr__ = __getattr__
        Model.chat_template = chat_template
        patches_applied.append("Model.__getattr__")
        
        # Patch 5: 7B model conv weight layout detection and fix
        from mlx_lm_omni.models.qwen_omni import AudioEncoder
        original_init = AudioEncoder.__init__
        
        def patched_init(self, config):
            original_init(self, config)
            # Auto-detect PyTorch layout mismatch in conv weights
            # PyTorch: (out_channels, in_channels, kernel_size)
            # MLX: (out_channels, kernel_size, in_channels)
            # Check if conv1.weight has wrong layout (shape[1] != 3 for RGB input)
            if hasattr(self, 'conv1') and hasattr(self.conv1, 'weight'):
                if self.conv1.weight.shape[1] != 3:
                    # Wrong layout detected - swap axes 1 and 2
                    self.conv1.weight = mx.swapaxes(self.conv1.weight, 1, 2)
                    if hasattr(self, 'conv2') and hasattr(self.conv2, 'weight'):
                        self.conv2.weight = mx.swapaxes(self.conv2.weight, 1, 2)
                    print("MLX: Applied conv weight layout fix for 7B model", file=sys.stderr, flush=True)
        
        AudioEncoder.__init__ = patched_init
        patches_applied.append("AudioEncoder.conv_layout")
        
        print(f"MLX: Applied patches: {', '.join(patches_applied)}", file=sys.stderr, flush=True)
        
    except Exception as e:
        print(f"MLX: Warning - failed to apply some patches: {e}", file=sys.stderr, flush=True)
```


#### Generate Transcript Command

Add new command handler:

```python
class MLXServer:
    def __init__(self):
        self.model = None
        self.tokenizer = None
        self.model_name = None
        self.capabilities = []  # NEW: track model capabilities from catalog
    
    def load_model(self, model_path: str, capabilities: list = None) -> Dict[str, Any]:
        """Load an MLX model from disk.
        
        Args:
            model_path: Path to the model directory
            capabilities: List of capabilities from catalog (e.g., ["audio", "text"])
                         If not provided, defaults to ["text"] for backward compatibility
        """
        try:
            self.model, self.tokenizer = load(model_path)
            self.model_name = model_path.split("/")[-1]
            
            # Store capabilities from catalog (more robust than introspection)
            # Default to ["text"] if not provided (backward compatibility)
            self.capabilities = capabilities if capabilities is not None else ["text"]
            
            return {
                "type": "response",
                "command": "load-model",
                "success": True,
                "model_name": self.model_name,
                "capabilities": self.capabilities
            }
        except Exception as e:
            return {"type": "error", "command": "load-model", "error": str(e)}
    
    def generate_transcript(self, audio_path: str) -> Dict[str, Any]:
        """Generate transcript from audio file."""
        if self.model is None:
            return {"type": "error", "command": "generate-transcript", "error": "No model loaded"}
        
        # Check capabilities from catalog, not model introspection
        if "audio" not in self.capabilities:
            return {"type": "error", "command": "generate-transcript", 
                    "error": "Loaded model does not support audio transcription"}
        
        try:
            import librosa
            import numpy as np
            
            # Check if file exists
            if not os.path.exists(audio_path):
                return {"type": "error", "command": "generate-transcript", 
                        "error": f"Audio file not found: {audio_path}"}
            
            # Load audio file
            if audio_path.endswith('.wav'):
                audio, sr = librosa.load(audio_path, sr=16000, mono=True)
            elif audio_path.endswith('.pcm'):
                # Read raw PCM (s16le, 16kHz, mono)
                with open(audio_path, 'rb') as f:
                    pcm_data = np.frombuffer(f.read(), dtype=np.int16)
                audio = pcm_data.astype(np.float32) / 32768.0  # Convert to float32 [-1, 1]
            else:
                return {"type": "error", "command": "generate-transcript", 
                        "error": f"Unsupported audio format: {audio_path}"}
            
            # Handle empty audio
            if len(audio) == 0:
                return {"type": "response", "command": "generate-transcript",
                        "language": "unknown", "transcript": ""}
            
            # Clear ExtendedEmbedding queue to prevent state leakage
            # The queue lives on the embedding layer, not attention layers
            if hasattr(self.model, 'language_model') and hasattr(self.model.language_model, 'model'):
                embed = self.model.language_model.model.embed_tokens
                if hasattr(embed, 'extended_embedding_queue'):
                    embed.extended_embedding_queue.clear()
            
            # Generate transcript with language detection prompt
            prompt_text = "First, identify the language spoken in this audio. Then transcribe the audio verbatim in that original language. Do NOT translate."
            
            # Use a large prefill_step_size to prevent chunking issues
            # CRITICAL: Do NOT call apply_chat_template separately to compute token count!
            # Each call to apply_chat_template pushes audio embeddings into the queue via embed_audio_chunk().
            # If we call it here AND generate() calls it internally, we get duplicate embeddings.
            # Instead, use a generous prefill_step_size (32768 tokens) to ensure the entire prompt
            # (text + audio tokens) is processed in a single prefill step.
            # Audio tokens are roughly (audio_duration_seconds * 50), text tokens are ~len(prompt_text)//4
            response = generate(
                self.model,
                self.tokenizer,
                prompt=prompt_text,
                audio=audio,
                max_tokens=2000,
                prefill_step_size=32768,  # Large enough for any prompt (prevents chunking)
                verbose=False
            )
            
            # Parse response to extract language and transcript
            # Expected format: "Language: Hindi\nTranscript: आने वाला था..."
            language = "unknown"
            transcript = response.strip()
            
            # Try to extract language from response
            if "Language:" in response:
                lines = response.split('\n')
                for line in lines:
                    if line.startswith("Language:"):
                        language = line.split(":", 1)[1].strip()
                        break
                # Remove language line from transcript
                transcript_lines = [l for l in lines if not l.startswith("Language:")]
                transcript = '\n'.join(transcript_lines)
            
            # Strip "Transcript:" prefix if present
            if transcript.startswith("Transcript:"):
                transcript = transcript.split(":", 1)[1].strip()
            
            return {
                "type": "response",
                "command": "generate-transcript",
                "language": language,
                "transcript": transcript.strip()
            }
            
        except Exception as e:
            return {"type": "error", "command": "generate-transcript", 
                    "error": f"Transcription failed: {str(e)}"}
    
    def handle_command(self, command_data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Route commands to appropriate handlers."""
        command = command_data.get("command")
        
        if command == "load-model":
            model_path = command_data.get("model_path")
            capabilities = command_data.get("capabilities")  # NEW: get capabilities from Rust
            if not model_path:
                return {"type": "error", "command": command, "error": "Missing model_path"}
            return self.load_model(model_path, capabilities)
        
        if command == "generate-transcript":
            audio_path = command_data.get("audio_path")
            if not audio_path:
                return {"type": "error", "command": command, "error": "Missing audio_path"}
            return self.generate_transcript(audio_path)
        
        # ... rest of handlers ...
```

Update `requirements.txt`:

```
mlx>=0.20.0
mlx-lm>=0.19.0
mlx-lm-omni>=0.1.3
huggingface-hub>=0.20.0
librosa>=0.10.0
soundfile>=0.12.0
numpy>=1.24.0
packaging>=20.0
```


### 4. Gem Enrichment Flow

**File**: `src-tauri/src/commands.rs`

The codebase has two enrichment entry points that both use a shared helper function:

1. **`save_gem()` command** (lines 160-203): Auto-enriches when saving a new gem
2. **`enrich_gem()` command** (lines 502-557): Manual re-enrichment of existing gems

Both call the shared `enrich_content()` helper function (lines 71-97), which is what needs to be extended with transcript generation logic.

#### Extend `enrich_content()` Helper

Update the shared helper to include transcript generation:

```rust
/// Helper function to enrich content with AI-generated metadata
/// 
/// This function calls the IntelProvider to generate tags, summary, and transcript
/// for the given content, then builds the ai_enrichment JSON structure.
/// 
/// # Arguments
/// 
/// * `provider` - The IntelProvider trait object
/// * `content` - The content to enrich (for tags/summary)
/// * `gem` - The full gem (to extract recording path for transcript)
/// * `provider_name` - The name of the provider being used
/// * `model_name` - Optional model name (for MLX provider)
/// 
/// # Returns
/// 
/// * `Ok((ai_enrichment, transcript, transcript_language))` - Enrichment data
/// * `Err(String)` - Error message if enrichment fails
async fn enrich_content(
    provider: &dyn IntelProvider,
    content: &str,
    gem: &Gem,
    provider_name: &str,
    model_name: Option<&str>,
) -> Result<(serde_json::Value, Option<String>, Option<String>), String> {
    // Run enrichment operations sequentially
    // NOTE: The MLX sidecar processes commands sequentially through a single stdin/stdout
    // channel protected by Arc<Mutex<>>. Using tokio::join! would not parallelize these
    // operations - they would serialize through the mutex anyway.
    
    // Generate tags (required)
    let tags = provider.generate_tags(content).await?;

    // Generate summary (required)
    let summary = provider.summarize(content).await?;
    
    // Generate transcript (optional - only for recording gems)
    let (transcript, transcript_language) = if let Some(recording_path) = extract_recording_path(gem) {
        match provider.generate_transcript(&recording_path).await {
            Ok(result) => (Some(result.transcript), Some(result.language)),
            Err(e) => {
                // Log error but don't fail enrichment
                eprintln!("Transcript generation failed (non-fatal): {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // Build ai_enrichment JSON
    let mut ai_enrichment = serde_json::json!({
        "tags": tags,
        "summary": summary,
        "provider": provider_name,
        "enriched_at": chrono::Utc::now().to_rfc3339(),
    });

    // Add model name if available (MLX provider has an active model)
    if let Some(model) = model_name {
        ai_enrichment["model"] = serde_json::Value::String(model.to_string());
    }

    Ok((ai_enrichment, transcript, transcript_language))
}

fn extract_recording_path(gem: &Gem) -> Option<PathBuf> {
    // Extract recording file path from gem metadata
    // IMPORTANT: This assumes recording gems have source_type "Recording" and store
    // the filename in source_meta. The actual field name needs to be verified against
    // how recording gems are created in the codebase (likely in recording.rs or commands.rs).
    // Common possibilities: "filename", "recording_path", "file", "path"
    
    if gem.source_type != "Recording" {
        return None;
    }
    
    // Try multiple possible field names for robustness
    let filename = gem.source_meta.get("filename")
        .or_else(|| gem.source_meta.get("recording_path"))
        .or_else(|| gem.source_meta.get("file"))
        .or_else(|| gem.source_meta.get("path"))
        .and_then(|v| v.as_str())?;
    
    let recordings_dir = dirs::home_dir()?.join(".jarvis/recordings");
    Some(recordings_dir.join(filename))
}
```

#### Update `save_gem()` Command

Update the auto-enrichment path to handle transcript fields:

```rust
#[tauri::command]
pub async fn save_gem(
    gist: crate::browser::extractors::PageGist,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String> {
    // Convert PageGist to Gem using the helper function
    let mut gem = page_gist_to_gem(gist);

    // Check if AI enrichment is available
    let availability = intel_provider.check_availability().await;
    
    if availability.available {
        // Get provider name and model from settings
        let (provider_name, model_name) = {
            let manager = settings_manager.read()
                .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
            let s = manager.get();
            (s.intelligence.provider.clone(), s.intelligence.active_model.clone())
        };
        let model_ref = if provider_name == "mlx" { Some(model_name.as_str()) } else { None };

        // Get content for enrichment (prefer content, fall back to description)
        let content_to_enrich = gem.content.as_ref()
            .or(gem.description.as_ref())
            .filter(|s| !s.trim().is_empty());

        if let Some(content) = content_to_enrich {
            // Try to enrich, but don't fail the save if enrichment fails
            match enrich_content(&**intel_provider, content, &gem, &provider_name, model_ref).await {
                Ok((ai_enrichment, transcript, transcript_language)) => {
                    gem.ai_enrichment = Some(ai_enrichment);
                    gem.transcript = transcript;
                    gem.transcript_language = transcript_language;
                }
                Err(e) => {
                    // Log error but continue with save
                    eprintln!("Failed to enrich gem: {}", e);
                }
            }
        }
    }

    // Save via GemStore trait (with or without enrichment)
    gem_store.save(gem).await
}
```

#### Update `enrich_gem()` Command

Update the manual re-enrichment path to handle transcript fields:

```rust
#[tauri::command]
pub async fn enrich_gem(
    app_handle: tauri::AppHandle,
    id: String,
    gem_store: State<'_, Arc<dyn GemStore>>,
    intel_provider: State<'_, Arc<dyn IntelProvider>>,
    settings_manager: State<'_, Arc<RwLock<SettingsManager>>>,
) -> Result<Gem, String> {
    // Check availability first
    let availability = intel_provider.check_availability().await;
    if !availability.available {
        return Err(format!(
            "AI enrichment not available: {}",
            availability.reason.unwrap_or_else(|| "Unknown reason".to_string())
        ));
    }
    
    // Get provider name and model from settings
    let (provider_name, model_name) = {
        let manager = settings_manager.read()
            .map_err(|e| format!("Failed to acquire settings lock: {}", e))?;
        let s = manager.get();
        (s.intelligence.provider.clone(), s.intelligence.active_model.clone())
    };
    let model_ref = if provider_name == "mlx" { Some(model_name.as_str()) } else { None };

    // Fetch gem by ID
    let mut gem = gem_store.get(&id).await?
        .ok_or_else(|| format!("Gem with id '{}' not found", id))?;

    // Get content for enrichment (prefer content, fall back to description)
    let content_to_enrich = gem.content.as_ref()
        .or(gem.description.as_ref())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Gem has no content or description to enrich".to_string())?;

    // Enrich the content
    let (ai_enrichment, transcript, transcript_language) = match enrich_content(&**intel_provider, content_to_enrich, &gem, &provider_name, model_ref).await {
        Ok(result) => result,
        Err(e) => {
            // Check if error indicates sidecar crash (broken pipe)
            if e.contains("broken pipe") || e.contains("closed connection") || e.contains("Sidecar") {
                // Emit event to frontend for toast notification
                let _ = app_handle.emit("mlx-sidecar-error", serde_json::json!({
                    "error": e.clone()
                }));
            }
            return Err(e);
        }
    };
    
    // Update gem with enrichment
    gem.ai_enrichment = Some(ai_enrichment);
    gem.transcript = transcript;
    gem.transcript_language = transcript_language;
    
    // Save and return
    gem_store.save(gem).await
}
```


## Data Models

### Gem Schema Extension

**File**: `src-tauri/src/gems/store.rs`

#### Architectural Decision: Separate Columns vs JSON Blob

**Pattern Break**: This design adds `transcript` and `transcript_language` as separate top-level columns on the Gem, breaking the existing pattern where all AI-generated enrichment data lives inside the `ai_enrichment` JSON blob (which currently contains tags and summary).

**Justification for Separate Columns**:

1. **Full-Text Search (FTS5)**: Transcripts can be very long (thousands of words for hour-long recordings). Storing them as separate columns enables efficient FTS5 indexing alongside title, description, and content. Extracting transcript from a JSON blob for FTS indexing would require complex triggers and JSON parsing.

2. **Query Performance**: Filtering or searching by transcript language (`WHERE transcript_language = 'Hindi'`) is much faster with a dedicated column than parsing JSON (`WHERE json_extract(ai_enrichment, '$.transcript_language') = 'Hindi'`).

3. **Size Considerations**: Transcripts can be 10-100x larger than tags/summary combined. Keeping them in the JSON blob would make `ai_enrichment` unwieldy and slow to parse.

4. **Different Lifecycle**: Tags and summary are always generated together as a unit. Transcripts are only generated for recording gems and may fail independently. Separate columns better reflect this independence.

5. **Future Extensibility**: Separate columns make it easier to add transcript-specific features like language filtering, transcript search, or transcript versioning without restructuring the JSON blob.

**Trade-off**: This breaks the pattern of "all AI enrichment in one place" but provides significant practical benefits for a field that behaves differently from tags/summary.

Update `Gem` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gem {
    // ... existing fields ...
    
    /// AI-generated enrichment metadata (JSON blob containing tags and summary)
    pub ai_enrichment: Option<serde_json::Value>,
    
    /// Accurate multilingual transcript from local multimodal model (NEW - separate column)
    /// Stored separately from ai_enrichment to enable FTS5 indexing and efficient queries
    pub transcript: Option<String>,
    
    /// Detected language of the transcript (e.g., "Hindi", "English") (NEW - separate column)
    pub transcript_language: Option<String>,
}
```

Update `GemPreview` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemPreview {
    // ... existing fields ...
    
    /// AI-generated summary (extracted from ai_enrichment JSON)
    pub summary: Option<String>,
    
    /// Detected language (NEW)
    /// Note: transcript_preview is intentionally omitted - the gem list view already shows
    /// content_preview. Transcript preview can be added later if needed.
    pub transcript_language: Option<String>,
}
```

### SQLite Migration

**File**: `src-tauri/src/gems/sqlite_store.rs`

Add migration to create new columns using the existing pattern:

```rust
fn initialize_schema(&self) -> Result<(), String> {
    let conn = self.conn.lock()
        .map_err(|e| format!("Failed to acquire lock: {}", e))?;
    
    // Main gems table (existing code)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS gems (
            id TEXT PRIMARY KEY,
            source_type TEXT NOT NULL,
            source_url TEXT NOT NULL UNIQUE,
            domain TEXT NOT NULL,
            title TEXT NOT NULL,
            author TEXT,
            description TEXT,
            content TEXT,
            source_meta TEXT NOT NULL,
            captured_at TEXT NOT NULL,
            ai_enrichment TEXT
        )",
        [],
    ).map_err(|e| format!("Failed to create gems table: {}", e))?;
    
    // Migration: Check for existing columns using PRAGMA table_info
    let mut stmt = conn.prepare("PRAGMA table_info(gems)")
        .map_err(|e| format!("Failed to prepare PRAGMA: {}", e))?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to query columns: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect columns: {}", e))?;
    
    // Add ai_enrichment column if it doesn't exist (existing migration)
    if !columns.contains(&"ai_enrichment".to_string()) {
        conn.execute("ALTER TABLE gems ADD COLUMN ai_enrichment TEXT", [])
            .map_err(|e| format!("Failed to add ai_enrichment column: {}", e))?;
    }
    
    // NEW: Add transcript column if it doesn't exist
    if !columns.contains(&"transcript".to_string()) {
        conn.execute("ALTER TABLE gems ADD COLUMN transcript TEXT", [])
            .map_err(|e| format!("Failed to add transcript column: {}", e))?;
    }
    
    // NEW: Add transcript_language column if it doesn't exist
    if !columns.contains(&"transcript_language".to_string()) {
        conn.execute("ALTER TABLE gems ADD COLUMN transcript_language TEXT", [])
            .map_err(|e| format!("Failed to add transcript_language column: {}", e))?;
    }
    
    // FTS5 and triggers (existing code continues...)
    // ...
}
```

**Migration Strategy**: The existing codebase uses `PRAGMA table_info(gems)` to check if columns exist before adding them. This approach is idempotent and doesn't require version tracking - the schema automatically upgrades when columns are missing.


### Settings Extension

**File**: `src-tauri/src/settings/manager.rs`

Update `TranscriptionSettings`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSettings {
    // ... existing fields ...
    
    /// Transcription engine: "whisper-rs", "whisperkit", or "mlx-omni" (NEW)
    #[serde(default = "default_transcription_engine")]
    pub transcription_engine: String,
    
    /// MLX Omni model repo ID (e.g., "giangndm/qwen2.5-omni-3b-mlx-8bit") (NEW)
    #[serde(default)]
    pub mlx_omni_model: String,
}

fn default_transcription_engine() -> String {
    "whisper-rs".to_string()
}
```

### Model Catalog Extension

**File**: `src-tauri/src/intelligence/llm_model_manager.rs`

#### Add `capabilities` Field to `LlmModelEntry`

The existing `LlmModelEntry` struct needs a NEW field to track model capabilities:

```rust
/// Static metadata for an LLM model in the catalog
struct LlmModelEntry {
    id: &'static str,
    repo_id: &'static str,
    display_name: &'static str,
    description: &'static str,
    size_estimate: &'static str,
    quality_tier: &'static str,
    capabilities: &'static [&'static str],  // NEW: ["text"], ["audio", "text"], etc.
}
```

**IMPORTANT**: This is a NEW field being added to the struct. All existing models in the catalog must be updated to include `capabilities: &["text"]`.

#### Update `LlmModelInfo` for Frontend

The `LlmModelInfo` struct (returned to frontend) also needs the capabilities field:

```rust
/// Information about an LLM model (returned to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub description: String,
    pub size_estimate: String,
    pub quality_tier: String,
    pub status: ModelStatus,
    pub capabilities: Vec<String>,  // NEW: ["text"], ["audio", "text"], etc.
}
```

#### Update Model Catalog

Update the `LLM_MODEL_CATALOG` constant to include capabilities for all models:

```rust
impl LlmModelManager {
    /// Model catalog — curated list of MLX-compatible models from mlx-community.
    /// Ordered by quality tier (basic → best) for UI display.
    const LLM_MODEL_CATALOG: &'static [LlmModelEntry] = &[
        // Existing text-only models (ADD capabilities field to each)
        LlmModelEntry {
            id: "llama-3.2-3b-4bit",
            repo_id: "mlx-community/Llama-3.2-3B-Instruct-4bit",
            display_name: "Llama 3.2 3B (Q4)",
            description: "Fast and lightweight. Good for quick tasks.",
            size_estimate: "~2 GB",
            quality_tier: "basic",
            capabilities: &["text"],  // NEW
        },
        LlmModelEntry {
            id: "qwen3-4b-4bit",
            repo_id: "mlx-community/Qwen3-4B-4bit",
            display_name: "Qwen 3 4B (Q4)",
            description: "Compact and efficient. Good balance for smaller machines.",
            size_estimate: "~3 GB",
            quality_tier: "good",
            capabilities: &["text"],  // NEW
        },
        LlmModelEntry {
            id: "qwen3-8b-4bit",
            repo_id: "mlx-community/Qwen3-8B-4bit",
            display_name: "Qwen 3 8B (Q4)",
            description: "Great quality, balanced performance. Recommended.",
            size_estimate: "~5 GB",
            quality_tier: "great",
            capabilities: &["text"],  // NEW
        },
        LlmModelEntry {
            id: "qwen3-14b-4bit",
            repo_id: "mlx-community/Qwen3-14B-4bit",
            display_name: "Qwen 3 14B (Q4)",
            description: "Highest quality. Needs 16GB+ RAM.",
            size_estimate: "~9 GB",
            quality_tier: "best",
            capabilities: &["text"],  // NEW
        },
        
        // NEW: Multimodal models
        LlmModelEntry {
            id: "qwen-omni-3b-8bit",
            repo_id: "giangndm/qwen2.5-omni-3b-mlx-8bit",
            display_name: "Qwen 2.5 Omni 3B (8-bit)",
            description: "Multilingual audio transcription + text generation",
            size_estimate: "~5 GB",
            quality_tier: "good",
            capabilities: &["audio", "text"],
        },
        LlmModelEntry {
            id: "qwen-omni-7b-4bit",
            repo_id: "giangndm/qwen2.5-omni-7b-mlx-4bit",
            display_name: "Qwen 2.5 Omni 7B (4-bit)",
            description: "Higher quality multilingual transcription + text",
            size_estimate: "~8 GB",
            quality_tier: "better",
            capabilities: &["audio", "text"],
        },
    ];
}
```

**Migration Note**: When converting `LlmModelEntry` to `LlmModelInfo`, copy the capabilities slice to a Vec:

```rust
LlmModelInfo {
    // ... other fields ...
    capabilities: entry.capabilities.iter().map(|s| s.to_string()).collect(),
}
```


### Frontend Types

**File**: `src/state/types.ts`

Update `Gem` interface:

```typescript
export interface Gem {
  // ... existing fields ...
  
  ai_enrichment: {
    tags: string[];
    summary: string;
    provider: string;
    model?: string;
    enriched_at: string;
  } | null;
  
  // NEW: Transcript fields
  transcript: string | null;
  transcript_language: string | null;
}
```

Update `GemPreview` interface:

```typescript
export interface GemPreview {
  // ... existing fields ...
  
  summary: string | null;
  enrichment_source: string | null;
  
  // NEW: Transcript language only (no preview - gem list already shows content_preview)
  transcript_language: string | null;
}
```

Update `TranscriptionSettings` interface:

```typescript
export interface TranscriptionSettings {
  // ... existing fields ...
  
  // NEW: Engine selection
  transcription_engine: "whisper-rs" | "whisperkit" | "mlx-omni";
  
  // NEW: MLX Omni model
  mlx_omni_model: string;
}
```

Update `LlmModelInfo` interface:

```typescript
export interface LlmModelInfo {
  // ... existing fields ...
  
  // NEW: Model capabilities
  capabilities: string[];  // ["text"], ["audio", "text"], etc.
}
```


## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property Reflection

After analyzing all acceptance criteria, I identified the following redundancies:
- Properties 3.3, 3.4, and 3.5 (UI model filtering by capability) can be combined into a single comprehensive property
- Properties 5.4 and 5.5 (sidecar response handling) can be combined into a single response parsing property
- Properties 7.1, 7.2, and 7.3 (UI transcript display) can be combined into a single conditional rendering property
- Properties 8.1, 8.2, and 8.3 (graceful degradation) can be combined into a single error handling property

### Property 1: Version-Gated Patches

For any version of `mlx-lm-omni`, patches should only be applied when the version is <= 0.1.3, ensuring forward compatibility when upstream fixes are released.

**Validates: Requirements 1.9**

### Property 2: Audio File Format Handling

For any valid audio file (either .wav or .pcm format), the sidecar should successfully load and convert it to the expected float32 format for model processing.

**Validates: Requirements 2.2, 2.3**

### Property 3: Transcript Generation Round Trip

For any valid audio file with a loaded multimodal model, calling `generate_transcript()` should return a result containing both a language identifier and transcript text, or return a specific error for known failure conditions.

**Validates: Requirements 2.1**

### Property 4: State Isolation Between Transcriptions

For any sequence of transcript generation calls, each call should produce independent results without state leakage from previous calls (ExtendedEmbedding queue is cleared between calls).

**Validates: Requirements 2.9**

### Property 5: Model Storage Location

For any downloaded multimodal model, the model files should be stored at `~/.jarvis/models/llm/<ModelDirName>/`, consistent with text-only LLM storage.

**Validates: Requirements 3.2**


### Property 6: UI Model Filtering by Capability

For any model in the catalog, it should appear in the Settings UI sections corresponding to its capabilities: models with "audio" capability appear in the MLX Omni transcription section, models with "text" capability appear in the Intelligence section, and models with both appear in both sections.

**Validates: Requirements 3.3, 3.4, 3.5**

### Property 7: Enrichment Method Invocation

For any gem created from a recording (has recording file path), the enrichment flow should call `generate_tags()`, `summarize()`, and `generate_transcript()`. For any gem without a recording file path, only `generate_tags()` and `summarize()` should be called.

**Validates: Requirements 4.3, 4.4**

### Property 8: Graceful Enrichment Failure

For any transcript generation failure (timeout, missing model, unsupported model, file not found), the enrichment flow should log the error, leave transcript fields as NULL, and successfully complete tags and summary generation without propagating the error.

**Validates: Requirements 4.5, 8.1, 8.2, 8.3**

### Property 9: Default Trait Implementation

For any `IntelProvider` implementation that doesn't override `generate_transcript()`, calling the method should return an error indicating transcript generation is not supported by that provider.

**Validates: Requirements 4.7**

### Property 10: Gem Serialization Completeness

For any gem with non-null transcript fields, the serialized JSON sent to the frontend should include both `transcript` and `transcript_language` fields with their values.

**Validates: Requirements 4.8**

### Property 11: Sidecar Command Protocol

For any audio file path, calling `MlxProvider.generate_transcript()` should send a correctly formatted NDJSON command `{"command":"generate-transcript","audio_path":"<path>"}` to the sidecar and correctly parse the response into either `Ok(TranscriptResult)` or `Err(String)` based on the response type.

**Validates: Requirements 5.2, 5.4, 5.5**

### Property 12: Real-Time Transcription Independence

For any transcription engine setting value, when `transcription_engine` is set to "mlx-omni", real-time transcription during recording should continue using Whisper (unchanged behavior), and MLX Omni should only be used for post-recording gem enrichment.

**Validates: Requirements 6.5**


### Property 13: UI Conditional Transcript Display

For any gem, the transcript section should be displayed if and only if the `transcript` field is non-null. When displayed, it should show the detected language. When both Whisper (content) and MLX (transcript) transcripts exist, the MLX transcript should be displayed prominently.

**Validates: Requirements 7.1, 7.2, 7.3**

### Property 14: Settings-Based Enrichment Control

For any gem enrichment operation, when `transcription_engine` is not set to "mlx-omni", the system should skip transcript generation entirely, even if a multimodal model is loaded and available.

**Validates: Requirements 8.6**

## Error Handling

### Sidecar Communication Errors

**Broken Pipe**: If the sidecar process crashes during transcript generation, the `send_command()` method will detect the broken pipe (empty response line) and return an error. The enrichment flow catches this error and leaves transcript fields as NULL without failing the overall enrichment.

**Timeout**: Transcript generation uses a 120-second timeout (longer than tags/summary's 60s). If exceeded, `tokio::time::timeout` returns an error, which is caught by the enrichment flow and logged.

**Invalid Response**: If the sidecar returns malformed JSON, `serde_json::from_str` fails and returns an error, which is handled gracefully.

### Model Compatibility Errors

**No Model Loaded**: If `generate-transcript` is called before a model is loaded, the sidecar returns `{"type":"error","command":"generate-transcript","error":"No model loaded"}`. The Rust side converts this to `Err(String)`.

**Text-Only Model**: If a text-only LLM is loaded (capabilities do not include "audio"), the sidecar returns `{"type":"error","command":"generate-transcript","error":"Loaded model does not support audio transcription"}`. The enrichment flow catches this and skips transcript generation.

### File System Errors

**Missing Audio File**: If the recording file has been deleted or moved, the sidecar returns `{"type":"error","command":"generate-transcript","error":"Audio file not found: ..."}`. The enrichment flow logs this and continues.

**Invalid Format**: If the audio file is neither .wav nor .pcm, the sidecar returns an unsupported format error.

### Dependency Errors

**Missing mlx-lm-omni**: If `mlx-lm-omni` is not installed in the venv, the sidecar will fail to import it at startup. The `MlxProvider::new()` method will fail during availability check, and the provider won't be initialized. The app falls back to IntelligenceKit or NoOp provider.

**Missing librosa/soundfile**: If these dependencies are missing, the `generate_transcript()` method will fail with an import error. The enrichment flow catches this and skips transcript generation.


### Graceful Degradation Strategy

The system is designed to degrade gracefully at multiple levels:

1. **Provider Level**: If MLX provider fails to initialize, fall back to IntelligenceKit (tags/summary only, no transcript)
2. **Enrichment Level**: If transcript generation fails, tags and summary still succeed
3. **UI Level**: If transcript is NULL, UI simply doesn't show a transcript section (no error state)
4. **Settings Level**: If no multimodal model is downloaded, show a helpful message in Settings

This ensures users never lose access to core functionality (recording, tags, summary) even when transcript generation is unavailable.

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests for comprehensive coverage:

**Unit Tests** focus on:
- Specific examples (e.g., startup patch logging, Settings UI with MLX Omni selected)
- Edge cases (e.g., empty audio files, missing files, no model loaded)
- Integration points (e.g., SQLite migration, enrichment flow coordination)
- Error conditions (e.g., sidecar crashes, timeouts, invalid responses)

**Property-Based Tests** focus on:
- Universal properties across all inputs (e.g., all .wav files load correctly, all gems serialize with transcript fields)
- Version-gated behavior (e.g., patches only apply to old versions)
- Conditional logic (e.g., transcript generation only for recording gems)
- State isolation (e.g., consecutive transcriptions don't interfere)

### Property-Based Testing Configuration

**Library**: Use `proptest` for Rust property-based testing, `fast-check` for TypeScript

**Iteration Count**: Minimum 100 iterations per property test (due to randomization)

**Test Tagging**: Each property test must reference its design document property:
```rust
// Feature: mlx-omni-transcription, Property 2: Audio File Format Handling
#[test]
fn prop_audio_file_format_handling() {
    // ...
}
```

### Unit Test Examples

**Patch Logging (Requirement 1.8)**:
```rust
#[test]
fn test_patches_logged_at_startup() {
    // Start sidecar, capture stderr
    // Assert stderr contains "Applied patches: AudioTower.__call__, ..."
}
```

**Database Migration (Requirement 4.2)**:
```rust
#[test]
fn test_transcript_columns_added() {
    // Create v1 database
    // Run migration
    // Assert transcript and transcript_language columns exist
}
```

**Settings UI (Requirement 6.1, 6.2)**:
```rust
#[test]
fn test_mlx_omni_option_appears() {
    // Render Settings component
    // Assert "MLX Omni (Local, Private)" option exists in dropdown
}

#[test]
fn test_model_picker_shown_when_selected() {
    // Set transcription_engine to "mlx-omni"
    // Render Settings
    // Assert model picker is visible
}
```


**Edge Cases (Requirements 2.4, 2.5, 2.6)**:
```rust
#[test]
fn test_no_model_loaded_error() {
    // Send generate-transcript without loading model
    // Assert error response
}

#[test]
fn test_missing_file_error() {
    // Send generate-transcript with non-existent path
    // Assert error response
}

#[test]
fn test_empty_audio_returns_empty_transcript() {
    // Create empty .pcm file
    // Generate transcript
    // Assert language="unknown", transcript=""
}
```

**Loading Indicator (Requirement 7.4)**:
```typescript
test('shows loading indicator during enrichment', () => {
  // Render gem detail with enrichment in progress
  // Assert loading indicator is visible in transcript area
});
```

### Property-Based Test Examples

**Property 1: Version-Gated Patches**:
```rust
// Feature: mlx-omni-transcription, Property 1: Version-Gated Patches
#[proptest]
fn prop_patches_only_applied_to_old_versions(version: String) {
    // For any version string
    // If version <= "0.1.3", patches should be applied
    // If version > "0.1.3", patches should be skipped
}
```

**Property 2: Audio File Format Handling**:
```rust
// Feature: mlx-omni-transcription, Property 2: Audio File Format Handling
#[proptest]
fn prop_wav_files_load_correctly(#[strategy(valid_wav_file())] wav_path: PathBuf) {
    // For any valid .wav file
    // Loading should succeed and return float32 audio
}

#[proptest]
fn prop_pcm_files_load_correctly(#[strategy(valid_pcm_file())] pcm_path: PathBuf) {
    // For any valid .pcm file (s16le, 16kHz, mono)
    // Loading should succeed and return float32 audio
}
```

**Property 3: Transcript Generation Round Trip**:
```rust
// Feature: mlx-omni-transcription, Property 3: Transcript Generation Round Trip
#[proptest]
fn prop_transcript_generation_returns_result_or_error(
    #[strategy(valid_audio_file())] audio_path: PathBuf
) {
    // For any valid audio file with model loaded
    // generate_transcript should return Ok(TranscriptResult) with language and transcript
    // OR return Err with specific error message
}
```

**Property 4: State Isolation Between Transcriptions**:
```rust
// Feature: mlx-omni-transcription, Property 4: State Isolation Between Transcriptions
#[proptest]
fn prop_consecutive_transcriptions_independent(
    #[strategy(vec(valid_audio_file(), 2..5))] audio_files: Vec<PathBuf>
) {
    // For any sequence of audio files
    // Transcribing file N should not affect the result of transcribing file N+1
    // (queue is cleared between calls)
}
```

**Property 5: Model Storage Location**:
```rust
// Feature: mlx-omni-transcription, Property 5: Model Storage Location
#[proptest]
fn prop_models_stored_in_correct_location(
    #[strategy(multimodal_model_id())] model_id: String
) {
    // For any multimodal model ID
    // After download, model should exist at ~/.jarvis/models/llm/<model_id>/
}
```

**Property 6: UI Model Filtering by Capability**:
```typescript
// Feature: mlx-omni-transcription, Property 6: UI Model Filtering by Capability
fc.assert(
  fc.property(fc.record({
    id: fc.string(),
    capabilities: fc.array(fc.constantFrom('audio', 'text'), { minLength: 1 })
  }), (model) => {
    // For any model with capabilities
    // If capabilities includes "audio", model appears in MLX Omni section
    // If capabilities includes "text", model appears in Intelligence section
  })
);
```

**Property 7: Enrichment Method Invocation**:
```rust
// Feature: mlx-omni-transcription, Property 7: Enrichment Method Invocation
#[proptest]
fn prop_enrichment_calls_correct_methods(#[strategy(gem_with_optional_recording())] gem: Gem) {
    // For any gem
    // If gem has recording file path, all three methods called
    // If gem has no recording file path, only tags and summary called
}
```

**Property 8: Graceful Enrichment Failure**:
```rust
// Feature: mlx-omni-transcription, Property 8: Graceful Enrichment Failure
#[proptest]
fn prop_transcript_failure_doesnt_fail_enrichment(
    #[strategy(transcript_error_scenario())] error: TranscriptError
) {
    // For any transcript generation failure
    // Enrichment should complete successfully with tags and summary
    // Transcript fields should be NULL
}
```

**Property 10: Gem Serialization Completeness**:
```rust
// Feature: mlx-omni-transcription, Property 10: Gem Serialization Completeness
#[proptest]
fn prop_gems_with_transcript_serialize_completely(
    #[strategy(gem_with_transcript())] gem: Gem
) {
    // For any gem with non-null transcript
    // Serialized JSON should include transcript and transcript_language fields
}
```

**Property 13: UI Conditional Transcript Display**:
```typescript
// Feature: mlx-omni-transcription, Property 13: UI Conditional Transcript Display
fc.assert(
  fc.property(fc.record({
    transcript: fc.option(fc.string()),
    content: fc.option(fc.string())
  }), (gem) => {
    // For any gem
    // Transcript section visible iff transcript is non-null
    // When both transcripts exist, MLX is prominent
  })
);
```


## UI Design

### Settings Page - Transcription Engine Selection

**Location**: Settings → Transcription tab

**Layout**:
```
┌─────────────────────────────────────────────────────────────┐
│ Transcription Engine                                        │
│                                                             │
│ ○ Whisper (Local)                                          │
│   Fast, accurate English transcription using whisper.cpp   │
│                                                             │
│ ○ WhisperKit (macOS Native)                                │
│   Apple Neural Engine acceleration (macOS 15+)             │
│                                                             │
│ ● MLX Omni (Local, Private)                                │
│   Multilingual audio transcription using MLX models        │
│                                                             │
│   ┌───────────────────────────────────────────────────┐   │
│   │ Multimodal Models                                 │   │
│   │                                                   │   │
│   │ ┌─────────────────────────────────────────────┐ │   │
│   │ │ ● Qwen 2.5 Omni 3B (8-bit)         [ACTIVE] │ │   │
│   │ │   ~5 GB • good quality                      │ │   │
│   │ │   Multilingual transcription + text         │ │   │
│   │ └─────────────────────────────────────────────┘ │   │
│   │                                                   │   │
│   │ ┌─────────────────────────────────────────────┐ │   │
│   │ │ ○ Qwen 2.5 Omni 7B (4-bit)      [Download]  │ │   │
│   │ │   ~8 GB • better quality                    │ │   │
│   │ │   Higher accuracy multilingual              │ │   │
│   │ └─────────────────────────────────────────────┘ │   │
│   │                                                   │   │
│   │ Venv Status: ✓ Ready                             │   │
│   └───────────────────────────────────────────────────┘   │
│                                                             │
│ Note: Real-time transcription during recording still uses  │
│ Whisper for instant feedback. MLX Omni provides accurate   │
│ multilingual transcripts after recording completes.         │
└─────────────────────────────────────────────────────────────┘
```

**Behavior**:
- When "MLX Omni" is selected, show the multimodal models panel
- Models with `capabilities: ["audio", ...]` appear in this list
- Show download button for models not yet downloaded
- Show progress bar during download
- Show "Active" badge on the currently selected model
- Show venv status indicator (not set up / ready / needs update)
- If no models downloaded, show: "Download a multimodal model to enable MLX transcription"

### Settings Page - Intelligence Section

**Layout**:
```
┌─────────────────────────────────────────────────────────────┐
│ Intelligence Provider                                       │
│                                                             │
│ ● MLX (Local, Private)                                     │
│   Local AI using Apple Silicon                             │
│                                                             │
│   ┌───────────────────────────────────────────────────┐   │
│   │ Text Models                                       │   │
│   │                                                   │   │
│   │ ┌─────────────────────────────────────────────┐ │   │
│   │ │ ● Qwen 3 8B (Q4)                   [ACTIVE] │ │   │
│   │ │   ~5 GB • great quality                     │ │   │
│   │ │   Fast, high-quality text generation        │ │   │
│   │ └─────────────────────────────────────────────┘ │   │
│   │                                                   │   │
│   │ ┌─────────────────────────────────────────────┐ │   │
│   │ │ ○ Qwen 2.5 Omni 3B (8-bit)      [Download]  │ │   │
│   │ │   ~5 GB • good quality                      │ │   │
│   │ │   Multilingual transcription + text         │ │   │
│   │ └─────────────────────────────────────────────┘ │   │
│   └───────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**Behavior**:
- Models with `capabilities: ["text", ...]` appear in this list
- Multimodal models (with both "audio" and "text") appear in BOTH sections
- Using one multimodal model for both reduces total memory usage


### Gem Detail View - Transcript Display

**Layout** (when transcript exists):
```
┌─────────────────────────────────────────────────────────────┐
│ Recording from March 15, 2024                               │
│                                                             │
│ ┌─────────────────────────────────────────────────────┐   │
│ │ Transcript (Hindi)                                  │   │
│ │                                                     │   │
│ │ आने वाला था मैं तो बस इंतज़ार कर रहा था...         │   │
│ │                                                     │   │
│ │ [Full multilingual transcript from MLX Omni]       │   │
│ └─────────────────────────────────────────────────────┘   │
│                                                             │
│ ┌─────────────────────────────────────────────────────┐   │
│ │ ▼ Real-time Transcript (Whisper)                   │   │
│ │                                                     │   │
│ │ [Collapsed by default - English-biased transcript] │   │
│ └─────────────────────────────────────────────────────┘   │
│                                                             │
│ Tags: #meeting #discussion #planning                       │
│ Summary: Discussion about upcoming project timeline...     │
└─────────────────────────────────────────────────────────────┘
```

**Layout** (when transcript is being generated):
```
┌─────────────────────────────────────────────────────────────┐
│ ┌─────────────────────────────────────────────────────┐   │
│ │ Transcript                                          │   │
│ │                                                     │   │
│ │ ⏳ Generating accurate multilingual transcript...  │   │
│ │                                                     │   │
│ └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**Layout** (when no transcript):
```
┌─────────────────────────────────────────────────────────────┐
│ Recording from March 15, 2024                               │
│                                                             │
│ ┌─────────────────────────────────────────────────────┐   │
│ │ Transcript (Whisper)                                │   │
│ │                                                     │   │
│ │ [Real-time transcript from recording]              │   │
│ └─────────────────────────────────────────────────────┘   │
│                                                             │
│ Tags: #meeting #discussion                                 │
│ Summary: Discussion about project...                       │
└─────────────────────────────────────────────────────────────┘
```

**Behavior**:
- If `transcript` is non-null, show it prominently with detected language
- If both `transcript` and `content` exist, show MLX transcript first, Whisper transcript collapsed
- If `transcript` is null, show only Whisper transcript (from `content` field)
- During enrichment, show loading indicator in transcript area
- No empty placeholder or "pending" state when transcript is null


## Implementation Notes

### Runtime Patch Details

The six bugs in `mlx-lm-omni` v0.1.3 that require patching:

1. **AudioTower reshape bug**: The reshape operation happens before the transformer loop, causing all audio chunks to be concatenated into a single batch element. This fails for audio longer than ~15 seconds. Fix: Move reshape after the loop so each chunk processes independently. **[Runtime Patch]**

2. **AudioMel precision loss**: Using float16 for mel filter computation causes precision loss on quiet audio. Fix: Use float32 for `mel_filters`, `waveform`, and `window`. **[Runtime Patch]**

3. **ExtendedQuantizedEmbedding kwargs**: The `to_quantized()` method doesn't accept `**kwargs`, causing compatibility issues with MLX 0.30+. Fix: Add `**kwargs` parameter. **[Runtime Patch]**

4. **Model attribute delegation**: The Model class doesn't delegate attribute access to the inner tokenizer, causing `AttributeError` when accessing tokenizer properties. Fix: Add `__getattr__` method and `chat_template` property. **[Runtime Patch]**

5. **7B model conv layout**: Some 7B models have conv weights in PyTorch layout (channels_first) instead of MLX layout (channels_last). Fix: Auto-detect by checking `conv1.weight.shape[1] != 3` and apply `mx.swapaxes(weight, 1, 2)` to both conv1 and conv2. **[Runtime Patch]**

6. **Prefill chunking bug**: The default prefill chunking splits the audio token region across multiple chunks, causing `IndexError: pop from empty list` on audio longer than ~30 seconds. Fix: Use a large `prefill_step_size=32768` to ensure the entire prompt (text + audio tokens) is processed in a single prefill step. **[Call-Site Fix]**

**Summary**: 5 bugs are fixed via runtime monkey-patching in `apply_runtime_patches()`, and 1 bug is fixed at the call-site via the `prefill_step_size` parameter. All patches are version-gated to only apply when `mlx_lm_omni.__version__ <= "0.1.3"`, ensuring they're automatically disabled when upstream fixes are released.

### Audio Format Conversion

Jarvis recordings are stored as raw PCM (s16le, 16kHz, mono). The sidecar must convert this to float32 for model processing:

```python
# Read raw PCM
with open(audio_path, 'rb') as f:
    pcm_data = np.frombuffer(f.read(), dtype=np.int16)

# Convert to float32 [-1.0, 1.0]
audio = pcm_data.astype(np.float32) / 32768.0
```

For .wav files, librosa handles the conversion automatically:
```python
audio, sr = librosa.load(audio_path, sr=16000, mono=True)
```

### State Management

The `ExtendedEmbedding` class in `mlx-lm-omni` maintains a queue that can leak state between transcription calls. To prevent this, the sidecar clears the queue before each transcription:

```python
# The queue lives on the embedding layer, not attention layers
if hasattr(self.model, 'language_model') and hasattr(self.model.language_model, 'model'):
    embed = self.model.language_model.model.embed_tokens
    if hasattr(embed, 'extended_embedding_queue'):
        embed.extended_embedding_queue.clear()
```

This ensures each transcription is independent and doesn't inherit artifacts from previous calls.

### Timeout Strategy

Different operations have different timeout requirements:

- **Availability check**: 15s (allows for initial imports)
- **Model loading**: 15s (model weights load quickly from disk)
- **Tag generation**: 60s (short text generation)
- **Summarization**: 60s (short text generation)
- **Transcript generation**: 120s (long audio can take time, especially on 3B models)

The longer timeout for transcription accounts for:
- Audio preprocessing (mel spectrogram computation)
- Model inference on long audio (up to 2000 tokens output)
- Slower inference on smaller models (3B vs 7B)


### Venv Management

The MLX sidecar uses a managed Python virtual environment at `~/.jarvis/venv/mlx/`. When `requirements.txt` is updated to include the new dependencies (`mlx-lm-omni`, `librosa`, `soundfile`, `numpy`), the `VenvManager` detects the change via SHA-256 hash comparison and marks the venv as "needs update".

The Settings UI shows the venv status and provides a button to update it. The update process:
1. Activates the venv
2. Runs `pip install -r requirements.txt`
3. Updates the marker file with the new hash
4. Marks venv as "ready"

This ensures users get the new dependencies without manual intervention.

### Model Catalog Organization

Models are organized by capability in the catalog:

```rust
capabilities: vec!["text".to_string()]                    // Text-only LLM
capabilities: vec!["audio".to_string(), "text".to_string()] // Multimodal
```

The Settings UI filters models based on context:
- **Transcription section**: Show models with "audio" capability
- **Intelligence section**: Show models with "text" capability
- **Both sections**: Multimodal models appear in both

This allows users to:
- Use a text-only model for tags/summary (lower memory)
- Use a multimodal model for transcription only
- Use a single multimodal model for both (most memory-efficient)

### Sequential Enrichment

The enrichment flow runs three operations sequentially:

```rust
let tags_result = provider.generate_tags(content).await;
let summary_result = provider.summarize(content).await;

let transcript_result = if let Some(recording_path) = extract_recording_path(&gem) {
    Some(provider.generate_transcript(&recording_path).await)
} else {
    None
};
```

**Why Sequential?** The MLX sidecar processes commands sequentially through a single stdin/stdout channel protected by `Arc<Mutex<>>`. Three concurrent `send_command()` calls would serialize through the mutex anyway, so using `tokio::join!` provides no benefit. The sidecar reads one NDJSON command at a time, processes it, responds, then reads the next.

Total enrichment time:
- **Tags**: ~5s
- **Summary**: ~5s  
- **Transcript**: ~12s (for 27s audio on 3B model)
- **Total**: ~22s

Future optimization: If multiple provider types are used (e.g., IntelligenceKit for tags/summary, MLX for transcript), those could run in parallel since they use separate processes.

### Database Schema Evolution

The SQLite schema uses `PRAGMA table_info(gems)` to check for column existence before adding new columns. This approach is idempotent and doesn't require version tracking:

```rust
// Check if columns exist
let columns: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(1))
    .collect::<Result<Vec<_>, _>>()?;

// Add columns only if they don't exist
if !columns.contains(&"transcript".to_string()) {
    conn.execute("ALTER TABLE gems ADD COLUMN transcript TEXT", [])?;
}
```

This matches the existing pattern used for the `ai_enrichment` column migration. The migration runs automatically on every app startup and is safe to run multiple times. Existing gems will have NULL transcript fields until they're re-enriched.


## Security and Privacy

### Local-Only Processing

All audio transcription happens locally on the user's machine:
- Audio files never leave the device
- No network requests during transcription
- Models are downloaded once and cached locally
- No telemetry or usage tracking

This ensures complete privacy for sensitive recordings (meetings, calls, personal notes).

### File System Access

The sidecar only accesses files explicitly provided by the Rust backend:
- Recording files at `~/.jarvis/recordings/*.pcm`
- Model files at `~/.jarvis/models/llm/*/`
- No arbitrary file system access

The Rust backend validates all paths before passing them to the sidecar.

### Dependency Security

All Python dependencies are pinned in `requirements.txt` with minimum versions:
- `mlx>=0.20.0` - Apple's MLX framework
- `mlx-lm>=0.19.0` - MLX language model utilities
- `mlx-lm-omni>=0.1.3` - Multimodal model support
- `librosa>=0.10.0` - Audio processing
- `soundfile>=0.12.0` - Audio file I/O
- `numpy>=1.24.0` - Numerical computing

The venv is isolated from the system Python environment, preventing conflicts and ensuring reproducibility.

### Model Provenance

All models are downloaded from HuggingFace Hub with verified repo IDs:
- `giangndm/qwen2.5-omni-3b-mlx-8bit` - Community-maintained MLX conversion
- `giangndm/qwen2.5-omni-7b-mlx-4bit` - Community-maintained MLX conversion

Users can verify model checksums and inspect model cards on HuggingFace before downloading.

## Performance Considerations

### Memory Usage

Multimodal models require significant memory:
- **Qwen 2.5 Omni 3B (8-bit)**: ~5.3 GB peak during inference
- **Qwen 2.5 Omni 7B (4-bit)**: ~8 GB peak during inference

The app should warn users if available memory is low before starting enrichment.

### Inference Speed

Transcription speed depends on model size and audio length:
- **3B model**: ~0.43x realtime (12s for 27.6s audio)
- **7B model**: ~0.6x realtime (faster, more accurate)

For long recordings (>5 minutes), transcription may take 2-3 minutes. The UI should show progress indication during enrichment.

### Model Loading Time

Models load quickly from disk (~1.5s for 3B model) because MLX uses memory-mapped files. The model stays loaded in the sidecar process, so subsequent transcriptions don't incur loading overhead.

### Disk Space

Users need sufficient disk space for models:
- **3B model**: ~5 GB
- **7B model**: ~8 GB
- **Both models**: ~13 GB

The Settings UI shows size estimates before download and available disk space.


## Migration Path

### For Existing Users

Users with existing gems will see:
1. **Immediate**: Settings UI shows new "MLX Omni" transcription engine option
2. **After venv update**: Venv status shows "needs update" due to new dependencies
3. **After model download**: Can select MLX Omni as transcription engine
4. **After re-enrichment**: Existing recording gems can be re-enriched to add transcripts

Existing gems are not automatically re-enriched. Users can trigger re-enrichment manually (future feature) or new recordings will automatically get transcripts.

### Backward Compatibility

The design maintains backward compatibility:
- **Settings**: New fields have `#[serde(default)]`, so old `settings.json` files work
- **Database**: Migration adds columns with NULL default, so existing gems work
- **Provider trait**: Default implementation of `generate_transcript()` ensures existing providers work
- **Frontend**: Gems without transcript fields render normally (no transcript section shown)

### Rollback Strategy

If issues arise, users can:
1. Switch transcription engine back to "Whisper" or "WhisperKit" in Settings
2. Existing tags and summaries continue to work (transcript generation is optional)
3. Delete multimodal models to free disk space if needed
4. Venv can be deleted and recreated without losing data

## Future Enhancements

### Out of Scope (for this feature)

The following are explicitly out of scope but could be added later:

1. **Streaming transcription**: Currently transcription happens post-recording. Streaming would require significant changes to the sidecar protocol.

2. **Speaker diarization**: Identifying who said what in multi-speaker recordings. Would require additional models or services.

3. **Translation**: Automatically translating transcripts to other languages. Current design transcribes in the original language only.

4. **Custom prompts**: Allowing users to customize the transcription prompt. Current design uses a fixed prompt optimized for accuracy.

5. **Video/image understanding**: Extending to visual modalities. Current design is audio-only.

6. **Cloud transcription**: Using cloud APIs (Whisper API, Google Speech-to-Text, etc.). Current design is local-only.

7. **Batch re-enrichment**: Automatically re-enriching all existing recording gems. Would require background job system.

### Potential Improvements

1. **Progress reporting**: Show real-time progress during transcript generation (e.g., "Processing audio... 45%")

2. **Quality settings**: Allow users to choose between speed and accuracy (e.g., max_tokens, temperature)

3. **Language hints**: Allow users to specify expected language to improve accuracy

4. **Transcript editing**: Allow users to manually correct transcripts

5. **Export formats**: Export transcripts as .txt, .srt (subtitles), or .vtt

6. **Search integration**: Make transcripts searchable in the gems search interface

## Dependencies

### Rust Crates

No new Rust dependencies required. Existing crates handle all functionality:
- `tokio` - Async runtime and process management
- `serde` / `serde_json` - Serialization
- `rusqlite` - SQLite database
- `async-trait` - Trait async methods

### Python Packages

New dependencies in `requirements.txt`:
- `mlx-lm-omni>=0.1.3` - Multimodal model support
- `librosa>=0.10.0` - Audio loading and processing
- `soundfile>=0.12.0` - Audio file I/O backend for librosa
- `numpy>=1.24.0` - Numerical operations (already a transitive dependency)

### System Requirements

- **macOS 15.0+**: Required for ScreenCaptureKit microphone capture (existing requirement)
- **Apple Silicon**: Required for MLX (existing requirement)
- **Python 3.10+**: Required for MLX (existing requirement)
- **~10 GB disk space**: For multimodal models (new requirement)
- **8-16 GB RAM**: For model inference (new requirement)

## Conclusion

This design extends Jarvis with local multimodal audio transcription by:
1. Reusing the existing MLX sidecar infrastructure (no new processes)
2. Adding a new `generate-transcript` command to the sidecar protocol
3. Extending the `IntelProvider` trait with transcript generation
4. Storing transcripts as new fields on the Gem data model
5. Providing a Settings UI for model selection and download
6. Maintaining graceful degradation when transcription is unavailable

The design prioritizes:
- **Privacy**: All processing happens locally, no data leaves the device
- **Simplicity**: Reuses existing patterns and infrastructure
- **Reliability**: Graceful degradation ensures core features always work
- **Extensibility**: Model-agnostic design supports future multimodal models

Users get accurate multilingual transcripts for their recordings while maintaining full control over their data.
