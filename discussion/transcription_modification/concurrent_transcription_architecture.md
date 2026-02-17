# Concurrent Transcription Architecture

## Problem

Vosk partials (grey text) never appear in the UI. Only Whisper finals (black text) show up after a 1-2s delay. The user should see instant Vosk text within ~10ms, which then gets replaced by accurate Whisper text 1-2s later.

---

## Current Architecture

### Flow

```
Audio Chunk (from AudioRouter)
        |
        v
TranscriptionManager (main loop)
        |
        |  push to AudioBuffer, extract 3s window
        |
        v
  block_in_place {              <-- blocks current tokio thread
    provider.lock()
    provider.transcribe(audio)  <-- single call, runs sequentially inside
  }
        |
        v
  HybridProvider.transcribe()
        |
        |  1. VAD check (~0.1ms)
        |  2. Vosk partial (~10ms) --> emit("transcription-update") directly
        |  3. Whisper final (~1-2s) --> returned in Vec
        |
        v
  Manager receives Vec<Segment>
        |
        |  for each segment:
        |    emit("transcription-update")
        |
        v
  Frontend receives events
```

### Why Vosk Partials Don't Appear

1. **Everything runs inside `block_in_place`**: Steps 2 and 3 both execute on the same blocked thread
2. **Events queue up**: The Vosk partial event emitted in step 2 and the Whisper final event emitted in step 4 both reach the webview at nearly the same time (when the blocked thread finishes)
3. **React batches**: Both events land in the same React render cycle. `processTranscript()` immediately replaces the partial with the final
4. **Result**: User sees only the Whisper final, never the Vosk partial

### Key Code Paths

**manager.rs** - Transcription loop:
```rust
// Everything happens inside block_in_place - no events can be
// delivered to webview until this entire block returns
let transcribe_result = tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async {
        provider.lock().await.transcribe(&audio)
    })
});

// Only AFTER block_in_place returns do we emit events
match transcribe_result {
    Ok(segments) => {
        for segment in segments {
            app_handle.emit("transcription-update", &segment);
        }
    }
}
```

**hybrid_provider.rs** - transcribe():
```rust
fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>> {
    // Step 1: VAD check
    // Step 2: Vosk partial - emitted directly via app_handle (BUT still inside block_in_place)
    if let Some(partial) = self.process_vosk(audio) {
        if let Some(handle) = &self.app_handle {
            handle.emit("transcription-update", &partial); // queued, not delivered
        }
    }
    // Step 3: Whisper final - blocks for 1-2s
    let whisper_segments = self.whisper.transcribe(audio)?;
    segments.extend(whisper_segments);
    Ok(segments)
}
```

### The Core Problem

`block_in_place` tells tokio "this thread is busy". Other tokio tasks CAN run on other worker threads. BUT the webview IPC events emitted from inside this block appear to be delivered together when the block completes, giving React no time to render the partial before the final arrives.

---

## Proposed Architecture

### Key Idea

Separate the **production** of transcription results from the **rendering** of them using an mpsc channel. A dedicated renderer task reads from the channel and emits events. Since the renderer runs on a different tokio task (different worker thread), it can emit events to the webview while Whisper is still blocking.

### Flow

```
Audio Chunk (from AudioRouter)
        |
        v
TranscriptionManager (main loop)
        |
        |  push to AudioBuffer, extract 3s window
        |
        v
  block_in_place {
    provider.lock()
    provider.transcribe(audio)
      |
      |  1. VAD check (~0.1ms)
      |  2. Vosk partial (~10ms) --> sends to channel immediately
      |  3. Whisper final (~1-2s) --> sends to channel when done
      |
      |  returns Ok(()) -- results go through channel, not return value
  }

                            CHANNEL (unbounded mpsc)
                                    |
                                    v
                          Renderer Task (separate tokio task, different thread)
                                    |
                                    |  receives segments as they arrive:
                                    |    - Vosk partial arrives at T+10ms
                                    |    - Whisper final arrives at T+1500ms
                                    |
                                    |  for each segment:
                                    |    push to transcript Vec
                                    |    emit("transcription-update")
                                    |
                                    v
                          Frontend receives events with real time gaps
                                    |
                                    |  T+10ms: renders grey Vosk partial
                                    |  T+1500ms: replaces with black Whisper final
```

### Why This Works

1. **Vosk sends to channel immediately**: After Vosk finishes (~10ms), its partial is in the channel
2. **Renderer runs on a different thread**: `block_in_place` frees up other worker threads. The renderer task picks up the Vosk partial and emits it to the webview
3. **Real time gap**: The webview receives the Vosk event ~1.5s BEFORE the Whisper event. React renders the partial, user sees grey text
4. **Whisper sends later**: When Whisper finishes, its final goes through the same channel. Renderer emits it. Frontend replaces partial with final

### Changes Required

#### 1. provider.rs - Add channel support to trait

```rust
pub trait TranscriptionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn initialize(&mut self, config: &TranscriptionConfig) -> Result<(), Box<dyn Error>>;
    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>>;

    /// Set a channel sender for streaming results as they become available.
    /// When set, transcribe() sends results through this channel.
    /// Default: no-op (provider returns results via return value as before)
    fn set_segment_sender(&mut self, _tx: tokio::sync::mpsc::UnboundedSender<TranscriptionSegment>) {}

    /// Clear the channel sender (closes the channel from producer side).
    /// Default: no-op
    fn clear_segment_sender(&mut self) {}
}
```

Default no-op implementations mean existing providers (MockProvider, tests) work unchanged.

