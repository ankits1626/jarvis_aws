# Design Document: JarvisListen

## Overview

JarvisListen is a macOS command-line audio capture tool that simultaneously captures system audio and microphone input, synchronizes them, and outputs a stereo PCM stream to stdout. The design follows a protocol-oriented architecture with clear separation between capture, conversion, synchronization, and output concerns.

The system operates as a pipeline:
1. **Capture**: ScreenCaptureKit (SCStream) captures both audio sources
2. **Convert**: AVAudioConverter resamples to target format (16kHz, s16le, mono per channel)
3. **Synchronize**: Ring buffers align timing between the two streams
4. **Interleave**: Combine mono channels into stereo (or mix to mono if --mono flag)
5. **Output**: Write PCM chunks to stdout

This design prioritizes simplicity, zero external dependencies, and Unix composability (stdout-based output).

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         main.swift                          │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Argument     │  │ Signal       │  │ Run Loop        │  │
│  │ Parser       │  │ Handler      │  │ (async/await)   │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              AudioCaptureProvider Protocol                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  func startCapture() async throws                    │  │
│  │  func stopCapture() async                            │  │
│  │  var audioDataStream: AsyncStream<AudioData>         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    AudioCapture.swift                       │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ScreenCaptureKit Integration                         │  │
│  │ • SCStream setup and lifecycle                       │  │
│  │ • SCStreamOutput delegate (audio + microphone)       │  │
│  │ • CMSampleBuffer extraction                          │  │
│  │ • Device enumeration and selection                   │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   PCMConverter.swift                        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Audio Format Conversion                              │  │
│  │ • AVAudioConverter (any format → 16kHz mono s16le)   │  │
│  │ • Ring buffer management (2-second capacity)         │  │
│  │ • Stereo interleaving (mic=L, system=R)              │  │
│  │ • Mono mixing (optional)                             │  │
│  │ • Chunk generation (100ms = 6,400 bytes stereo)      │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    FileHandle.standardOutput                │
│                    (stdout - PCM stream)                    │
└─────────────────────────────────────────────────────────────┘
```

### Component Descriptions

#### 1. main.swift
**Responsibilities:**
- Parse command-line arguments (--mono, --sample-rate, --mic-device, --list-devices, --help)
- Validate arguments and handle errors
- Set up signal handlers for SIGINT, SIGTERM, SIGPIPE
- Instantiate AudioCapture with configuration
- Run the async capture loop
- Handle graceful shutdown and buffer flushing

**Key APIs:**
- `CommandLine.arguments` for argument parsing
- `signal()` for SIGINT/SIGTERM/SIGPIPE handling
- `Task` for async/await execution
- `FileHandle.standardError` for logging

#### 2. AudioCaptureProvider Protocol
**Responsibilities:**
- Define the interface for audio capture backends
- Enable testability through protocol abstraction
- Allow future alternative implementations (e.g., mock for testing)

**Protocol Definition:**
```swift
protocol AudioCaptureProvider {
    func startCapture() async throws
    func stopCapture() async
    var audioDataStream: AsyncStream<AudioData> { get }
}
```

#### 3. AudioCapture.swift
**Responsibilities:**
- Discover available displays and audio devices using `SCShareableContent.current`
- Configure SCStream with audio capture settings
- Implement SCStreamOutput delegate to receive audio callbacks
- Extract CMSampleBuffer data from both `.audio` and `.microphone` types
- Handle device changes and fallback logic
- Emit AudioData events through AsyncStream

**Key APIs:**
- `SCShareableContent.current` - discover displays/apps
- `SCContentFilter` - define capture scope (display-based)
- `SCStreamConfiguration` - set capturesAudio, captureMicrophone, excludesCurrentProcessAudio
- `SCStream` - start/stop capture
- `SCStreamOutput` - delegate callbacks for audio data
- `CMSampleBuffer` - extract AudioBufferList

**Configuration:**
- `capturesAudio = true` (system audio)
- `captureMicrophone = true` (microphone)
- `excludesCurrentProcessAudio = true` (prevent feedback)
- `width = 2, height = 2` (minimal video overhead)
- `minimumFrameInterval = CMTime(value: 1, timescale: 1)` (1 FPS - maximum interval to minimize video processing)

#### 4. PCMConverter.swift
**Responsibilities:**
- Convert incoming audio to target format (16kHz, s16le, mono per channel)
- Manage two ring buffers (one for mic, one for system audio)
- Synchronize audio streams by reading aligned chunks
- Interleave mono channels into stereo PCM
- Mix to mono if --mono flag is set
- Generate 100ms chunks (6,400 bytes stereo or 3,200 bytes mono)

**Key APIs:**
- `AVAudioConverter` - resample and format conversion
- `AVAudioPCMBuffer` - buffer management
- `AVAudioFormat` - format specifications

**Ring Buffer Specifications:**
- Capacity: 2 seconds of audio at target sample rate
- For 16kHz: 2 * 16000 * 2 bytes = 64,000 bytes per buffer
- Overflow behavior: discard oldest data, log warning to stderr
- Underflow behavior: fill missing channel with zeros (silence)

## Data Models

### AudioData
Represents a chunk of audio from a single source (microphone or system audio).

```swift
struct AudioData {
    enum Source {
        case microphone
        case systemAudio
    }
    
