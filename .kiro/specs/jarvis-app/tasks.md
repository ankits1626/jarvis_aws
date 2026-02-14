# Implementation Plan: JarvisApp Desktop Application

## Overview

This implementation plan breaks down the JarvisApp desktop application into discrete coding tasks. The application is built with Tauri v2 (Rust backend) and React/TypeScript (frontend), bundling the JarvisListen CLI tool as a sidecar binary for audio capture.

The implementation follows a layered approach:
1. Set up project structure and core Rust backend components
2. Implement file management and WAV conversion
3. Build recording lifecycle management with sidecar integration
4. Create React frontend with state management
5. Integrate frontend with backend commands and events
6. Add global shortcuts and platform-specific features
7. Implement comprehensive error handling and testing

## Tasks

- [x] 1. Initialize Tauri project structure
  - Create new Tauri v2 project with React template: `npm create tauri-app@latest -- --template react-ts`
  - Configure tauri.conf.json with bundle identifier, window settings, and sidecar binary path
  - Set up binaries/ directory with JarvisListen sidecar (macOS) and stub binaries (Windows/Linux)
  - Configure capabilities/default.json with shell and global-shortcut permissions
  - Add required dependencies to Cargo.toml (tauri-plugin-shell, tauri-plugin-global-shortcut, serde, tokio, nix)
  - Add required dependencies to package.json (@tauri-apps/api, @tauri-apps/plugin-shell)
  - _Requirements: 1.3, 1.4, 7.1, 7.2_

- [x] 2. Implement Rust backend core types and error handling
  - [x] 2.1 Create error.rs with AppError enum and Display implementation
    - Define error variants: SidecarSpawnFailed, SidecarCrashed, FileIOError, PermissionDenied, PlatformNotSupported, InvalidRecording, ConcurrentRecording
    - Implement Display trait for user-friendly error messages
    - _Requirements: 8.1, 8.2, 8.3_

  - [x] 2.2 Create types for RecordingMetadata and AppConfig in files.rs
    - Define RecordingMetadata struct with Serialize/Deserialize (filename, size_bytes, created_at, duration_seconds)
    - Define constants: SAMPLE_RATE=16000, BYTES_PER_SAMPLE=2, CHANNELS=1
    - _Requirements: 3.2, 3.3, 3.5_

- [x] 3. Implement FileManager for recording storage and listing
  - [x] 3.1 Create FileManager struct with recordings directory management
    - Implement new() to get platform-specific app data directory and create recordings subdirectory
    - Implement get_recordings_dir() to return recordings directory path
    - _Requirements: 1.1, 3.1_

  - [ ]* 3.2 Write unit test for recordings directory creation
    - Test that directory is created if missing on initialization
    - Test that existing directory is not overwritten
    - _Requirements: 1.1_

  - [x] 3.3 Implement duration calculation function
    - Create calculate_duration(size_bytes) function using formula: size_bytes / (SAMPLE_RATE * BYTES_PER_SAMPLE * CHANNELS)
    - _Requirements: 3.4_

  - [ ]* 3.4 Write property test for duration calculation
    - **Property 8: Duration Calculation Formula**
    - **Validates: Requirements 3.4**

  - [x] 3.5 Implement list_recordings() method
    - Read all .pcm files from recordings directory
    - Extract metadata: filename, size, creation timestamp, calculated duration
    - Sort by created_at descending (newest first)
    - Return Vec<RecordingMetadata>
    - _Requirements: 1.2, 3.5, 4.1_

  - [ ]* 3.6 Write property test for metadata completeness
    - **Property 9: Metadata Completeness**
    - **Validates: Requirements 3.5**

  - [ ]* 3.7 Write property test for recording sort order
    - **Property 10: Recording Sort Order**
    - **Validates: Requirements 4.1**

  - [x] 3.8 Implement delete_recording(filename) method
    - Validate filename (no path traversal)
    - Delete PCM file from recordings directory
    - Return descriptive error on failure
    - _Requirements: 6.3, 8.3_

  - [ ]* 3.9 Write unit tests for delete_recording
    - Test successful deletion
    - Test error when file doesn't exist
    - Test error for invalid filename (path traversal attempt)
    - _Requirements: 6.3, 6.5_

  - [ ]* 3.10 Write property test for file deletion and notification
    - **Property 18: File Deletion and Notification**
    - **Validates: Requirements 6.2, 6.3, 6.4**

