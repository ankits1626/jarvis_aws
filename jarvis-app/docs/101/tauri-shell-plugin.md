# Tauri Shell Plugin for JarvisApp

This guide explains how JarvisApp uses the Tauri shell plugin to spawn and manage the JarvisListen sidecar process.

## What is the Shell Plugin?

The shell plugin provides APIs for:
- Spawning external processes (including sidecar binaries)
- Reading stdout and stderr streams
- Monitoring process lifecycle events
- Terminating processes gracefully or forcefully

## Spawning a Sidecar Process

The sidecar pattern allows you to bundle external executables with your Tauri app. Here's how JarvisApp spawns JarvisListen:

```rust
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tokio::sync::mpsc::Receiver;

fn spawn_sidecar(
    &self,
    output_path: &Path,
) -> Result<(Receiver<CommandEvent>, CommandChild), String> {
    // Get the sidecar command from the shell plugin
    let sidecar = self.app_handle
        .shell()
        .sidecar("JarvisListen")
        .map_err(|e| format!("Failed to get sidecar command: {}", e))?;
    
    // Add command-line arguments
    let sidecar_with_args = sidecar.args([
        "--mono",
        "--sample-rate",
        "16000",
        "--output",
        output_path.to_str().unwrap(),
    ]);
    
    // Spawn the process
    let (rx, child) = sidecar_with_args
        .spawn()
        .map_err(|e| format!("Failed to spawn sidecar process: {}", e))?;
    
    Ok((rx, child))
}
```

**Key points:**
- `shell().sidecar("name")` - Gets a command builder for the named sidecar
- `.args([...])` - Adds command-line arguments
- `.spawn()` - Spawns the process and returns a receiver for events and a child handle

## Reading Stdout and Stderr

The `Receiver<CommandEvent>` allows you to monitor the process output asynchronously:

```rust
use tauri_plugin_shell::process::CommandEvent;

fn monitor_events(&self, mut rx: Receiver<CommandEvent>) {
    let app_handle = self.app_handle.clone();
    
    // Spawn async task to monitor events
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(data) => {
                    // Handle stdout data (Vec<u8>)
                    let output = String::from_utf8_lossy(&data);
                    println!("Stdout: {}", output);
                }
                CommandEvent::Stderr(data) => {
                    // Handle stderr data (Vec<u8>)
                    let error = String::from_utf8_lossy(&data);
                    eprintln!("Stderr: {}", error);
                    
                    // Parse stderr for specific errors
                    if error.contains("permission") {
                        app_handle.emit("permission-error", error.to_string()).ok();
                    }
                }
                CommandEvent::Terminated(payload) => {
                    // Handle process termination
                    println!("Process exited with code: {:?}", payload.code);
                }
                CommandEvent::Error(error) => {
                    // Handle spawn/execution errors
                    eprintln!("Process error: {}", error);
                }
            }
        }
    });
}
```

## Handling Process Events

JarvisApp uses event monitoring to detect errors and crashes in real-time:

```rust
fn monitor_events(&self, mut rx: Receiver<CommandEvent>) {
    let app_handle = self.app_handle.clone();
    
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line).to_string();
                    let line_lower = line_str.to_lowercase();
                    
                    // Classify errors by parsing stderr
                    if line_lower.contains("permission")
                        || line_lower.contains("screen recording")
                        || line_lower.contains("microphone")
                    {
                        // Permission error - show dialog with guidance
                        app_handle.emit("permission-error", line_str).ok();
                    } else {
                        // General error - show toast
                        app_handle.emit("sidecar-error", line_str).ok();
                    }
                }
                CommandEvent::Terminated(payload) => {
                    // Check for crashes (non-zero exit code)
                    if payload.code != Some(0) {
                        app_handle.emit("sidecar-crashed", payload.code).ok();
                    }
                }
                _ => {}
            }
        }
    });
}
```

## Killing Processes Gracefully

JarvisApp needs to stop recording gracefully to avoid data loss. The JarvisListen CLI has signal handlers that flush audio buffers before exit.