    let source: Source
    let buffer: CMSampleBuffer
    let timestamp: CMTime
}
```

### CaptureConfiguration
Holds the runtime configuration for audio capture.

```swift
struct CaptureConfiguration {
    let sampleRate: Int           // 8000, 16000, 24000, 44100, or 48000
    let outputMono: Bool          // true if --mono flag set
    let microphoneDeviceID: String?  // nil = default device
    
    static let validSampleRates: Set<Int> = [8000, 16000, 24000, 44100, 48000]
    static let defaultSampleRate: Int = 16000
    static let chunkDurationMs: Int = 100
    
    func bytesPerChunk() -> Int {
        let samplesPerChunk = (sampleRate * chunkDurationMs) / 1000
        let bytesPerSample = 2  // s16le = 2 bytes
        let channels = outputMono ? 1 : 2
        return samplesPerChunk * bytesPerSample * channels
    }
}
```

### RingBuffer
A circular buffer for audio synchronization.

```swift
class RingBuffer {
    private var buffer: [UInt8]
    private var readIndex: Int
    private var writeIndex: Int
    private var availableBytes: Int
    private let capacity: Int
    private var lock: os_unfair_lock_s  // More performant than NSLock for high-frequency audio
    
    init(capacity: Int)
    func write(_ data: [UInt8]) -> Bool  // returns false on overflow
    func read(_ count: Int) -> [UInt8]?  // returns nil if insufficient data
    func availableData() -> Int
    func clear()
    
    // Lock/unlock helpers
    private func withLock<T>(_ body: () -> T) -> T {
        os_unfair_lock_lock(&lock)
        defer { os_unfair_lock_unlock(&lock) }
        return body()
    }
}
```

### AudioDevice
Represents an audio input device.

```swift
struct AudioDevice {
    let id: String
    let name: String
}
```

## Components and Interfaces

### ArgumentParser
A simple struct to parse and validate command-line arguments.

```swift
struct ArgumentParser {
    enum ParseError: Error {
        case invalidFlag(String)
        case invalidSampleRate(String)
        case missingSampleRateValue
        case missingMicDeviceValue
    }
    
    static func parse(_ arguments: [String]) throws -> ParsedArguments
}

struct ParsedArguments {
    enum Action {
        case capture(CaptureConfiguration)
        case listDevices
        case showHelp
    }
    
    let action: Action
}
```

### SignalHandler
Manages Unix signal handling for graceful shutdown.

```swift
class SignalHandler {
    private var shutdownHandler: (() -> Void)?
    
    func setup(onShutdown: @escaping () -> Void)
    func handleSIGINT()
    func handleSIGTERM()
    func handleSIGPIPE()
}
```

### AudioCapture (Implementation)
Concrete implementation of AudioCaptureProvider using ScreenCaptureKit.

**Architecture Note:** SCStreamOutput is an NSObjectProtocol-based delegate that cannot be directly implemented by actors. We use a separate NSObject subclass (StreamDelegate) to receive callbacks on ScreenCaptureKit's dispatch queue, then forward data to the actor via AsyncStream.

**AsyncStream Pattern:** Use `AsyncStream.makeStream(of:)` (Swift 5.9+) in init to avoid race conditions. The stream and continuation are created once, not on each property access.

```swift
actor AudioCapture: AudioCaptureProvider {
    private let configuration: CaptureConfiguration
    private var stream: SCStream?
    private var streamDelegate: StreamDelegate?
    
    let audioDataStream: AsyncStream<AudioData>
    private let continuation: AsyncStream<AudioData>.Continuation
    
    init(configuration: CaptureConfiguration) {
        self.configuration = configuration
        
        // Create AsyncStream once in init (Swift 5.9+)
        let (stream, continuation) = AsyncStream.makeStream(of: AudioData.self)
        self.audioDataStream = stream
        self.continuation = continuation
        
        // Create delegate with continuation
        self.streamDelegate = StreamDelegate(continuation: continuation)
    }
    
    func startCapture() async throws
    func stopCapture() async
    func listDevices() async throws -> [AudioDevice]
}