- [x] 4. Implement WAV conversion for playback
  - [x] 4.1 Create WavConverter with pcm_to_wav function
    - Read PCM file from path
    - Generate 44-byte WAV header with correct parameters (16kHz, 16-bit, mono, PCM format)
    - Concatenate header + PCM data
    - Return Vec<u8>
    - _Requirements: 5.1_

  - [ ]* 4.2 Write unit test for WAV header generation
    - Test header structure with known PCM file
    - Verify RIFF, WAVE, fmt, data chunks
    - Verify sample rate, bits per sample, channels, format code
    - _Requirements: 5.1_

  - [ ]* 4.3 Write property test for WAV conversion
    - **Property 14: WAV Conversion and Return**
    - **Validates: Requirements 5.1, 5.2**

- [x] 5. Implement PlatformDetector for cross-platform support
  - [x] 5.1 Create PlatformDetector with platform detection methods
    - Implement is_supported() to return true only on macOS
    - Implement get_sidecar_name() to return platform-specific binary name
    - Implement open_system_settings() for macOS (x-apple.systempreferences URL)
    - Return error for open_system_settings() on non-macOS platforms
    - _Requirements: 7.1, 7.2, 7.3_

  - [ ]* 5.2 Write unit tests for platform detection
    - Test is_supported() returns correct value for current platform
    - Test get_sidecar_name() returns correct binary name
    - Test open_system_settings() on macOS (if running on macOS)
    - _Requirements: 7.1, 7.2, 7.3_

  - [ ]* 5.3 Write property test for platform error display
    - **Property 20: Platform Error Display**
    - **Validates: Requirements 7.5**

- [x] 6. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Implement RecordingManager for sidecar lifecycle
  - [x] 7.1 Create RecordingManager struct with state tracking
    - Define struct with current_child: Option<CommandChild>, current_filepath: Option<PathBuf>, app_handle: AppHandle
    - Implement new(app_handle) constructor
    - Implement is_recording() to check if recording is active
    - _Requirements: 2.6_

  - [x] 7.2 Implement generate_timestamped_path() method
    - Generate filename in format YYYYMMDD_HHMMSS.pcm using current timestamp
    - Return full path in recordings directory
    - _Requirements: 2.1, 3.2_

  - [ ]* 7.3 Write property test for timestamped filepath generation
    - **Property 2: Timestamped Filepath Generation**
    - **Validates: Requirements 2.1, 3.1, 3.2**

  - [x] 7.4 Implement spawn_sidecar() method
    - Use tauri_plugin_shell to get sidecar command
    - Add arguments: --mono, --sample-rate 16000, --output <filepath>
    - Spawn process and return (Receiver<CommandEvent>, CommandChild)
    - Return descriptive error on spawn failure
    - _Requirements: 2.2, 2.3_

  - [ ]* 7.5 Write property test for sidecar spawn arguments
    - **Property 3: Sidecar Spawn Arguments**
    - **Validates: Requirements 2.2**

  - [x] 7.6 Implement monitor_events() method
    - Spawn async task to listen to CommandEvent receiver
    - Parse stderr for permission errors (keywords: "permission", "Screen Recording", "Microphone")
    - Emit "permission-error" event for permission issues
    - Emit "sidecar-error" event for other stderr output
    - Emit "sidecar-crashed" event for non-zero exit codes
    - _Requirements: 7.4, 8.1, 8.2_

  - [ ]* 7.7 Write unit tests for stderr parsing and error classification
    - Test permission error detection from stderr
    - Test general error detection from stderr
    - Test crash detection from exit code
    - _Requirements: 7.4, 8.1, 8.2_

  - [x] 7.8 Implement start_recording() method
    - Check if already recording, return ConcurrentRecording error if true
    - Generate timestamped filepath
    - Spawn sidecar with --output flag
    - Store child process and filepath in state
    - Start monitoring events
    - Emit "recording-started" event with filename
    - Return filename on success
    - _Requirements: 2.1, 2.2, 2.6_

  - [ ]* 7.9 Write unit test for concurrent recording prevention
    - **Property 6: Concurrent Recording Prevention**
    - **Validates: Requirements 2.6**

  - [x] 7.10 Implement stop_recording() method
    - Check if recording is active, return error if not
    - Send SIGTERM to child process using nix crate: `kill(Pid::from_raw(child.pid() as i32), Signal::SIGTERM)`
    - Wait for process exit with timeout (5 seconds)
    - If timeout expires, fall back to child.kill() (SIGKILL) as last resort
    - Verify PCM file exists and has data
    - Clear state (current_child, current_filepath)
    - Emit "recording-stopped" event
    - Note: SIGTERM allows JarvisListen signal handlers to flush audio buffers before exit; SIGKILL would cause data loss
    - _Requirements: 2.4, 2.5_

  - [ ]* 7.11 Write property test for process termination
    - **Property 4: Process Termination on Stop**
    - **Validates: Requirements 2.4**

  - [ ]* 7.12 Write integration test for recording lifecycle
    - Test start → stop → verify file exists
    - Test start → crash → verify error event
    - Test concurrent recording attempt
    - _Requirements: 2.1, 2.2, 2.4, 2.5, 2.6_

