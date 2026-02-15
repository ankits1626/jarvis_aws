# FIFO (Named Pipe) Validation for Audio Streaming

## Verdict: Validated. FIFO is the right choice.

## Key Findings

### 1. Proven Pattern for Audio
Production software uses FIFOs for exactly this purpose:
- **Snapcast** — reads PCM from `/tmp/snapfifo` as primary audio input
- **PipeWire** — `pipe-tunnel` module creates source/sink backed by Unix pipes
- **MPD** — has native "fifo" audio output type for PCM streaming

### 2. Tauri stdout Corruption — Confirmed
Multiple GitHub issues confirm Tauri's shell plugin corrupts binary data:
- [Issue #4673](https://github.com/tauri-apps/tauri/issues/4673) — `String::from_utf8_lossy()` corrupts non-UTF-8 data
- [Issue #7684](https://github.com/tauri-apps/tauri/issues/7684) — Shell plugin drops lines with large output
- [Issue #7127](https://github.com/tauri-apps/tauri/issues/7127) — Feature request for raw binary IPC (still open)

### 3. macOS-Specific Details

| Parameter | macOS Value | Impact |
|-----------|-------------|--------|
| Default pipe buffer | **16KB** (expandable to 64KB) | Fine — 3200 bytes/100ms = only 5 writes before full |
| PIPE_BUF (atomic write) | **512 bytes** | 3200-byte writes NOT atomic, but fine for single-writer |
| F_SETPIPE_SZ | **Not supported** | Can't resize buffer, but kernel handles dynamically |
| PATH_MAX | **1024 bytes** | Keep FIFO paths short |

### 4. Throughput
Our use case: **32KB/s** (3200 bytes every 100ms).
FIFO throughput on Linux: 8+ Gbps for large writes.
We're using <0.001% of capacity. Not a concern.

### 5. SIGPIPE Handling — Already Covered
JarvisListen already ignores SIGPIPE (main.swift line 85):
```swift
Darwin.signal(SIGPIPE, SIG_IGN)
```
If the Rust reader crashes, sidecar gets `EPIPE` instead of terminating.

### 6. Race Condition Solution
FIFO `open()` blocks until both reader and writer are present:
1. Rust: `mkfifo()` + open for reading (blocks until writer connects)
2. Rust: spawn sidecar with FIFO path as `--output`
3. Sidecar: opens FIFO for writing (unblocks Rust reader)

Since Rust controls sidecar spawning, reader always opens before writer.

### 7. Tokio Support
`tokio::net::unix::pipe::Receiver` provides async FIFO reading:
```rust
let mut rx = pipe::OpenOptions::new()
    .open_receiver("/tmp/jarvis_audio.fifo")?;
let mut buf = vec![0u8; 3200];
rx.read_exact(&mut buf).await?;
```

**macOS caveat**: `read_write(true)` is Linux-only. Use read-only mode on macOS.

### 8. Backpressure
When FIFO buffer fills (reader slow), writer blocks automatically.
This is desirable — natural backpressure, no data loss, no unbounded memory growth.

## Recommended Implementation

```
Startup:
  1. Rust: mkfifo("/tmp/jarvis_audio_<session_id>.fifo")
  2. Rust: spawn tokio task → open FIFO for reading (blocks)
  3. Rust: spawn Swift sidecar with --output <fifo_path>
  4. Swift: opens FIFO for writing → unblocks Rust reader
  5. Audio flows: Swift → FIFO → Rust → file + channel

Shutdown:
  1. Rust: SIGTERM to sidecar
  2. Swift: flushes, closes FIFO
  3. Rust: reads EOF, cleans up
  4. Rust: unlink FIFO file
```

## Why Not Alternatives?

| Alternative | Why Not |
|------------|---------|
| **File tailing (poll)** | 200ms delay, disk I/O overhead |
| **stdout pipe** | Tauri corrupts binary data (splits on 0x0A) |
| **Unix domain socket** | More complex (accept, connect, framing) for no benefit |
| **Shared memory** | Way too complex, needs separate signaling |

## Sources
- [Tauri Issue #4673](https://github.com/tauri-apps/tauri/issues/4673)
- [Tauri Issue #7684](https://github.com/tauri-apps/tauri/issues/7684)
- [Snapcast](https://github.com/badaix/snapcast)
- [PipeWire pipe-tunnel](https://docs.pipewire.org/page_module_pipe_tunnel.html)
- [nix::unistd::mkfifo](https://docs.rs/nix/latest/nix/unistd/fn.mkfifo.html)
- [tokio::net::unix::pipe](https://docs.rs/tokio/latest/tokio/net/unix/pipe/)
- [XNU kernel pipe.h](https://github.com/apple/darwin-xnu/blob/2ff845c2e033bd0ff64b5b6aa6063a1f8f65aa32/bsd/sys/pipe.h)
- [IPC buffer sizes](https://www.netmeister.org/blog/ipcbufs.html)
