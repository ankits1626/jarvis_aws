# Tauri Architecture for JarvisApp

This guide explains Tauri's hybrid architecture and how JarvisApp uses it to build a cross-platform desktop application.

## What is Tauri?

Tauri is a framework for building desktop applications using web technologies (HTML/CSS/JavaScript) for the frontend and Rust for the backend. It provides:

- **Small bundle size** - No bundled Chromium (uses system webview)
- **Security** - Rust's memory safety + sandboxed frontend
- **Native performance** - Rust backend for system operations
- **Cross-platform** - macOS, Windows, Linux from one codebase

## Architecture Overview

```
┌─────────────────────────────────────┐
│   React Frontend (TypeScript)       │
│   - UI Components                   │
│   - State Management                │
│   - User Interactions               │
└──────────────┬──────────────────────┘
               │ invoke() / listen()
               │ (IPC Bridge)
┌──────────────▼──────────────────────┐
│   Tauri Backend (Rust)              │
│   - Commands (API endpoints)        │
│   - Events (push notifications)     │
│   - System Operations               │
│   - Process Management              │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│   Operating System                  │
│   - File System                     │
│   - Processes                       │
│   - Global Shortcuts                │
└─────────────────────────────────────┘
```

## Tauri Commands

Commands are Rust functions exposed to the frontend as async APIs. They're like REST endpoints but for desktop apps.

**Defining a command:**
```rust
// In commands.rs
use tauri::State;
use std::sync::Mutex;

#[tauri::command]
async fn start_recording(
    state: State<'_, Mutex<RecordingManager>>
) -> Result<String, String> {
    let mut manager = state.lock().unwrap();
    let recordings_dir = /* get directory */;
    manager.start_recording(&recordings_dir)
}
```

**Registering commands:**
```rust
// In lib.rs
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        commands::start_recording,
        commands::stop_recording,
        commands::list_recordings,
        commands::convert_to_wav,
        commands::delete_recording,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

**Calling from frontend:**
```typescript
// In React component
import { invoke } from '@tauri-apps/api/core';

async function handleStartRecording() {
  try {
    const filename = await invoke<string>('start_recording');
    console.log('Recording started:', filename);
  } catch (error) {
    console.error('Failed to start recording:', error);
  }
}
```

## Tauri Events

Events allow the backend to push notifications to the frontend asynchronously. This is crucial for real-time updates like recording status changes.

**Emitting events from Rust:**
```rust
use tauri::Emitter;

// In RecordingManager
pub fn start_recording(&mut self, recordings_dir: &Path) -> Result<String, String> {
    // ... spawn sidecar ...
    
    // Emit event to frontend
    self.app_handle.emit("recording-started", filename.clone())?;
    
    Ok(filename)
}
```

**Listening in frontend:**
```typescript
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
  const unlisten = listen('recording-started', (event) => {
    console.log('Recording started:', event.payload);
    dispatch({ type: 'RECORDING_STARTED', filename: event.payload });
  });
  
  return () => {
    unlisten.then(fn => fn());
  };
}, []);
```

## Plugins

Tauri plugins extend functionality with pre-built modules. JarvisApp uses:

**Shell Plugin** - For spawning processes:
```rust
// In lib.rs
tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    // ...
```

**Global Shortcut Plugin** - For keyboard shortcuts:
```rust
// In lib.rs
tauri::Builder::default()
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    // ...
```

**Usage example:**
```rust
use tauri_plugin_shell::ShellExt;

let sidecar = app_handle
    .shell()
    .sidecar("JarvisListen")?
    .args(["--mono", "--sample-rate", "16000"]);
```

## Sidecar Pattern

Sidecars are external binaries bundled with your app. JarvisApp bundles the JarvisListen CLI tool as a sidecar.

**Configuration in tauri.conf.json:**
```json
{
  "bundle": {
    "externalBin": [
      "binaries/JarvisListen"
    ]
  }
}
```

**Permissions in capabilities/default.json:**
```json
{
  "permissions": [
    {
      "identifier": "shell:allow-execute",
      "allow": [
        {
          "name": "binaries/JarvisListen",
          "sidecar": true
        }
      ]
    }
  ]
}
```

**Spawning the sidecar:**
```rust
use tauri_plugin_shell::ShellExt;

fn spawn_sidecar(&self, output_path: &Path) -> Result<(Receiver<CommandEvent>, CommandChild), String> {
    let sidecar = self.app_handle
        .shell()
        .sidecar("JarvisListen")
        .map_err(|e| format!("Failed to get sidecar: {}", e))?
        .args(["--mono", "--sample-rate", "16000", "--output", output_path.to_str().unwrap()]);
    
    let (rx, child) = sidecar.spawn()
        .map_err(|e| format!("Failed to spawn: {}", e))?;
    
    Ok((rx, child))
}
```

## Managed State

Tauri allows you to share state across commands using managed state.

**Setting up state:**
```rust
// In lib.rs
.setup(|app| {
    // Initialize and manage FileManager
    let file_manager = FileManager::new()?;
    app.manage(file_manager);
    
    // Initialize and manage RecordingManager (wrapped in Mutex for thread safety)
    let recording_manager = RecordingManager::new(app.handle().clone());
    app.manage(Mutex::new(recording_manager));
    
    Ok(())
})
```

**Accessing state in commands:**
```rust
#[tauri::command]
async fn list_recordings(
    state: State<'_, FileManager>
) -> Result<Vec<RecordingMetadata>, String> {
    state.list_recordings()
}

#[tauri::command]
async fn start_recording(
    state: State<'_, Mutex<RecordingManager>>
) -> Result<String, String> {
    let mut manager = state.lock().unwrap();
    // ... use manager ...
}
```

## Configuration Files

**tauri.conf.json** - Main configuration:
```json
{
  "productName": "JarvisApp",
  "identifier": "com.jarvis.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "JarvisApp",
        "width": 1000,
        "height": 700
      }
    ]
  }
}
```

**capabilities/default.json** - Security permissions:
```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-execute",
    "global-shortcut:allow-register"
  ]
}
```

## Development Workflow

```bash
# Install dependencies
npm install

# Run in development mode (hot reload)
npm run tauri dev

# Build for production
npm run tauri build

# Run Rust tests
cd src-tauri && cargo test
```

## Learn More

- [Tauri Documentation](https://tauri.app/v2/guides/) - Official guides
- [Tauri API Reference](https://tauri.app/v2/reference/) - Complete API docs
- [Tauri Examples](https://github.com/tauri-apps/tauri/tree/dev/examples) - Sample projects