// Separate delegate class to bridge SCStreamOutput to AsyncStream
class StreamDelegate: NSObject, SCStreamOutput {
    private let continuation: AsyncStream<AudioData>.Continuation
    
    init(continuation: AsyncStream<AudioData>.Continuation) {
        self.continuation = continuation
        super.init()
    }
    
    func stream(_ stream: SCStream, 
                didOutputSampleBuffer sampleBuffer: CMSampleBuffer, 
                of type: SCStreamOutputType) {
        // Handle .audio and .microphone types
        let source: AudioData.Source = (type == .microphone) ? .microphone : .systemAudio
        let timestamp = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
        let audioData = AudioData(source: source, buffer: sampleBuffer, timestamp: timestamp)
        continuation.yield(audioData)
    }
}
```

### PCMConverter
Handles audio format conversion, synchronization, and output.

```swift
class PCMConverter {
    private let configuration: CaptureConfiguration
    private let micBuffer: RingBuffer
    private let systemBuffer: RingBuffer
    private var micConverter: AVAudioConverter?
    private var systemConverter: AVAudioConverter?
    
    init(configuration: CaptureConfiguration)
    
    func process(_ audioData: AudioData) throws
    func generateChunk() -> [UInt8]?
    func flush() -> [UInt8]
    
    private func convert(_ sampleBuffer: CMSampleBuffer) throws -> [UInt8]
    private func interleave(mic: [UInt8], system: [UInt8]) -> [UInt8]
    private func mix(mic: [UInt8], system: [UInt8]) -> [UInt8]
}
```

## Algorithms

### Audio Capture Flow

```
1. Initialize SCStream with configuration
   - Get SCShareableContent.current
   - Select first display for content filter
   - Create SCStreamConfiguration:
     * capturesAudio = true
     * captureMicrophone = true
     * excludesCurrentProcessAudio = true
     * width = 2, height = 2
     * minimumFrameInterval = 1 second
   - If microphoneDeviceID specified, set it in configuration
   
2. Start SCStream
   - Add StreamDelegate as SCStreamOutput delegate
   - Call stream.startCapture()
   - If successful, print startup message to stderr:
     * Format: "Capturing: mic=<device name>, format=<rate>Hz s16le <stereo|mono>, method=ScreenCaptureKit"
     * Example: "Capturing: mic=MacBook Pro Microphone, format=16000Hz s16le stereo, method=ScreenCaptureKit"
   
3. Receive audio callbacks
   - stream(_:didOutputSampleBuffer:of:)
   - Check type: .audio or .microphone
   - Extract CMSampleBuffer
   - Create AudioData with source, buffer, timestamp
   - Emit through AsyncStream continuation
   
4. Handle device changes
   - Device disconnection detection options:
     * Monitor SCStream error callbacks (stream(_:didStopWithError:))
     * Use CoreAudio's AudioObjectAddPropertyListenerBlock on kAudioHardwarePropertyDevices
     * Detect absence of microphone data (no .microphone callbacks for extended period)
   - On microphone disconnect:
     * Log warning to stderr: "Warning: Microphone device '<id>' disconnected. Falling back to default device."
     * Reconfigure SCStream with nil microphoneDeviceID (system default)
     * Continue capturing with default device
   - On system audio device change:
     * ScreenCaptureKit automatically follows the default output device
     * No action required (continue capturing)
```

### Audio Conversion Algorithm

```
Input: CMSampleBuffer (any format)
Output: [UInt8] (16kHz, s16le, mono)

1. Extract AudioBufferList from CMSampleBuffer
   - Use CMSampleBufferGetAudioBufferList()
   
2. Create source AVAudioFormat from CMSampleBuffer
   - Use CMSampleBufferGetFormatDescription()
   - Extract sample rate, channels, format flags
   
3. Create target AVAudioFormat
   - Sample rate: configuration.sampleRate (default 16000)
   - Channels: 1 (mono)
   - Format: .pcmFormatInt16 (s16le)
   - Interleaved: true
   