- [x] 8. Implement ShortcutManager for global keyboard shortcuts
  - [x] 8.1 Create ShortcutManager struct with shortcut registration
    - Implement new(app_handle) constructor
    - Implement register_shortcuts() to register Cmd+Shift+R on macOS
    - On shortcut press, check recording state and emit "shortcut-triggered" event with "start" or "stop" action
    - Log warning and continue if registration fails (non-fatal)
    - _Requirements: (implicit global shortcut feature)_

  - [ ]* 8.2 Write unit test for shortcut registration
    - Test that registration is attempted
    - Test that failure is non-fatal
    - _Requirements: (implicit global shortcut feature)_

- [x] 9. Wire up Tauri commands in main.rs
  - [x] 9.1 Create command handlers in commands.rs
    - Implement start_recording(state: State<Mutex<RecordingManager>>) command
    - Implement stop_recording(state: State<Mutex<RecordingManager>>) command
    - Implement list_recordings(state: State<FileManager>) command
    - Implement convert_to_wav(filename: String, state: State<FileManager>) command
    - Implement delete_recording(filename: String, state: State<FileManager>) command
    - Implement check_platform_support() command
    - Implement open_system_settings() command
    - All commands return Result<T, String> for error handling
    - _Requirements: 1.5, 2.1, 2.4, 3.5, 5.1, 6.2, 7.3, 7.4_

  - [x] 9.2 Set up Tauri app in main.rs
    - Initialize FileManager and add to managed state
    - Initialize RecordingManager with AppHandle and add to managed state (wrapped in Mutex)
    - Initialize ShortcutManager and register shortcuts
    - Register all command handlers with invoke_handler
    - Add shell and global-shortcut plugins
    - _Requirements: 1.1, 1.3, 1.4_

  - [ ]* 9.3 Write unit tests for command handlers
    - Test each command with valid inputs
    - Test error cases (missing file, concurrent recording, etc.)
    - _Requirements: 2.6, 6.5, 8.4_

  - [ ]* 9.4 Write property test for sidecar binary verification
    - **Property 27: Sidecar Binary Verification**
    - **Validates: Requirements 1.3, 1.4, 1.5**

