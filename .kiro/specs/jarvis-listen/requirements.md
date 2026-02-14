# Requirements Document

## Introduction

JarvisListen is a macOS command-line tool that captures audio from two simultaneous sources (system audio and microphone) and streams the combined output as raw PCM data to stdout. This tool serves as the "Listen" module in the JARVIS AI assistant pipeline (Listen → Transcribe → Augment → Display) for the AWS 10,000 AIdeas Competition. The tool enables capturing complete conversations from communication apps like Zoom, Google Meet, Slack, WhatsApp, Teams, and FaceTime by recording both the user's voice and other participants' audio.

## Glossary

- **System_Audio**: Audio output from all applications on the Mac (e.g., other participants' voices in video calls)
- **Microphone_Audio**: Audio input from the user's microphone device
- **PCM**: Pulse-Code Modulation, a digital representation of analog audio signals
- **SCStream**: ScreenCaptureKit's streaming API for capturing system audio and video
- **Ring_Buffer**: A circular buffer used to synchronize audio streams with different timing characteristics
- **Interleaving**: The process of combining two mono audio channels into a single stereo stream by alternating samples
- **Sample_Rate**: The number of audio samples per second (measured in Hz)
- **Audio_Chunk**: A fixed-duration segment of audio data (100ms in this system)
- **s16le**: 16-bit signed integer audio format, little-endian byte order

## Requirements

### Requirement 1: System Audio Capture

**User Story:** As a user, I want to capture all system audio output, so that I can record other participants' voices during video calls and meetings.

#### Acceptance Criteria

1. WHEN the tool starts, THE System SHALL use ScreenCaptureKit's SCStream with capturesAudio set to true
2. WHEN capturing system audio, THE System SHALL set excludesCurrentProcessAudio to true to prevent feedback loops
3. WHEN system audio is available, THE System SHALL capture audio from all running applications including Zoom, Google Meet, Slack, WhatsApp, Teams, and FaceTime
4. WHEN ScreenCaptureKit delivers audio in any format, THE System SHALL accept and process it correctly

### Requirement 2: Microphone Audio Capture

**User Story:** As a user, I want to capture my own voice from the microphone, so that both sides of the conversation are recorded.

#### Acceptance Criteria

1. WHEN the tool starts, THE System SHALL use ScreenCaptureKit's captureMicrophone set to true
2. WHEN no microphone device is specified, THE System SHALL use the system default microphone
3. WHEN the --mic-device flag is provided with a device ID, THE System SHALL use the specified microphone device
4. WHEN the --list-devices flag is provided, THE System SHALL print each device on a separate line to stdout showing the device ID and device name in the format: <id>: <name> (e.g., BuiltInMicrophoneDevice: MacBook Pro Microphone), and exit with code 0
5. WHEN a specified microphone device is not available, THE System SHALL print an error message to stderr and exit with a non-zero code
6. WHEN the active microphone device disconnects during capture, THE System SHALL fall back to the system default microphone and log a warning to stderr
7. WHEN the system audio output device changes during capture, THE System SHALL continue capturing from the new default output device

### Requirement 3: Audio Format Conversion

**User Story:** As a developer, I want all audio converted to a consistent format, so that downstream processing is simplified and predictable.

#### Acceptance Criteria

1. WHEN audio is received from either source, THE System SHALL convert it to 16kHz sample rate using AVAudioConverter
2. WHEN audio is received from either source, THE System SHALL convert it to 16-bit signed integer format (s16le)
3. WHEN audio is received from either source, THE System SHALL convert it to mono (single channel) before interleaving
4. WHEN the --sample-rate flag is provided, THE System SHALL validate that the value is one of: 8000, 16000, 24000, 44100, 48000. WHEN an unsupported sample rate is provided, THE System SHALL print an error to stderr listing valid values and exit with a non-zero code
5. WHEN audio conversion fails, THE System SHALL log an error to stderr and continue processing

### Requirement 4: Audio Stream Synchronization

**User Story:** As a developer, I want the two audio streams synchronized, so that the output maintains temporal alignment between microphone and system audio.

#### Acceptance Criteria

1. WHEN receiving audio from both sources, THE System SHALL use ring buffers to store incoming audio data
2. WHEN receiving audio from both sources, THE System SHALL use ring buffers capable of holding at least 2 seconds of audio data
3. WHEN a ring buffer overflows, THE System SHALL log a warning to stderr and discard the oldest data
4. WHEN one audio source has no data available, THE System SHALL fill that channel with zero bytes (silence)
5. WHEN both ring buffers have sufficient data, THE System SHALL read synchronized chunks for output

### Requirement 5: Stereo PCM Output

**User Story:** As a downstream consumer, I want to receive stereo PCM data on stdout, so that I can distinguish between microphone and system audio channels.

#### Acceptance Criteria

1. WHEN outputting audio, THE System SHALL write stereo PCM data to stdout
2. WHEN outputting stereo audio, THE System SHALL place microphone audio in channel 0 (left)
3. WHEN outputting stereo audio, THE System SHALL place system audio in channel 1 (right)
4. WHEN outputting audio, THE System SHALL use little-endian byte order
5. WHEN outputting audio, THE System SHALL write chunks of 100ms duration (6,400 bytes for 16kHz stereo s16le)
6. WHEN the --mono flag is provided, THE System SHALL mix both audio sources into a single mono channel instead of stereo output (3,200 bytes per 100ms chunk for 16kHz mono s16le)

### Requirement 6: Command-Line Interface

**User Story:** As a user, I want to control the tool's behavior through command-line flags, so that I can customize the capture configuration for different use cases.

#### Acceptance Criteria

1. WHEN the tool is invoked without arguments, THE System SHALL start capturing with default settings
2. WHEN the --mono flag is provided, THE System SHALL output mono audio instead of stereo
3. WHEN the --sample-rate flag is provided with a value N, THE System SHALL use N Hz as the output sample rate
4. WHEN the --mic-device flag is provided with a device ID, THE System SHALL use that microphone device
5. WHEN the --list-devices flag is provided, THE System SHALL list available microphones and exit
6. WHEN the --help flag is provided, THE System SHALL print usage information to stderr and exit with code 0
7. WHEN an invalid flag is provided, THE System SHALL print an error message to stderr and exit with a non-zero code

### Requirement 7: Signal Handling and Graceful Shutdown

**User Story:** As a user, I want the tool to shut down cleanly when interrupted, so that resources are properly released and buffers are flushed.

#### Acceptance Criteria

1. WHEN SIGINT is received (Ctrl+C), THE System SHALL stop the SCStream gracefully
2. WHEN SIGINT is received, THE System SHALL flush any remaining audio data in buffers to stdout
3. WHEN SIGINT is received, THE System SHALL exit with code 0
4. WHEN SIGTERM is received, THE System SHALL perform the same graceful shutdown as SIGINT (stop stream, flush buffers, exit 0)
5. WHEN SIGPIPE is received, THE System SHALL handle it silently and exit gracefully
6. WHEN shutting down, THE System SHALL release all ScreenCaptureKit resources properly

### Requirement 8: Error Handling and Logging

**User Story:** As a user, I want clear error messages when something goes wrong, so that I can diagnose and fix configuration issues.

#### Acceptance Criteria

1. WHEN ScreenCaptureKit permission is denied, THE System SHALL print a helpful error message to stderr explaining how to grant permission
2. WHEN any error occurs, THE System SHALL write error messages to stderr only (never to stdout)
3. WHEN logging informational messages, THE System SHALL write them to stderr only
4. WHEN an unrecoverable error occurs, THE System SHALL exit with a non-zero code
5. WHEN audio capture fails to start, THE System SHALL print a descriptive error message to stderr
6. WHEN capture starts successfully, THE System SHALL print a startup message to stderr showing: the microphone device name being used, the output format (sample rate, channels, bit depth), and the capture method

### Requirement 9: Permission Handling

**User Story:** As a user, I want clear guidance when permissions are missing, so that I can grant the necessary access to make the tool work.

#### Acceptance Criteria

1. WHEN Screen Recording permission is denied, THE System SHALL print an error message to stderr with instructions to enable it in System Settings > Privacy & Security > Screen & System Audio Recording, and exit with a non-zero code
2. WHEN Microphone permission is denied, THE System SHALL print a warning to stderr with instructions to enable it in System Settings > Privacy & Security > Microphone
3. WHEN Screen Recording permission is granted but Microphone permission is denied, THE System SHALL continue capturing system audio only and output silence (zero bytes) on channel 0 (microphone), logging a warning to stderr
4. WHEN both permissions are granted, THE System SHALL proceed with normal operation without printing permission-related messages

### Requirement 10: Platform and Dependency Requirements

**User Story:** As a developer, I want the tool to use only Apple frameworks, so that deployment is simple and there are no external dependencies to manage.

#### Acceptance Criteria

1. THE System SHALL require macOS 15.0 or later
2. THE System SHALL use only Apple-provided frameworks (ScreenCaptureKit, CoreMedia, AVFoundation, Foundation)
3. THE System SHALL NOT depend on any external libraries or packages
4. THE System SHALL be compiled for Apple Silicon (arm64) architecture
5. THE System SHALL use Swift 5.9 or later with async/await concurrency

### Requirement 11: Video Capture Minimization

**User Story:** As a developer, I want to minimize video capture overhead, so that the tool uses minimal system resources for audio-only capture.

#### Acceptance Criteria

1. WHEN configuring SCStream, THE System SHALL set video capture width to 2 pixels
2. WHEN configuring SCStream, THE System SHALL set video capture height to 2 pixels
3. WHEN configuring SCStream, THE System SHALL set the maximum frame interval to minimize video processing overhead
4. WHEN video frames are received, THE System SHALL discard them without processing