4. Create AVAudioConverter
   - Source format → Target format
   - Cache converter for reuse
   
5. Convert audio
   - Create source AVAudioPCMBuffer from AudioBufferList
   - Create target AVAudioPCMBuffer with target format
   - Call converter.convert(to:error:withInputFrom:)
   
6. Extract bytes from target buffer
   - Access buffer.int16ChannelData
   - Copy to [UInt8] array
   - Return byte array
```

### Ring Buffer Synchronization Algorithm

```
Input: AudioData stream (microphone and system audio)
Output: Synchronized 100ms chunks

1. Initialize two ring buffers
   - micBuffer: capacity = 2 * sampleRate * 2 bytes
   - systemBuffer: capacity = 2 * sampleRate * 2 bytes
   
2. On receiving AudioData:
   - Convert CMSampleBuffer to [UInt8] (16kHz, s16le, mono)
   - If source == .microphone:
     * Write to micBuffer
     * If overflow: log warning, discard oldest
   - If source == .systemAudio:
     * Write to systemBuffer
     * If overflow: log warning, discard oldest
   
3. Generate chunk (called periodically via timer):
   - Timing mechanism: Use Task.sleep(for: .milliseconds(100)) in async loop
   - This provides consistent 100ms chunk output regardless of audio callback timing
   - Alternative: DispatchSourceTimer, but Task.sleep is simpler with async/await
   
   - Calculate chunk size: (sampleRate * 100ms) / 1000 * 2 bytes
   - For 16kHz: 16000 * 0.1 * 2 = 3,200 bytes per channel
   
   - Read micData from micBuffer (3,200 bytes)
     * If insufficient: fill with zeros (silence) for entire channel
   
   - Read systemData from systemBuffer (3,200 bytes)
     * If insufficient: fill with zeros (silence) for entire channel
   
   - If outputMono:
     * Mix micData and systemData (average samples)
     * Return 3,200 bytes
   - Else:
     * Interleave micData and systemData
     * Return 6,400 bytes (stereo)
```

### Stereo Interleaving Algorithm

```
Input: micData [UInt8] (3,200 bytes), systemData [UInt8] (3,200 bytes)
Output: stereo [UInt8] (6,400 bytes)

For each sample (2 bytes = 1 Int16):
  1. Read 2 bytes from micData → leftSample
  2. Read 2 bytes from systemData → rightSample
  3. Write leftSample to output (2 bytes)
  4. Write rightSample to output (2 bytes)

Result: L R L R L R ... (1,600 stereo frames)
```

### Mono Mixing Algorithm

```
Input: micData [UInt8] (3,200 bytes), systemData [UInt8] (3,200 bytes)
Output: mono [UInt8] (3,200 bytes)

For each sample (2 bytes = 1 Int16):
  1. Read 2 bytes from micData → micSample (Int16)
  2. Read 2 bytes from systemData → systemSample (Int16)
  3. mixedSample = (Int32(micSample) + Int32(systemSample)) / 2
  4. Clamp mixedSample to Int16 range [-32768, 32767]
  5. Write mixedSample to output (2 bytes)

Result: M M M M ... (1,600 mono samples)
```

### Signal Handling Algorithm

```
1. Setup (in main.swift):
   - Create SignalHandler instance
   - Register handlers for SIGINT, SIGTERM, SIGPIPE
   - Store reference to AudioCapture instance
   
2. On SIGINT or SIGTERM:
   - Set shutdown flag
   - Call audioCapture.stopCapture()
   - Call pcmConverter.flush()
   - Write flushed data to stdout
   - Exit with code 0
   
3. On SIGPIPE:
   - Handle silently (downstream consumer closed pipe)
   - Exit gracefully with code 0
   
4. Graceful shutdown sequence:
   - Stop SCStream
   - Wait for pending audio callbacks to complete
   - Flush ring buffers
   - Write remaining data to stdout
   - Release ScreenCaptureKit resources
