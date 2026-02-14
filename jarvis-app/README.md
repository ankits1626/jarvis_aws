# JarvisApp - Audio Recording Desktop Application

A Tauri v2 desktop application for recording system audio and microphone on macOS using the JarvisListen CLI tool.

## Features

- 🎙️ Record system audio + microphone simultaneously
- ⏺️ Simple one-button interface (Start/Stop)
- ⏱️ Real-time elapsed time display
- 🔔 Permission error handling with system settings integration
- ⌨️ Global keyboard shortcut (Cmd+Shift+R) to toggle recording

## Requirements

- macOS 15.0+ (for ScreenCaptureKit microphone capture)
- Apple Silicon (arm64) or Intel (x86_64)
- Node.js 18+ and npm
- Rust 1.70+

## Installation

```bash
cd jarvis-app
npm install
```

## Development

Run the app in development mode with hot-reload:

```bash
npm run tauri dev
```

This will:
1. Start the Vite dev server for the React frontend
2. Build and launch the Tauri app
3. Enable hot-reload for both frontend and backend changes

## Building for Production

Build the production app bundle:

```bash
npm run tauri build
```

The built app will be in `src-tauri/target/release/bundle/macos/`

## Usage

### First Time Setup

1. Launch the app
2. Click "Start Recording"
3. If prompted, grant Screen Recording and Microphone permissions:
   - Click "Open System Settings"
   - Enable permissions for JarvisApp
   - Return to the app and click "Start Recording" again

### Recording

- **Start Recording**: Click the "⏺ Start Recording" button or press `Cmd+Shift+R`
- **Stop Recording**: Click the "⏹ Stop Recording" button or press `Cmd+Shift+R` again
- **View Status**: The app displays the current state (Idle, Recording, Processing) and elapsed time

### Recordings Location

Recordings are stored as PCM files in:
```
~/Library/Application Support/com.jarvis.app/recordings/
```

Filename format: `YYYYMMDD_HHMMSS.pcm`

## Current Status

### ✅ Implemented (Minimal UI)
- Rust backend (100% complete)
  - FileManager (directory management, listing, deletion)
  - WavConverter (PCM to WAV conversion)
  - RecordingManager (sidecar lifecycle, start/stop)
  - PlatformDetector (macOS support detection)
  - ShortcutManager (global keyboard shortcuts)
  - All Tauri commands (7 commands)
- React frontend (Minimal UI)
  - State management (reducer pattern)
  - Record button with state transitions
  - Status display with elapsed time
  - Error handling with permission dialog
  - Event listeners for backend events

### 🚧 Not Yet Implemented
- Recordings list display
- Audio playback
- Recording deletion UI
- Recordings list refresh
- Additional UI polish and animations

## Architecture

### Backend (Rust)
- **Tauri v2**: Desktop app framework
- **JarvisListen**: Bundled sidecar binary for audio capture
- **FileManager**: Manages recordings directory and file operations
- **RecordingManager**: Controls sidecar process lifecycle
- **WavConverter**: Converts PCM to WAV for playback

### Frontend (React + TypeScript)
- **State Management**: useReducer pattern with atomic updates
- **Tauri API**: Command invocation and event listening
- **CSS**: Custom styling with animations

## Troubleshooting

### "Permission denied" error
- Open System Settings → Privacy & Security → Screen Recording
- Enable JarvisApp
- Restart the app

### Recording file is empty
- Ensure you have granted both Screen Recording and Microphone permissions
- Check that you're not already recording in another app

### App won't start
- Verify you're on macOS 15.0+
- Check that the JarvisListen binary exists in `src-tauri/binaries/`
- Run `cargo build` in `src-tauri/` to verify Rust compilation

## Development Notes

### Testing Backend
```bash
cd src-tauri
cargo test
```

### Checking for Errors
```bash
cd src-tauri
cargo check
cargo clippy
```

### Viewing Logs
Backend logs (Rust) appear in the terminal where you ran `npm run tauri dev`.

## License

MIT