- [x] 10. Checkpoint - Backend complete, verify all Rust tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 11. Implement React frontend state management
  - [x] 11.1 Create TypeScript types in state/types.ts
    - Define RecordingState type: "idle" | "recording" | "processing"
    - Define AppState interface matching design
    - Define AppAction union type with all action variants
    - Define RecordingMetadata interface matching Rust type
    - Define event payload types (RecordingStartedEvent, ErrorEvent, ShortcutEvent)
    - _Requirements: 9.1_

  - [x] 11.2 Implement state reducer in state/reducer.ts
    - Implement appReducer function with all action cases
    - Handle state transitions: idle ↔ processing ↔ recording
    - Handle error states and recovery
    - Ensure atomic state updates (all fields updated together)
    - _Requirements: 9.1, 9.3, 9.4, 9.5_

  - [ ]* 11.3 Write unit tests for state reducer
    - Test all state transitions with specific actions
    - Test error handling and recovery
    - Test atomic updates
    - _Requirements: 9.1, 9.3, 9.4, 9.5_

  - [ ]* 11.4 Write property test for atomic state updates
    - **Property 24: Atomic State Updates**
    - **Validates: Requirements 9.3**

- [x] 12. Implement React custom hooks
  - [x] 12.1 Create hooks directory and useTauriCommand hook in hooks/useTauriCommand.ts
    - Create src/hooks/ directory
    - Accept command name and return [invokeFunction, { loading, error }]
    - Handle loading state during command execution
    - Handle errors and return error message
    - _Requirements: 9.2_

  - [x] 12.2 Create useTauriEvent hook in hooks/useTauriEvent.ts
    - Accept event name and handler function
    - Set up event listener on mount
    - Clean up listener on unmount
    - _Requirements: 9.2_

  - [x] 12.3 Create useRecording hook in hooks/useRecording.ts
    - Use useReducer with appReducer
    - Implement startRecording function calling Tauri command
    - Implement stopRecording function calling Tauri command
    - Implement selectRecording, deleteRecording, refreshRecordings functions
    - Implement openSystemSettings and retryRecording functions
    - Listen to all Tauri events (recording-started, recording-stopped, permission-error, sidecar-error, sidecar-crashed, shortcut-triggered)
    - Dispatch appropriate actions on events
    - _Requirements: 2.1, 2.4, 2.5, 2.8, 6.2, 8.1, 8.2, 8.4_

  - [ ]* 12.4 Write unit tests for custom hooks
    - Test useTauriCommand with mock commands
    - Test useTauriEvent with mock events
    - Test useRecording state transitions
    - _Requirements: 9.2_

  - [ ]* 12.5 Write property test for recordings list load on startup
    - **Property 1: Recordings List Load on Startup**
    - **Validates: Requirements 1.2**

- [x] 13. Implement utility functions
  - [x] 13.1 Create utils directory and formatters in utils/formatters.ts
    - Create src/utils/ directory
    - Implement formatDuration(seconds) to return MM:SS format
    - Implement formatFileSize(bytes) to return KB/MB format
    - Implement formatTimestamp(unixTimestamp) to return readable date/time
    - _Requirements: 4.2_

  - [ ]* 13.2 Write property test for duration formatting
    - **Property 11: Recording Display Fields**
    - **Validates: Requirements 4.2**

