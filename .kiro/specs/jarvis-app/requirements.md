# Requirements Document

## Introduction

JarvisApp is a cross-platform desktop application built with Tauri v2, React, and TypeScript that provides a user interface for recording, managing, and playing back audio captured via the JarvisListen CLI tool. The application bundles JarvisListen as a sidecar binary and manages its lifecycle, storing recordings in platform-appropriate directories.

## Glossary

- **JarvisApp**: The Tauri desktop application
- **JarvisListen**: The CLI audio capture tool bundled as a sidecar binary
- **Sidecar**: A bundled executable managed by Tauri's shell plugin
- **Recording**: A timestamped PCM audio file captured from JarvisListen
- **App_Data_Directory**: Platform-specific application data storage location
- **PCM**: Pulse Code Modulation, raw audio format (16kHz, 16-bit signed integer, mono)
- **WAV**: Waveform Audio File Format with 44-byte header
- **Tauri_Backend**: Rust code handling system operations
- **React_Frontend**: TypeScript/React UI code

## Requirements

### Requirement 1: Application Initialization

**User Story:** As a user, I want the application to initialize properly on startup, so that I can begin using the recording features immediately.

#### Acceptance Criteria

1. WHEN the application starts, THE Tauri_Backend SHALL create the recordings directory in the App_Data_Directory if it does not exist
2. WHEN the application starts, THE React_Frontend SHALL load the list of existing recordings
3. WHEN the application starts on macOS, THE Tauri_Backend SHALL verify the JarvisListen sidecar binary is available
4. WHEN the application starts on Windows or Linux, THE Tauri_Backend SHALL verify the stub sidecar binary is available
5. IF the sidecar binary is missing, THEN THE Tauri_Backend SHALL return an initialization error to the React_Frontend

### Requirement 2: Recording Management

**User Story:** As a user, I want to start and stop audio recordings, so that I can capture conversations happening on my Mac.

#### Acceptance Criteria

1. WHEN a user clicks the record button while idle, THE Tauri_Backend SHALL generate a timestamped filepath in the recordings directory
2. WHEN a user clicks the record button while idle, THE Tauri_Backend SHALL spawn the JarvisListen sidecar process with --mono --sample-rate 16000 --output <filepath> flags
3. WHEN the JarvisListen sidecar is spawned with --output, THE JarvisListen sidecar SHALL write PCM data directly to the specified file path
4. WHEN a user clicks the stop button while recording, THE Tauri_Backend SHALL terminate the JarvisListen process gracefully
5. WHEN the JarvisListen process terminates, THE Tauri_Backend SHALL close the PCM file and notify the React_Frontend
6. IF a user attempts to start recording while already recording, THEN THE Tauri_Backend SHALL reject the request with a concurrent recording error
7. WHEN a recording is in progress, THE React_Frontend SHALL display a recording status indicator and elapsed time counter
8. WHEN a recording completes, THE React_Frontend SHALL refresh the recordings list

### Requirement 3: Recording Storage

**User Story:** As a user, I want my recordings stored in a standard location, so that I can find them later and they persist across application restarts.

#### Acceptance Criteria

1. THE Tauri_Backend SHALL store recordings in the platform-specific App_Data_Directory
2. THE Tauri_Backend SHALL name recording files using compact timestamp format (YYYYMMDD_HHMMSS.pcm)
3. THE Tauri_Backend SHALL store recordings as raw PCM data where sample_rate=16000, bytes_per_sample=2 (s16le), channels=1 (mono)
4. WHEN listing recordings, THE Tauri_Backend SHALL calculate duration from file size using the formula: duration_seconds = file_size_bytes / (sample_rate * bytes_per_sample * channels)
5. WHEN listing recordings, THE Tauri_Backend SHALL return metadata including filename, file size, creation date, and calculated duration

### Requirement 4: Recordings List Display

**User Story:** As a user, I want to see a list of my saved recordings, so that I can select one to play back.

#### Acceptance Criteria

1. THE React_Frontend SHALL display recordings sorted by date in descending order (newest first)
2. WHEN displaying a recording, THE React_Frontend SHALL show the timestamp, duration, and file size
3. WHEN a user taps a recording, THE React_Frontend SHALL initiate playback for that recording
4. WHEN the recordings list is empty, THE React_Frontend SHALL display a message indicating no recordings exist
5. WHEN a recording is deleted, THE React_Frontend SHALL remove it from the list immediately

