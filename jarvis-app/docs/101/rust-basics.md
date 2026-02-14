# Rust Basics for JarvisApp

This guide covers essential Rust concepts you'll encounter in the JarvisApp codebase. It's designed for developers new to Rust who want to understand and contribute to the project.

## Ownership and Borrowing

Rust's ownership system ensures memory safety without a garbage collector. Every value has a single owner, and when the owner goes out of scope, the value is dropped.

**Example from RecordingManager:**
```rust
pub fn start_recording(&mut self, recordings_dir: &std::path::Path) -> Result<String, String> {
    // Take ownership of current_child (moves it out of Option)
    let child = self.current_child.take()
        .ok_or("No recording in progress")?;
    
    // We now own 'child' and can use it
    let pid = child.pid();
    // ...
}
```

**Borrowing rules:**
- `&T` - Immutable reference (read-only, multiple allowed)
- `&mut T` - Mutable reference (read-write, only one at a time)

**Example:**
```rust
// Immutable borrow - can have many
pub fn is_recording(&self) -> bool {
    self.current_child.is_some()
}

// Mutable borrow - only one at a time
pub fn start_recording(&mut self, recordings_dir: &std::path::Path) -> Result<String, String> {
    self.current_child = Some(child);
    // ...
}
```

## Structs and Enums

**Structs** group related data together:

```rust
pub struct RecordingManager {
    current_child: Option<CommandChild>,
    current_filepath: Option<PathBuf>,
    app_handle: AppHandle,
}

impl RecordingManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            current_child: None,
            current_filepath: None,
            app_handle,
        }
    }
}
```

**Enums** represent a value that can be one of several variants:

```rust
#[derive(Debug)]
pub enum AppError {
    SidecarSpawnFailed(String),
    SidecarCrashed(String),
    FileIOError(String),
    PermissionDenied(String),
    PlatformNotSupported,
    ConcurrentRecording,
}
```

## Result and Option Types

Rust uses `Result<T, E>` for operations that can fail and `Option<T>` for values that might be absent.

**Option<T>:**
```rust
pub fn is_recording(&self) -> bool {
    // Option::is_some() returns true if Some, false if None
    self.current_child.is_some()
}

// Taking ownership from Option
let child = self.current_child.take()  // Returns Option<CommandChild>
    .ok_or("No recording in progress")?;  // Convert None to Err
```

**Result<T, E>:**
```rust
pub fn start_recording(&mut self, recordings_dir: &Path) -> Result<String, String> {
    // Check for concurrent recording
    if self.is_recording() {
        return Err("A recording is already in progress".to_string());
    }
    
    // Spawn sidecar - propagate errors with ?
    let (rx, child) = self.spawn_sidecar(&output_path)?;
    
    // Return success
    Ok(filename)
}
```

**The `?` operator:**
- Unwraps `Ok` values or returns early with `Err`
- Converts error types automatically (if `From` trait is implemented)

## Pattern Matching

Use `match` to handle different variants:

```rust
match event {
    CommandEvent::Stderr(line) => {
        let line_str = String::from_utf8_lossy(&line).to_string();
        // Handle stderr...
    }
    CommandEvent::Terminated(payload) => {
        if payload.code != Some(0) {
            // Handle crash...
        }
    }
    _ => {
        // Ignore other variants
    }
}
```

## Cargo Basics

**Common commands:**
```bash
# Build the project
cargo build

# Build for release (optimized)
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

**Dependencies in Cargo.toml:**
```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
nix = { version = "0.29", features = ["signal"] }
```

## Common Patterns in JarvisApp

**Mutex for shared state:**
```rust
// In lib.rs - wrap RecordingManager in Mutex for thread-safe access
app.manage(Mutex::new(recording_manager));

// In commands.rs - lock to access
#[tauri::command]
async fn start_recording(
    state: State<'_, Mutex<RecordingManager>>
) -> Result<String, String> {
    let mut manager = state.lock().unwrap();
    manager.start_recording(recordings_dir)
}
```

**Error propagation:**
```rust
// Convert errors to strings for Tauri commands
let sidecar = self.app_handle
    .shell()
    .sidecar("JarvisListen")
    .map_err(|e| format!("Failed to get sidecar: {}", e))?;
```

## Learn More

- [The Rust Book](https://doc.rust-lang.org/book/) - Comprehensive Rust guide
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - Learn through examples
- [Cargo Book](https://doc.rust-lang.org/cargo/) - Package manager documentation