```

## Error Handling

### Error Categories

#### 1. Permission Errors
**Screen Recording Permission Denied:**
- Error: ScreenCaptureKit throws permission error
- Handling: Print to stderr with instructions, exit with code 1
- Message: "Error: Screen Recording permission denied. Please enable it in System Settings > Privacy & Security > Screen & System Audio Recording"

**Microphone Permission Denied:**
- Error: ScreenCaptureKit microphone access denied
- Handling: Print warning to stderr, continue with system audio only
- Behavior: Output silence (zeros) on channel 0 (microphone)
- Message: "Warning: Microphone permission denied. Please enable it in System Settings > Privacy & Security > Microphone. Continuing with system audio only."

#### 2. Configuration Errors
**Invalid Sample Rate:**
- Error: --sample-rate value not in [8000, 16000, 24000, 44100, 48000]
- Handling: Print error to stderr, list valid values, exit with code 1
- Message: "Error: Invalid sample rate '<value>'. Valid values: 8000, 16000, 24000, 44100, 48000"

**Invalid Microphone Device:**
- Error: --mic-device ID not found in available devices
- Handling: Print error to stderr, exit with code 1
- Message: "Error: Microphone device '<id>' not found. Use --list-devices to see available devices."

**Invalid Flag:**
- Error: Unknown command-line flag
- Handling: Print error to stderr, suggest --help, exit with code 1
- Message: "Error: Unknown flag '<flag>'. Use --help for usage information."

#### 3. Runtime Errors
**Audio Conversion Failure:**
- Error: AVAudioConverter fails to convert audio
- Handling: Log error to stderr, skip the problematic buffer, continue processing
- Message: "Warning: Audio conversion failed for <source>: <error>. Skipping buffer."

**Ring Buffer Overflow:**
- Error: Ring buffer full, cannot write new data
- Handling: Discard oldest data, write new data, log warning to stderr
- Message: "Warning: Ring buffer overflow for <source>. Discarding oldest data."

**SCStream Start Failure:**
- Error: SCStream.startCapture() throws error
- Handling: Print error to stderr, exit with code 1
- Message: "Error: Failed to start audio capture: <error>"

**Device Disconnection:**
- Error: Active microphone device disconnects during capture
- Handling: Fall back to system default microphone, log warning to stderr, continue
- Message: "Warning: Microphone device '<id>' disconnected. Falling back to default device."

### Error Handling Strategy

1. **Fail Fast for Configuration Errors**: Invalid arguments or missing permissions should cause immediate exit with clear error messages.

2. **Graceful Degradation for Runtime Errors**: Audio conversion failures or buffer overflows should log warnings but not crash the application.

3. **Silent Fallback for Missing Data**: If one audio source is unavailable, output silence for that channel rather than failing.

4. **All Errors to stderr**: Never write errors or warnings to stdout (reserved for PCM data).

5. **Descriptive Error Messages**: Include actionable information (e.g., how to grant permissions, valid flag values).

6. **Exit Codes**:
   - 0: Successful execution or graceful shutdown
   - 1: Configuration error, permission denied, or unrecoverable runtime error


## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

After analyzing all acceptance criteria, I identified several areas where properties can be consolidated:

1. **Audio format conversion properties (3.1, 3.2, 3.3)** can be combined into a single comprehensive property that validates the complete conversion pipeline produces the correct output format.

2. **Stereo channel assignment properties (5.2, 5.3)** can be combined into a single property that validates correct interleaving of both channels.

3. **Logging destination properties (8.2, 8.3)** can be combined into a single property that validates all non-PCM output goes to stderr.

4. **Configuration validation properties** for sample rates (3.4) and device IDs (2.5) follow the same pattern and can use similar testing approaches.

5. **Many example tests** (configuration checks, specific signal handling) don't need properties—they're better suited as unit tests for specific scenarios.

### Core Properties

#### Property 1: Audio Format Conversion Completeness
*For any* audio input (regardless of source format, sample rate, or channel count), the conversion pipeline SHALL produce output in the target format: specified sample rate (default 16kHz), 16-bit signed integer (s16le), mono (single channel), little-endian byte order.

**Validates: Requirements 3.1, 3.2, 3.3, 5.4**

**Testing approach:** Generate random audio buffers with varying formats (8kHz-48kHz, different bit depths, mono/stereo/multi-channel). Convert each through the PCMConverter. Verify output format matches target specification by checking: sample rate, bit depth, channel count, and byte order.

#### Property 2: Ring Buffer Overflow Handling
*For any* ring buffer at or near capacity, when new data is written that exceeds capacity, the buffer SHALL discard the oldest data, accept the new data, and maintain its capacity invariant (availableBytes ≤ capacity).

**Validates: Requirements 4.3**

**Testing approach:** Generate random ring buffer states (varying fill levels). Write random data chunks that would cause overflow. Verify: (1) oldest data is discarded, (2) new data is written, (3) capacity is not exceeded, (4) availableBytes is correct.

#### Property 3: Silent Channel Filling
*For any* chunk generation request, when one ring buffer has insufficient data (underflow), the output SHALL contain zero bytes (silence) for that channel while the other channel contains actual audio data.

**Validates: Requirements 4.4**

**Testing approach:** Generate random buffer states where one buffer is empty or has insufficient data. Generate chunks. Verify the missing channel contains all zeros while the available channel contains non-zero data (or zeros if that's the actual audio).

#### Property 4: Stereo Interleaving Correctness
*For any* two mono audio buffers (microphone and system audio), the stereo interleaving process SHALL produce output where: (1) channel 0 (left) contains microphone samples, (2) channel 1 (right) contains system audio samples, (3) samples alternate L-R-L-R, (4) no samples are lost or duplicated.

**Validates: Requirements 5.2, 5.3**

**Testing approach:** Generate random mono audio buffers with known patterns (e.g., incrementing values). Interleave them. Extract left and right channels from output. Verify: (1) left channel matches microphone input, (2) right channel matches system audio input, (3) total sample count is preserved.

#### Property 5: Mono Mixing Correctness
*For any* two mono audio buffers (microphone and system audio), the mono mixing process SHALL produce output where each sample is the average of the corresponding microphone and system audio samples, clamped to Int16 range [-32768, 32767].

**Validates: Requirements 5.6**

**Testing approach:** Generate random mono audio buffers with known values. Mix them. Verify each output sample equals (mic[i] + sys[i]) / 2, properly clamped. Test edge cases: max values, min values, opposite signs.

#### Property 6: Chunk Size Consistency
*For any* valid configuration (sample rate, mono/stereo), the generated audio chunks SHALL have size equal to: (sampleRate × 0.1 seconds × 2 bytes × channelCount), where channelCount is 1 for mono or 2 for stereo.

**Validates: Requirements 5.5, 5.6**

**Testing approach:** Generate configurations with different sample rates (8000, 16000, 24000, 44100, 48000) and mono/stereo settings. Generate chunks for each. Verify chunk size matches formula. For 16kHz stereo: 16000 × 0.1 × 2 × 2 = 6,400 bytes. For 16kHz mono: 16000 × 0.1 × 2 × 1 = 3,200 bytes.

#### Property 7: Sample Rate Validation
*For any* integer value provided as --sample-rate argument, the argument parser SHALL accept it if and only if it is in the set {8000, 16000, 24000, 44100, 48000}, and SHALL reject all other values with an error message.

**Validates: Requirements 3.4**

**Testing approach:** Generate random integer values (including valid and invalid sample rates). Parse arguments with each value. Verify: (1) valid values are accepted, (2) invalid values are rejected with error, (3) error message lists valid values.

#### Property 8: Device ID Argument Parsing
*For any* string provided as --mic-device argument, the argument parser SHALL store that string in the configuration's microphoneDeviceID field without modification.

**Validates: Requirements 2.3, 6.4**

**Testing approach:** Generate random device ID strings (including valid IDs, invalid IDs, empty strings, special characters). Parse arguments with each. Verify the configuration contains the exact string provided.

#### Property 9: Device List Formatting
*For any* list of audio devices, the --list-devices output SHALL contain one line per device in the format "<id>: <name>", where <id> and <name> are the device's ID and name fields.

**Validates: Requirements 2.4, 6.5**

**Testing approach:** Generate random lists of AudioDevice objects with various IDs and names. Format them for output. Verify: (1) one line per device, (2) format matches "<id>: <name>", (3) all devices are included, (4) order is preserved.

#### Property 10: Error Output Destination
*For any* error or log message generated by the system, the output SHALL be written to stderr (FileHandle.standardError), never to stdout (FileHandle.standardOutput).

**Validates: Requirements 8.2, 8.3**

**Testing approach:** Trigger various error conditions and log events. Capture stdout and stderr separately. Verify: (1) stderr contains error/log messages, (2) stdout contains only PCM data (or is empty if no audio generated).

#### Property 11: Unrecoverable Error Exit Codes
*For any* unrecoverable error (permission denied, invalid configuration, capture start failure), the system SHALL exit with a non-zero exit code.

**Validates: Requirements 8.4**

**Testing approach:** Trigger various unrecoverable errors (mock permission denial, invalid sample rate, etc.). Capture exit codes. Verify all are non-zero (typically 1).

#### Property 12: Buffer Flush Completeness
*For any* ring buffer state at shutdown time, the flush operation SHALL return all available data in the buffer, leaving the buffer empty (availableBytes = 0).

**Validates: Requirements 7.2**

**Testing approach:** Generate random ring buffer states with varying amounts of data. Call flush on each. Verify: (1) returned data length equals initial availableBytes, (2) buffer is empty after flush, (3) data content matches what was in buffer.

#### Property 13: Invalid Flag Rejection
*For any* command-line argument that is not a recognized flag or value, the argument parser SHALL reject it with an error message and non-zero exit code.

**Validates: Requirements 6.7**

**Testing approach:** Generate random invalid flags (--invalid, --xyz, -z, etc.). Parse arguments with each. Verify: (1) parsing fails, (2) error message is generated, (3) error suggests using --help.

### Edge Case Properties

These properties focus on boundary conditions and special cases that are important to handle correctly:

#### Property 14: Empty Buffer Chunk Generation
*For any* chunk generation request when both ring buffers are empty, the output SHALL be a chunk of the correct size filled entirely with zero bytes.

**Validates: Requirements 4.4**

**Testing approach:** Create empty ring buffers. Generate chunk. Verify: (1) chunk size is correct for configuration, (2) all bytes are zero.

#### Property 15: Full Silence on Buffer Underflow
*For any* chunk generation request when a ring buffer contains less than a full chunk's worth of data, the output SHALL contain zero bytes (silence) for that entire channel.

**Validates: Requirements 4.4, 4.5**

**Testing approach:** Generate ring buffer states with partial data (e.g., 50% of chunk size). Generate chunk. Verify: (1) chunk size is correct, (2) the underflow channel is entirely zeros, (3) the other channel (if it has sufficient data) contains actual audio data.

**Note:** This is simpler than partial reads—if a buffer doesn't have enough data for a full chunk, treat it as empty for that chunk cycle. This prevents audio glitches from partial frames.

#### Property 16: Maximum Value Clamping in Mono Mix
*For any* two Int16 samples where their sum exceeds Int16 range, the mono mixing SHALL clamp the result to [-32768, 32767] without overflow or wraparound.

**Validates: Requirements 5.6**

**Testing approach:** Generate pairs of Int16 values that sum to values outside Int16 range (e.g., 32767 + 32767 = 65534, should clamp to 32767). Mix them. Verify output is correctly clamped.

### Round-Trip Properties

#### Property 17: Byte Array to Int16 Conversion Round-Trip
*For any* valid Int16 value, converting it to a little-endian byte pair and back to Int16 SHALL produce the original value.

**Validates: Requirements 5.4**

**Testing approach:** Generate random Int16 values. Convert to bytes (little-endian). Convert back to Int16. Verify result equals original value.

### Configuration Properties

These properties validate that configuration objects correctly represent parsed arguments:

#### Property 18: Default Configuration Values
*When* the argument parser receives an empty argument list (no flags), the resulting configuration SHALL have: sampleRate = 16000, outputMono = false, microphoneDeviceID = nil.

**Validates: Requirements 6.1**

**Testing approach:** Parse empty argument list. Verify configuration has default values.

#### Property 19: Mono Flag Configuration
*When* the argument parser receives the --mono flag, the resulting configuration SHALL have outputMono = true.

**Validates: Requirements 6.2**

**Testing approach:** Parse arguments with --mono flag. Verify configuration.outputMono is true.

#### Property 20: Sample Rate Flag Configuration
*For any* valid sample rate value N in {8000, 16000, 24000, 44100, 48000}, when the argument parser receives --sample-rate N, the resulting configuration SHALL have sampleRate = N.

**Validates: Requirements 6.3**

**Testing approach:** For each valid sample rate, parse arguments with --sample-rate <value>. Verify configuration.sampleRate equals the provided value.

## Testing Strategy

### Dual Testing Approach

This project requires both unit tests and property-based tests to ensure comprehensive correctness:

**Unit Tests** focus on:
- Specific examples and scenarios (e.g., SIGINT handling, permission error messages)
- Integration points between components (e.g., AudioCapture → PCMConverter flow)
- Edge cases that are difficult to generate randomly (e.g., device disconnection)
- Configuration validation for specific values

**Property-Based Tests** focus on:
- Universal properties that hold across all inputs (e.g., format conversion, interleaving)
- Randomized input generation to find edge cases
- Invariants that must be maintained (e.g., ring buffer capacity, chunk sizes)
- Mathematical properties (e.g., mono mixing averages, clamping)

Together, these approaches provide comprehensive coverage: unit tests catch concrete bugs in specific scenarios, while property tests verify general correctness across the input space.

### Property-Based Testing Configuration

**Framework:** Manual implementation using XCTest with randomized input generation. No external dependencies required—property-based tests are implemented as simple loops generating random inputs.

**Test Configuration:**
- Minimum 100 iterations per property test (due to randomization)
- Each test must reference its design document property in a comment
- Tag format: `// Feature: jarvis-listen, Property N: <property title>`
- Use `arc4random_uniform()` or `Int.random(in:)` for random value generation