**Problem:** Tauri's `CommandChild::kill()` sends SIGKILL on Unix, which cannot be caught by signal handlers.

**Solution:** Use the `nix` crate to send SIGTERM first, then fall back to SIGKILL if needed:

```rust
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::time::{Duration, Instant};

pub fn stop_recording(&mut self) -> Result<(), String> {
    // Get the child process
    let child = self.current_child.take()
        .ok_or("No recording in progress")?;
    
    let pid = child.pid();
    
    // Send SIGTERM to allow signal handlers to flush buffers
    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;
    
    // Wait for graceful exit with timeout (5 seconds)
    let timeout = Duration::from_secs(5);
    let start = Instant::now();
    
    // Poll for process exit
    while start.elapsed() < timeout {
        // Check if process has exited by sending null signal
        if kill(Pid::from_raw(pid as i32), None).is_err() {
            // Process has exited
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    
    // Timeout expired - force kill with SIGKILL as last resort
    eprintln!("Warning: Process didn't exit gracefully, sending SIGKILL");
    child.kill()
        .map_err(|e| format!("Failed to kill process: {}", e))?;
    
    Ok(())
}
```

**Why this matters:**
- SIGTERM (15) - Can be caught by signal handlers, allows cleanup
- SIGKILL (9) - Cannot be caught, immediate termination, potential data loss
- JarvisListen uses SIGTERM/SIGINT handlers to flush audio buffers
- Always try SIGTERM first, use SIGKILL only as last resort

## Complete Example: Recording Lifecycle

Here's the complete flow from start to stop:

```rust
pub struct RecordingManager {
    current_child: Option<CommandChild>,
    current_filepath: Option<PathBuf>,
    app_handle: AppHandle,
}

impl RecordingManager {
    pub fn start_recording(&mut self, recordings_dir: &Path) -> Result<String, String> {
        // 1. Generate output path
        let output_path = self.generate_timestamped_path(recordings_dir);
        let filename = output_path.file_name()
            .and_then(|s| s.to_str())
            .ok_or("Failed to extract filename")?
            .to_string();
        
        // 2. Spawn sidecar with --output flag
        let (rx, child) = self.spawn_sidecar(&output_path)?;
        
        // 3. Store state
        self.current_child = Some(child);
        self.current_filepath = Some(output_path);
        
        // 4. Start monitoring events
        self.monitor_events(rx);
        
        // 5. Notify frontend
        self.app_handle.emit("recording-started", filename.clone()).ok();
        
        Ok(filename)
    }
    
    pub fn stop_recording(&mut self) -> Result<(), String> {
        // 1. Get child process
        let child = self.current_child.take()
            .ok_or("No recording in progress")?;
        
        // 2. Send SIGTERM for graceful shutdown
        let pid = child.pid();
        kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?;
        
        // 3. Wait with timeout
        let timeout = Duration::from_secs(5);
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if kill(Pid::from_raw(pid as i32), None).is_err() {
                // Process exited
                self.app_handle.emit("recording-stopped", ()).ok();
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        
        // 4. Force kill if timeout
        child.kill()?;
        self.app_handle.emit("recording-stopped", ()).ok();
        
        Ok(())
    }
}
```

## Common Patterns

**Error handling:**
```rust
let sidecar = self.app_handle
    .shell()
    .sidecar("JarvisListen")
    .map_err(|e| format!("Failed to get sidecar: {}", e))?;
```

**Async event monitoring:**
```rust
tauri::async_runtime::spawn(async move {
    while let Some(event) = rx.recv().await {
        // Handle event
    }
});
```

**Process ID access:**
```rust
let pid = child.pid();  // Returns u32
```

## Learn More

- [tauri-plugin-shell Documentation](https://v2.tauri.app/plugin/shell/) - Official plugin docs
- [Tauri Process API](https://v2.tauri.app/reference/rust/tauri_plugin_shell/process/) - Rust API reference
- [Unix Signals](https://man7.org/linux/man-pages/man7/signal.7.html) - Understanding SIGTERM vs SIGKILL