### Requirement 5: Audio Playback

**User Story:** As a user, I want to play back my recordings, so that I can review captured conversations.

#### Acceptance Criteria

1. WHEN a user selects a recording for playback, THE Tauri_Backend SHALL convert the PCM file to WAV format by prepending a 44-byte WAV header
2. WHEN the WAV conversion completes, THE Tauri_Backend SHALL return the WAV data to the React_Frontend
3. WHEN the React_Frontend receives WAV data, THE React_Frontend SHALL create a blob URL and load it into an HTML5 audio element
4. THE React_Frontend SHALL provide play/pause controls for the audio element
5. THE React_Frontend SHALL provide a seek bar for navigating within the recording
6. WHEN playback completes, THE React_Frontend SHALL reset the audio player to the beginning

### Requirement 6: Recording Deletion

**User Story:** As a user, I want to delete recordings I no longer need, so that I can manage storage space.

#### Acceptance Criteria

1. WHEN a user requests to delete a recording, THE React_Frontend SHALL prompt for confirmation
2. WHEN the user confirms deletion, THE React_Frontend SHALL invoke the Tauri_Backend delete command
3. WHEN the Tauri_Backend receives a delete command, THE Tauri_Backend SHALL remove the PCM file from the recordings directory
4. WHEN deletion succeeds, THE Tauri_Backend SHALL notify the React_Frontend
5. IF deletion fails, THEN THE Tauri_Backend SHALL return an error to the React_Frontend

### Requirement 7: Cross-Platform Support

**User Story:** As a developer, I want the application to build for multiple platforms, so that users on different operating systems can use it.

#### Acceptance Criteria

1. THE Tauri_Backend SHALL bundle the JarvisListen-aarch64-apple-darwin sidecar for macOS builds
2. THE Tauri_Backend SHALL bundle stub sidecar binaries for Windows and Linux builds
3. WHEN a user attempts to record on Windows or Linux, THE stub sidecar SHALL log "not yet supported" to stderr
4. WHEN a stub sidecar runs, THE Tauri_Backend SHALL capture the stderr output and return it as an error to the React_Frontend
5. THE React_Frontend SHALL display platform-specific error messages when recording is not supported

### Requirement 8: Error Handling

**User Story:** As a user, I want clear error messages when something goes wrong, so that I understand what happened and can take corrective action.

#### Acceptance Criteria

1. WHEN the JarvisListen process fails to start, THE Tauri_Backend SHALL return a descriptive error to the React_Frontend
2. WHEN the JarvisListen process crashes during recording, THE Tauri_Backend SHALL detect the failure and notify the React_Frontend
3. WHEN file I/O operations fail, THE Tauri_Backend SHALL return descriptive errors including the file path and system error message
4. WHEN a Tauri command fails, THE React_Frontend SHALL display the error message to the user
5. WHEN a concurrent recording is attempted, THE React_Frontend SHALL display "A recording is already in progress"

### Requirement 9: UI State Management

**User Story:** As a developer, I want predictable state management, so that the UI remains consistent and bug-free.

#### Acceptance Criteria

1. THE React_Frontend SHALL use a reducer pattern for managing recording state (idle, recording, processing)
2. THE React_Frontend SHALL use custom hooks for integrating Tauri command invocations
3. WHEN a state transition occurs, THE React_Frontend SHALL update all dependent UI elements atomically
4. WHEN an error occurs, THE React_Frontend SHALL transition to an error state and display the error message
5. WHEN recovering from an error, THE React_Frontend SHALL allow the user to return to the idle state

### Requirement 10: Developer Onboarding

**User Story:** As a developer new to Rust and Tauri, I want concise learning resources, so that I can understand the codebase and contribute effectively.

#### Acceptance Criteria

1. THE project SHALL include a docs/101/rust-basics.md guide covering ownership, borrowing, structs, enums, Result/Option, and cargo basics
2. THE project SHALL include a docs/101/tauri-architecture.md guide covering Tauri's architecture, commands, events, plugins, sidecar pattern, and tauri.conf.json structure
3. THE project SHALL include a docs/101/tauri-shell-plugin.md guide covering the shell plugin, spawning processes, reading stdout/stderr, and killing processes
4. THE project SHALL include a docs/101/react-state-patterns.md guide covering useReducer for complex state and custom hooks for Tauri command integration
5. WHEN a guide is created, THE guide SHALL be concise (1-2 pages), include working code examples relevant to the project, and link to official documentation