**Example:**
```swift
// Feature: jarvis-listen, Property 1: Audio Format Conversion Completeness
func testAudioFormatConversionCompleteness() {
    let converter = PCMConverter(configuration: testConfig)
    
    // Test 100 random audio formats
    for _ in 0..<100 {
        let inputFormat = generateRandomAudioFormat()
        let inputBuffer = generateAudioBuffer(format: inputFormat)
        
        let output = try! converter.convert(inputBuffer)
        
        XCTAssertEqual(output.sampleRate, 16000, "Sample rate should be 16kHz")
        XCTAssertEqual(output.bitDepth, 16, "Bit depth should be 16-bit")
        XCTAssertEqual(output.channels, 1, "Should be mono")
        XCTAssertTrue(output.isSigned, "Should be signed integer")
        XCTAssertTrue(output.isLittleEndian, "Should be little-endian")
    }
}

// Helper to generate random audio formats for testing
func generateRandomAudioFormat() -> AVAudioFormat {
    let sampleRates: [Double] = [8000, 16000, 22050, 44100, 48000]
    let channels: [UInt32] = [1, 2, 4, 6]
    let sampleRate = sampleRates.randomElement()!
    let channelCount = channels.randomElement()!
    return AVAudioFormat(commonFormat: .pcmFormatFloat32, 
                        sampleRate: sampleRate, 
                        channels: channelCount, 
                        interleaved: Bool.random())!
}
```