- [x] 14. Implement React UI components
  - [x] 14.1 Create components directory and RecordButton component
    - Create src/components/ directory
    - Accept state, onStart, onStop props
    - Display large circular button with icon
    - Show different states: idle (record icon), recording (stop icon, pulsing), processing (spinner)
    - Disable button during processing
    - _Requirements: 2.1, 2.4_

  - [x] 14.2 Create StatusIndicator component
    - Accept state and elapsedTime props
    - Display "Idle", "Recording...", or "Processing..." based on state
    - Show elapsed time counter during recording (updates every second)
    - _Requirements: 2.7_

  - [ ]* 14.3 Write property test for recording state UI display
    - **Property 7: Recording State UI Display**
    - **Validates: Requirements 2.7**

  - [x] 14.4 Create RecordingRow component
    - Accept recording, selected, onSelect, onDelete props
    - Display timestamp (formatted), duration (MM:SS), file size (KB/MB)
    - Highlight if selected
    - Show delete button on hover
    - _Requirements: 4.2_

  - [x] 14.5 Create RecordingsList component
    - Accept recordings, selectedRecording, onSelect, onDelete props
    - Map recordings to RecordingRow components
    - Display "No recordings yet" message if list is empty
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [ ]* 14.6 Write property test for empty recordings list message
    - **Property 28: Empty Recordings List Message**
    - **Validates: Requirements 4.4**

  - [x] 14.7 Create AudioPlayer component
    - Accept filename and onClose props
    - Call convert_to_wav command when filename changes
    - Create blob URL from WAV bytes
    - Render HTML5 audio element with controls (play/pause, seek bar)
    - Reset to beginning on playback completion (ended event)
    - Clean up blob URL on unmount
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

  - [ ]* 14.8 Write property test for blob URL creation
    - **Property 15: Blob URL Creation for Playback**
    - **Validates: Requirements 5.3**

  - [ ]* 14.9 Write unit test for playback reset
    - **Property 16: Playback Reset on Completion**
    - **Validates: Requirements 5.6**

  - [ ]* 14.10 Write property test for playback initiation
    - **Property 12: Playback Initiation**
    - **Validates: Requirements 4.3**

  - [x] 14.11 Create PermissionDialog component
    - Accept visible, message, onOpenSettings, onRetry, onClose props
    - Display modal dialog with permission error message
    - Show "Open System Settings" button calling onOpenSettings
    - Show "Retry" button calling onRetry
    - Show "Close" button calling onClose
    - _Requirements: 8.1_

  - [x] 14.12 Create ErrorToast component
    - Accept message and onClose props
    - Display toast notification with error message
    - Auto-dismiss after 5 seconds or on close button click
    - _Requirements: 8.4_

  - [x] 14.13 Create DeleteConfirmDialog component
    - Accept visible, recordingName, onConfirm, onCancel props
    - Display modal dialog asking for deletion confirmation
    - Show recording name in message
    - Show "Delete" and "Cancel" buttons
    - _Requirements: 6.1_

  - [ ]* 14.14 Write property test for deletion confirmation dialog
    - **Property 17: Deletion Confirmation Dialog**
    - **Validates: Requirements 6.1**

- [x] 15. Implement main App component
  - [x] 15.1 Update App.tsx with full integration
    - Use useRecording hook for state and actions
    - Use useEffect to load recordings on mount
    - Use useEffect to start timer interval when recording state is "recording"
    - Render RecordButton with state and handlers
    - Render StatusIndicator with state and elapsed time
    - Render RecordingsList with recordings and handlers
    - Render AudioPlayer when recording is selected
    - Render PermissionDialog when showPermissionDialog is true
    - Render ErrorToast when error is not null
    - Render DeleteConfirmDialog when delete is requested
    - _Requirements: 1.2, 2.1, 2.4, 2.7, 2.8, 4.1, 4.2, 4.3, 5.3, 6.1, 8.4_

  - [ ]* 15.2 Write integration tests for App component
    - Test recording lifecycle (start → stop → list refresh)
    - Test playback flow (select → convert → play)
    - Test deletion flow (request → confirm → delete → list refresh)
    - Test permission error flow (error → dialog → settings → retry)
    - Test shortcut triggering (event → state change)
    - _Requirements: 2.1, 2.4, 2.5, 2.8, 4.3, 5.1, 6.1, 6.2_

  - [ ]* 15.3 Write property test for list update after deletion
    - **Property 13: List Update After Deletion**
    - **Validates: Requirements 4.5**

- [x] 16. Write property tests for event handling
  - [ ]* 16.1 Write property test for recording completion notification
    - **Property 5: Recording Completion Notification**
    - **Validates: Requirements 2.5, 2.8**

  - [ ]* 16.2 Write property test for platform error propagation
    - **Property 19: Platform Error Propagation**
    - **Validates: Requirements 7.4**

  - [ ]* 16.3 Write property test for crash detection
    - **Property 21: Crash Detection and Notification**
    - **Validates: Requirements 8.2**

