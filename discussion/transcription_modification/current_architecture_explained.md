# Current Transcription Architecture - First Principles

## The Big Picture

Jarvis records audio from your microphone and transcribes it to text in real-time. There are 7 components involved, forming a linear pipeline:

```
Microphone → JarvisListen → FIFO → AudioRouter → File + Channel → TranscriptionManager → Frontend
```

Let's understand each one.

---

## Component 1: JarvisListen (Sidecar Process)

**What**: A separate binary (not Rust, runs as a child process) that captures audio from the microphone.

**Where**: Spawned by `RecordingManager` in [recording.rs:164-194](jarvis-app/src-tauri/src/recording.rs#L164-L194)

**What it does**:
- Captures raw audio from the default microphone
- Outputs 16kHz, mono, s16le (16-bit signed integer, little-endian) PCM data
- Writes to whatever path is given via `--output <path>`

**Key detail**: It doesn't write to a regular file. It writes to a **FIFO** (named pipe). This is important - we'll explain why next.

**How it's spawned**:
```rust
sidecar.args(["--mono", "--sample-rate", "16000", "--output", fifo_path_str])
    .spawn()
```

---

## Component 2: FIFO (Named Pipe)

**What**: A special file on the filesystem that acts as a pipe between two processes.

**Where**: Created by `AudioRouter` in [audio_router.rs:29-48](jarvis-app/src-tauri/src/transcription/audio_router.rs#L29-L48)

**Why a FIFO and not a regular file?**

Without FIFO, the architecture would be:
```
JarvisListen → writes to file.pcm
                                    (later) Read file.pcm → transcribe
```
This doesn't work for **real-time** transcription. You'd have to wait for the recording to finish.

With FIFO:
```
JarvisListen → writes to FIFO ←→ AudioRouter reads from FIFO (in real-time)
```

A FIFO behaves like a pipe:
- JarvisListen writes bytes into one end
- AudioRouter reads bytes from the other end
- Data flows in real-time, byte by byte, as it's produced
- No file on disk grows - data passes through the pipe

**How it's created**:
```rust
let fifo_path = temp_dir().join(format!("jarvis_audio_{}.fifo", uuid));
nix::unistd::mkfifo(&fifo_path, Mode::S_IRUSR | Mode::S_IWUSR)
```

**Lifecycle**: Created before recording starts, deleted when AudioRouter is dropped.

---

## Component 3: AudioRouter

**What**: Reads from the FIFO and sends audio to two destinations simultaneously.

**Where**: [audio_router.rs](jarvis-app/src-tauri/src/transcription/audio_router.rs)

**What it does**:

```
FIFO (raw bytes from mic)
        |
        | reads 3200-byte chunks (100ms of audio each)
        |
        ├──→ Route 1: Write to recording file (for playback later)
        |
        └──→ Route 2: Send via mpsc channel (for live transcription)
```

**The two routes**:
1. **File**: Writes PCM bytes to `20240315_143022.pcm` on disk. This is the permanent recording.
2. **Channel**: Sends the same PCM bytes through a `tokio::sync::mpsc::Sender<Vec<u8>>`. The TranscriptionManager receives these on the other end.

**Why 3200 bytes per chunk?**
```
16000 samples/sec × 2 bytes/sample × 0.1 sec = 3200 bytes = 100ms of audio
```
So each chunk is exactly 100ms of audio.

**Threading**: Runs inside `spawn_blocking` because FIFO reads are blocking I/O (the read call waits until data is available).

```rust
// Simplified
loop {
    let n = fifo_reader.read(&mut buffer)?;  // blocks until data arrives
    if n == 0 { break; }                      // EOF = sidecar stopped
    recording_file.write_all(&buffer[..n])?;  // route 1: file
    tx.blocking_send(buffer[..n].to_vec())?;  // route 2: channel
}
```

---

## Component 4: mpsc Channel

**What**: A multi-producer, single-consumer async channel connecting AudioRouter to TranscriptionManager.

**Where**: Created in [recording.rs:388](jarvis-app/src-tauri/src/recording.rs#L388)

```rust
let (tx, rx) = mpsc::channel::<Vec<u8>>(1000);
```

**Buffer size**: 1000 chunks = 1000 × 3200 bytes = 100 seconds of audio buffering.

**Why so large?** Because Whisper transcription blocks for 1-2 seconds per 3-second window. If the channel were small, AudioRouter would get blocked waiting to send, which would block FIFO reads, which would block JarvisListen. The large buffer absorbs Whisper's latency spikes.

**Data flow**:
```
AudioRouter                             TranscriptionManager
    |                                           |
    tx.blocking_send(chunk) ──────────→ rx.recv().await
    |                                           |
    (sends Vec<u8> every 100ms)        (receives Vec<u8>)
```

---

## Component 5: TranscriptionManager

**What**: Orchestrates the transcription lifecycle. Receives raw audio chunks, accumulates them into windows, feeds them to the transcription engine, and emits results to the frontend.

**Where**: [manager.rs](jarvis-app/src-tauri/src/transcription/manager.rs)

**This is the most important component. Let's break it down.**

### 5a. AudioBuffer (Windowing)

Raw chunks arrive every 100ms (3200 bytes each). But transcription engines need larger chunks to work with - you can't transcribe 100ms of audio meaningfully. So the manager accumulates chunks into **3-second windows** with **0.5-second overlap**.

**Where**: [audio_buffer.rs](jarvis-app/src-tauri/src/transcription/audio_buffer.rs)

```
Time: ──────────────────────────────────────────────────────→

Chunks:  [100ms][100ms][100ms]...[100ms]  (30 chunks = 3 seconds)
                                    ↓
                        AudioBuffer accumulates
                                    ↓
Window 1: [═══════════════ 3 seconds ═══════════════]
                                         [overlap]
Window 2:                           [═══════════════ 3 seconds ═══════════════]
```

**Window math**:
```
window_size  = 3.0 sec × 16000 Hz × 2 bytes = 96,000 bytes
overlap      = 0.5 sec × 16000 Hz × 2 bytes = 16,000 bytes
advance      = 96,000 - 16,000 = 80,000 bytes (2.5 sec)
```

After extracting a window, the buffer keeps the last 0.5s (overlap) and discards the first 2.5s. The overlap ensures words at window boundaries aren't cut in half.

**Format conversion during extraction**:
```
Raw bytes (s16le) → i16 samples → f32 samples (range -1.0 to 1.0)
```
Transcription engines expect f32 audio.

### 5b. The Transcription Loop

This is the core loop inside `manager.rs`. Here's what happens step by step:

```rust
tokio::spawn(async move {
    loop {
        tokio::select! {
            // Priority 1: Check if we should stop
            _ = stop_rx.changed() => { break; }

            // Priority 2: Receive audio chunks from AudioRouter
            chunk = rx.recv() => {
                // Push chunk to AudioBuffer
                audio_buffer.push(&chunk);

                // Try to extract complete windows
                while let Some(audio_f32) = audio_buffer.extract_window() {
                    // THIS IS THE CRITICAL PART ↓
                    let result = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            provider.lock().await.transcribe(&audio_f32)
                        })
                    });

                    // Emit results to frontend
                    for segment in result {
                        app_handle.emit("transcription-update", &segment);
                    }
                }
            }
        }
    }
});
```

### 5c. Understanding `block_in_place`

This is the most confusing part, so let's unpack it.

**The problem**: `provider.transcribe()` is a **synchronous CPU-bound** function. Whisper inference takes 1-2 seconds of heavy CPU work. You can't `await` it - it just blocks the thread.

**Why not just call it directly?** Tokio's async runtime has a small number of worker threads (typically = CPU cores). If you block one of them for 2 seconds, you reduce the runtime's capacity. Block all of them and the entire app freezes.

**`block_in_place` solves this**: It tells tokio "I'm about to block this thread for a while. Please move any pending async tasks off this thread so they can still make progress on other threads." Then it blocks.

```rust
tokio::task::block_in_place(|| {
    // This closure blocks the current thread for 1-2 seconds
    // But tokio has moved other tasks to other threads, so they still run
    provider.lock().await.transcribe(&audio_f32)
})
```

**The `block_on` inside**: `block_in_place` takes a synchronous closure. But `provider.lock()` is async (it's a tokio Mutex). So we use `block_on` to run the async lock acquisition synchronously within the blocked thread.

```
block_in_place(|| {           // "I'll block this thread"
    block_on(async {          // "Run this async code synchronously"
        provider.lock()       // "Get exclusive access to the provider"
            .await            // (this await resolves almost instantly)
            .transcribe(audio) // "This blocks for 1-2 seconds"
    })
})
```

### 5d. After Transcription

When `transcribe()` returns, the manager:
1. Gets back a `Vec<TranscriptionSegment>`
2. Pushes each segment into `self.transcript` (accumulates the full transcript)
3. Emits a `"transcription-update"` event for each segment via `app_handle.emit()`

### 5e. Stop and Drain

When recording stops:
1. A `watch::channel` signal is sent (`stop_tx.send(true)`)
2. The loop breaks
3. Any remaining audio in the buffer (< 3 seconds) is drained and transcribed if it's >= 1 second
4. A `"transcription-stopped"` event is emitted with the complete transcript

---

## Component 6: HybridProvider (The Transcription Engine)

**What**: The actual speech-to-text engine. Combines three sub-engines in a pipeline.

**Where**: [hybrid_provider.rs](jarvis-app/src-tauri/src/transcription/hybrid_provider.rs)

**The three sub-engines**:

### 6a. Silero VAD (Voice Activity Detection)

**Purpose**: Skip silence. If nobody is speaking, don't waste CPU on transcription.

**How**: An ONNX neural network model (2.2MB). Processes 512 samples (32ms) at a time. Outputs a speech probability (0.0 to 1.0). If any chunk exceeds the threshold (default 0.5), speech is detected.

**Speed**: Sub-millisecond.

**Result**: `Some(true)` = speech, `Some(false)` = silence (skip), `None` = VAD unavailable (assume speech).

### 6b. Vosk (Fast Partial Transcription)

**Purpose**: Give the user instant feedback while waiting for Whisper.

**How**: Uses a small statistical language model. Processes audio as i16 samples. Returns "partial" text - a best guess based on what it's heard so far.

**Speed**: ~10ms for a 3-second window.

**Result**: `TranscriptionSegment { text: "hello wor", is_final: false }` (grey text in UI)

**Note**: Vosk is not very accurate. It's a rough approximation that gives the user something to read while waiting.

### 6c. Whisper (Accurate Final Transcription)

**Purpose**: Produce the real, accurate transcription.

**How**: A transformer neural network (whisper.cpp via whisper-rs). Processes audio as f32 samples. Uses Metal GPU acceleration on macOS.

**Speed**: 1-2 seconds for a 3-second window.

**Result**: `TranscriptionSegment { text: "Hello world", is_final: true }` (black text in UI)

### 6d. The Pipeline Inside `transcribe()`

When `HybridProvider.transcribe(audio)` is called:

```
Step 1: VAD Check (~0.1ms)
    └─ Speech detected? → Continue
    └─ Silence? → Return empty vec immediately (skip steps 2-3)
    └─ VAD unavailable? → Continue (assume speech)

Step 2: Vosk Partial (~10ms)
    └─ Convert f32 → i16
    └─ Feed to Vosk recognizer
    └─ Get partial text
    └─ Emit directly via app_handle (attempt to show before Whisper)
    └─ Reset Vosk for next window

Step 3: Whisper Final (~1-2 seconds)
    └─ Feed f32 audio to Whisper
    └─ Get accurate text with timestamps
    └─ Return as Vec<TranscriptionSegment>
```

**The critical observation**: Steps 2 and 3 run **sequentially** inside a single `transcribe()` call. The Vosk result from step 2 is emitted directly, and the Whisper result from step 3 is returned. But both happen within the same `block_in_place` block on the same thread.

---

## Component 7: Frontend (React)

**What**: Receives Tauri events and renders the transcript in the UI.

**Where**: Multiple files work together:

### 7a. Event Listening ([useTauriEvent.ts](jarvis-app/src/hooks/useTauriEvent.ts))

A thin wrapper around Tauri's `listen()` API:
```typescript
listen<T>(eventName, (event) => {
    handler(event.payload);
});
```

### 7b. Event Handling ([useRecording.ts](jarvis-app/src/hooks/useRecording.ts))

Listens for `"transcription-update"` events and dispatches to the reducer:
```typescript
useTauriEvent<TranscriptionSegment>("transcription-update",
    (payload) => {
        dispatch({ type: "TRANSCRIPTION_UPDATE", segment: payload });
    }
);
```

### 7c. State Management ([reducer.ts](jarvis-app/src/state/reducer.ts))

Appends each segment to the transcript array:
```typescript
case "TRANSCRIPTION_UPDATE":
    return {
        ...state,
        transcript: [...state.transcript, action.segment],
    };
```

**Important**: Each `TRANSCRIPTION_UPDATE` creates a new state object. React sees the new reference and re-renders.

### 7d. Rendering ([TranscriptDisplay.tsx](jarvis-app/src/components/TranscriptDisplay.tsx))

Before rendering, `processTranscript()` cleans up the segment list:

```typescript
function processTranscript(segments: TranscriptionSegment[]): TranscriptionSegment[] {
    const result = [];
    for (const segment of segments) {
        if (segment.is_final) {
            // Remove the last partial (same audio window)
            if (result.length > 0 && !result[result.length - 1].is_final) {
                result.pop();
            }
            result.push(segment);
        } else {
            result.push(segment);
        }
    }
    return result;
}
```

**What this does**: When a Whisper final (`is_final: true`) arrives, it removes the preceding Vosk partial (`is_final: false`). The partial was a rough guess; the final is the real answer.

**Styling**:
- Partials: `segment-partial` class (grey, italic)
- Finals: `segment-final` class (black, normal weight)

---

## End-to-End Timeline

Here's what happens from the moment you speak to seeing text:

```
T+0ms      You say "Hello world"
T+0ms      Microphone captures audio
T+0-100ms  JarvisListen writes PCM bytes to FIFO
T+100ms    AudioRouter reads 3200 bytes from FIFO
T+100ms    AudioRouter writes to recording file
T+100ms    AudioRouter sends chunk via mpsc channel
T+100ms    TranscriptionManager receives chunk, pushes to AudioBuffer
           ... (29 more chunks arrive over the next 2.9 seconds) ...
T+3000ms   AudioBuffer has 96,000 bytes (3 seconds). Extracts a window.
T+3000ms   TranscriptionManager calls block_in_place(transcribe)
T+3000ms     HybridProvider: VAD check → speech detected
T+3010ms     HybridProvider: Vosk → "hello wor" (partial, is_final=false)
T+3010ms     HybridProvider: emits "transcription-update" directly
T+3010ms     HybridProvider: Whisper starts processing...
T+4500ms     HybridProvider: Whisper → "Hello world" (final, is_final=true)
T+4500ms   block_in_place returns
T+4500ms   TranscriptionManager emits "transcription-update" for Whisper segment
T+4501ms   Frontend receives event(s), dispatches TRANSCRIPTION_UPDATE
T+4502ms   React re-renders TranscriptDisplay
```

**Total latency**: ~4.5 seconds from speech to text appearing (3s accumulation + 1.5s Whisper processing).

---

## The Vosk Visibility Problem

The intended behavior is:
```
T+3010ms  User sees grey "hello wor" (Vosk partial)
T+4500ms  Grey text replaced with black "Hello world" (Whisper final)
```

The actual behavior is:
```
T+4500ms  Both events arrive at nearly the same time
T+4500ms  React batches both updates into one render
T+4500ms  processTranscript() immediately replaces partial with final
T+4500ms  User only sees "Hello world" (never sees the partial)
```

**Why?**

The Vosk emit at T+3010ms happens **inside** `block_in_place`. Although `block_in_place` allows other tokio tasks to run on other threads, the Tauri event system and webview IPC appear to deliver both events in close succession when observed from the webview's perspective. React's batching then combines both state updates (`partial` then `final`) into a single render, and `processTranscript()` strips the partial before it's ever painted.

The partial exists in state for 0 milliseconds from the user's perspective.

---

## Glossary

| Term | Meaning |
|---|---|
| **PCM** | Pulse-Code Modulation - raw uncompressed audio samples |
| **s16le** | Signed 16-bit Little-Endian - each sample is 2 bytes, range -32768 to 32767 |
| **f32** | 32-bit float - each sample is 4 bytes, range -1.0 to 1.0 |
| **16kHz** | 16,000 samples per second (telephone quality, sufficient for speech) |
| **FIFO** | First In First Out - a named pipe on the filesystem for IPC |
| **mpsc** | Multi-Producer Single-Consumer channel (tokio async channel) |
| **VAD** | Voice Activity Detection - determines if audio contains speech |
| **Partial** | A rough, fast transcription guess (Vosk, `is_final: false`) |
| **Final** | An accurate transcription result (Whisper, `is_final: true`) |
| **Window** | A 3-second slice of audio extracted from the buffer for transcription |
| **Overlap** | 0.5 seconds of audio shared between consecutive windows |
| **block_in_place** | Tokio function that blocks current thread while letting other tasks run elsewhere |
| **block_on** | Runs an async future synchronously on the current thread |
| **IPC** | Inter-Process Communication - how Tauri backend talks to webview frontend |