#### 2. hybrid_provider.rs - Send results through channel

```rust
pub struct HybridProvider {
    vad: Option<SileroVad>,
    vosk: Option<VoskProvider>,
    whisper: WhisperProvider,
    segment_tx: Option<UnboundedSender<TranscriptionSegment>>,  // replaces app_handle
}

impl TranscriptionProvider for HybridProvider {
    fn set_segment_sender(&mut self, tx: UnboundedSender<TranscriptionSegment>) {
        self.segment_tx = Some(tx);
    }

    fn clear_segment_sender(&mut self) {
        self.segment_tx = None;  // drops sender, closing the channel
    }

    fn transcribe(&mut self, audio: &[f32]) -> Result<Vec<TranscriptionSegment>, Box<dyn Error>> {
        // 1. VAD check (unchanged)
        // ...

        // 2. Vosk partial - send through channel immediately
        if let Some(partial) = self.process_vosk(audio) {
            if let Some(tx) = &self.segment_tx {
                let _ = tx.send(partial);
                // partial is now in the channel, renderer can pick it up
                // while Whisper blocks below
            }
        }

        // 3. Whisper final - blocks 1-2s, then sends through channel
        let whisper_segments = self.whisper.transcribe(audio)?;
        if let Some(tx) = &self.segment_tx {
            for seg in &whisper_segments {
                let _ = tx.send(seg.clone());
            }
            Ok(vec![])  // results went through channel
        } else {
            Ok(whisper_segments)  // fallback: return normally (for tests)
        }
    }
}
```

#### 3. manager.rs - Add renderer task, simplify main loop

```rust
pub async fn start(&mut self, mut rx: mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
    // Create segment channel
    let (segment_tx, mut segment_rx) = tokio::sync::mpsc::unbounded_channel();

    // Set sender on provider
    self.provider.lock().await.set_segment_sender(segment_tx);

    // Spawn RENDERER task - reads from channel, emits events
    let transcript = self.transcript.clone();
    let status = self.status.clone();
    let app_handle_renderer = self.app_handle.clone();
    tokio::spawn(async move {
        while let Some(segment) = segment_rx.recv().await {
            // Accumulate transcript
            transcript.lock().await.push(segment.clone());
            // Emit to frontend - this runs on a DIFFERENT thread from block_in_place
            let _ = app_handle_renderer.emit("transcription-update", &segment);
        }

        // Channel closed = transcription done
        let final_transcript = transcript.lock().await.clone();
        let _ = app_handle_renderer.emit("transcription-stopped",
            json!({ "transcript": final_transcript }));
        *status.lock().await = TranscriptionStatus::Idle;
    });

    // Spawn MAIN LOOP task - feeds audio to provider
    let provider = self.provider.clone();
    tokio::spawn(async move {
        let mut audio_buffer = AudioBuffer::new(3.0, 0.5, 16000);

        loop {
            tokio::select! {
                biased;
                _ = stop_rx.changed() => { break; }
                chunk_opt = rx.recv() => {
                    match chunk_opt {
                        Some(chunk) => {
                            audio_buffer.push(&chunk);
                            while let Some(audio) = audio_buffer.extract_window() {
                                // transcribe() sends results through channel internally
                                // renderer picks them up on another thread
                                let _ = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        let _ = provider.lock().await.transcribe(&audio);
                                    })
                                });
                            }
                        }
                        None => { break; }
                    }
                }
            }
        }

        // Drain remaining audio
        if let Some(audio) = audio_buffer.drain_remaining(1.0) {
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    provider.lock().await.transcribe(&audio)
                })
            });
        }

        // Close the channel by clearing the sender
        provider.lock().await.clear_segment_sender();
        // ^ This drops the sender, segment_rx.recv() returns None,
        //   renderer emits transcription-stopped
    });

    Ok(())
}
```

### What Stays The Same

| Component | Changes? | Notes |
|---|---|---|
| AudioRouter | No | Still sends PCM chunks to manager |
| AudioBuffer | No | Still windows audio into 3s chunks |
| SileroVad | No | Still gates pipeline |
| VoskProvider | No | Still produces partials |
| WhisperProvider | No | Still produces finals |
| TranscriptDisplay.tsx | No | `processTranscript()` already handles partial-to-final replacement |
| useRecording.ts | No | Already listens for `transcription-update` events |
| Frontend state/reducer | No | Already handles TRANSCRIPTION_UPDATE action |

### What Changes

| Component | Change | Why |
|---|---|---|
| provider.rs | Add `set_segment_sender` + `clear_segment_sender` to trait | Channel plumbing |
| hybrid_provider.rs | Store `segment_tx` instead of `app_handle`, send results through channel | Decouple production from emission |
| manager.rs | Spawn renderer task, simplify main loop, remove direct event emission | Renderer on separate thread enables real-time delivery |

### Timing Diagram

```
Time    block_in_place thread          Renderer thread           Frontend
----    ----------------------          ---------------           --------
T+0ms   VAD check starts
T+0.1ms Vosk starts
T+10ms  Vosk done, sends to channel
T+11ms  Whisper starts                 receives Vosk partial
T+12ms  |                              emit("transcription-update")
T+13ms  |                                                        renders grey text
T+15ms  | (Whisper processing...)
  ...   | (still processing...)                                  user sees grey text
T+1500ms Whisper done, sends to channel
T+1501ms                               receives Whisper final
T+1502ms                               emit("transcription-update")
T+1503ms                                                         replaces with black
```

The ~1.5s gap between Vosk and Whisper events gives the frontend ample time to render the partial.