- [x] 17. Add comprehensive error handling
  - [x] 17.1 Implement error display logic in App component
    - Show PermissionDialog for permission errors with guidance
    - Show ErrorToast for general errors
    - Show inline error for concurrent recording attempts
    - Provide dismiss/close buttons for all errors
    - Provide "Open System Settings" and "Retry" for permission errors
    - _Requirements: 8.1, 8.4, 8.5, 9.4, 9.5_

  - [ ]* 17.2 Write property test for error display and state transition
    - **Property 23: Error Display and State Transition**
    - **Validates: Requirements 8.4, 9.4**

  - [ ]* 17.3 Write property test for error recovery
    - **Property 25: Error Recovery to Idle**
    - **Validates: Requirements 9.5**

  - [ ]* 17.4 Write property test for file I/O error messages
    - **Property 22: File I/O Error Messages**
    - **Validates: Requirements 8.3**

- [x] 18. Checkpoint - Frontend complete, verify all React tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 19. Create developer onboarding documentation
  - [x] 19.1 Create docs/101 directory and write rust-basics.md
    - Create docs/101/ directory structure
    - Cover ownership and borrowing with examples from the project
    - Explain structs and enums (RecordingManager, AppError)
    - Explain Result and Option types with command examples
    - Cover cargo basics (build, test, dependencies)
    - Keep to 1-2 pages, link to official Rust documentation
    - _Requirements: 10.1_

  - [x] 19.2 Write docs/101/tauri-architecture.md
    - Explain Tauri's hybrid architecture (Rust backend + web frontend)
    - Cover commands with examples from commands.rs
    - Cover events with examples from RecordingManager
    - Explain plugins (shell, global-shortcut) with usage examples
    - Explain sidecar pattern with JarvisListen example
    - Cover tauri.conf.json structure and capabilities
    - Keep to 1-2 pages, link to official Tauri documentation
    - _Requirements: 10.2_

  - [x] 19.3 Write docs/101/tauri-shell-plugin.md
    - Explain spawning sidecar processes with code from RecordingManager
    - Cover reading stdout and stderr with event monitoring example
    - Explain handling process events (CommandEvent)
    - Cover killing processes gracefully (SIGTERM)
    - Include complete spawn_sidecar and monitor_events examples
    - Keep to 1-2 pages, link to tauri-plugin-shell documentation
    - _Requirements: 10.3_

  - [x] 19.4 Write docs/101/react-state-patterns.md
    - Explain useReducer for complex state with appReducer example
    - Cover custom hooks for Tauri commands (useTauriCommand)
    - Explain event listening patterns (useTauriEvent)
    - Cover atomic state updates and why they matter
    - Include complete useRecording hook example
    - Keep to 1-2 pages, link to React documentation
    - _Requirements: 10.4_

- [x] 20. Final integration and polish
  - [x] 20.1 Add loading states and animations
    - Add spinner during processing state
    - Add pulsing animation for recording button
    - Add fade transitions for dialogs and toasts
    - Add skeleton loaders for recordings list

  - [x] 20.2 Add CSS styling
    - Style RecordButton as large circular button
    - Style RecordingsList with hover effects
    - Style AudioPlayer with custom controls
    - Style dialogs and toasts
    - Ensure responsive layout

  - [x] 20.3 Test end-to-end flows manually
    - Test complete recording lifecycle on macOS
    - Test permission error handling and recovery
    - Test playback with various recording lengths
    - Test deletion with confirmation
    - Test global shortcut (Cmd+Shift+R)
    - Test error scenarios (concurrent recording, missing file, etc.)

  - [x] 20.4 Verify all property tests pass with 100+ iterations
    - Run all property tests with --iterations 100 flag
    - Verify no flaky tests
    - Fix any failing properties

- [x] 21. Final checkpoint - All tests pass, ready for deployment
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at major milestones
- Property tests validate universal correctness properties with 100+ iterations
- Unit tests validate specific examples, edge cases, and error conditions
- The implementation follows a backend-first approach, then frontend, then integration
- Global shortcuts are a nice-to-have feature and failure to register is non-fatal
- Platform support is macOS-only for MVP; Windows and Linux use stub sidecars