**Note:** This approach maintains the zero-dependency principle—even test targets use only Apple frameworks (XCTest, Foundation). Property-based testing is achieved through simple randomized loops rather than external PBT frameworks.

### Unit Testing Strategy

**Framework:** Use XCTest (Apple's standard testing framework).

**Test Organization:**
- `AudioCaptureTests.swift` - SCStream setup, device enumeration, callback handling
- `PCMConverterTests.swift` - Format conversion, ring buffers, interleaving, mixing
- `ArgumentParserTests.swift` - Flag parsing, validation, error handling
- `SignalHandlerTests.swift` - SIGINT/SIGTERM/SIGPIPE handling
- `IntegrationTests.swift` - End-to-end flows (may require mocking ScreenCaptureKit)

**Key Test Cases:**
- Configuration validation (invalid sample rates, device IDs)
- Permission error messages (screen recording, microphone)
- Signal handling (graceful shutdown, buffer flushing)
- Device changes (disconnection, fallback to default)
- Error logging (all errors to stderr, never stdout)
- Startup messages (device name, format, capture method)

### Testing Challenges and Mitigations

**Challenge 1: ScreenCaptureKit Dependency**
- Mitigation: Use AudioCaptureProvider protocol for dependency injection
- Create MockAudioCapture for testing without actual screen capture
- Test AudioCapture implementation separately with integration tests

**Challenge 2: Real-Time Audio Timing**
- Mitigation: Use deterministic time in tests (mock CMTime)
- Test ring buffer logic independently of actual audio timing
- Use synthetic audio buffers with known patterns

**Challenge 3: Permission Handling**
- Mitigation: Mock permission checks in tests
- Document manual testing procedures for actual permission flows
- Test error message content without requiring actual permission denial

**Challenge 4: Signal Handling**
- Mitigation: Test signal handler logic separately from actual signal delivery
- Use flag-based shutdown triggers in tests
- Verify cleanup logic is called correctly

### Test Coverage Goals

- **Line Coverage:** >80% for core logic (PCMConverter, ArgumentParser, RingBuffer)
- **Branch Coverage:** >70% for error handling paths
- **Property Coverage:** 100% of identified correctness properties implemented as tests
- **Integration Coverage:** Key end-to-end flows tested (capture → convert → output)

### Continuous Testing

- Run unit tests on every commit
- Run property tests (100 iterations) on every commit
- Run extended property tests (1000 iterations) nightly
- Monitor for flaky tests (especially timing-dependent tests)
- Track test execution time (property tests may be slower)

