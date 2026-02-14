# Implementation Plan: JarvisListen

## Overview

This implementation plan breaks down the JarvisListen audio capture tool into discrete coding tasks. The approach follows a bottom-up strategy: build foundational components first (data models, protocols, utilities), then core functionality (audio capture, conversion, synchronization), and finally wire everything together in the main entry point.

Each task builds incrementally on previous work, with property-based tests and unit tests integrated as sub-tasks to validate correctness early. The implementation uses only Apple frameworks (ScreenCaptureKit, CoreMedia, AVFoundation, Foundation) with zero external dependencies.

## Tasks

- [x] 1. Set up project structure and foundational types
  - Create Swift package with Sources/ directory
  - Create Package.swift with platform requirements (macOS 15.0+, arm64)
  - Define build configuration for binary target named "JarvisListen"
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 2. Implement core data models and protocols
  - [x] 2.1 Create AudioCaptureProvider.swift with protocol definition
    - Define AudioCaptureProvider protocol with startCapture(), stopCapture(), audioDataStream
    - Enable testability through protocol abstraction
    - _Requirements: 10.5_
  
  - [x] 2.2 Create data models in AudioCapture.swift
    - Define AudioData struct (source enum, CMSampleBuffer, timestamp)
    - Define AudioDevice struct (id, name)
    - Define CaptureConfiguration struct (sampleRate, outputMono, microphoneDeviceID)
    - Add validSampleRates constant and bytesPerChunk() method
    - _Requirements: 2.2, 2.3, 3.4, 5.5, 5.6, 6.1, 6.2, 6.3_
  
  - [x]* 2.3 Write property test for CaptureConfiguration.bytesPerChunk()
    - **Property 6: Chunk Size Consistency**
    - **Validates: Requirements 5.5, 5.6**
    - For each valid sample rate (8000, 16000, 24000, 44100, 48000) and mono/stereo setting
    - Verify chunk size = sampleRate × 0.1 × 2 × channelCount
    - Run 100 iterations with randomized configurations

- [x] 3. Implement RingBuffer for audio synchronization
  - [x] 3.1 Create RingBuffer class in PCMConverter.swift
    - Implement circular buffer with os_unfair_lock for thread safety
    - Implement init(capacity:), write(_:), read(_:), availableData(), clear()
    - write() returns true if data fit, false if overflow occurred (oldest data discarded)
    - read() returns nil if insufficient data available
    - _Requirements: 4.1, 4.2, 4.3, 4.4_
  
  - [x]* 3.2 Write property test for ring buffer overflow handling
    - **Property 2: Ring Buffer Overflow Handling**
    - **Validates: Requirements 4.3**
    - Generate random buffer states at/near capacity
    - Write data causing overflow
    - Verify oldest data discarded, new data written, capacity maintained
    - Run 100 iterations
  
  - [x]* 3.3 Write property test for ring buffer underflow handling
    - **Property 3: Silent Channel Filling**
    - **Validates: Requirements 4.4**
    - Generate buffer states with insufficient data
    - Verify read() returns nil when data unavailable
    - Run 100 iterations
  
  - [x]* 3.4 Write property test for buffer flush completeness
    - **Property 12: Buffer Flush Completeness**
    - **Validates: Requirements 7.2**
    - Generate random buffer states with varying data amounts
    - Read all available data
    - Verify buffer empty afterward (availableBytes = 0)
    - Run 100 iterations

- [x] 4. Implement argument parsing
  - [x] 4.1 Create ArgumentParser struct in main.swift
    - Define ParsedArguments struct with Action enum (capture, listDevices, showHelp)
    - Implement parse(_:) method to handle all flags
    - Handle --mono, --sample-rate, --mic-device, --list-devices, --help
    - Validate sample rate against valid values {8000, 16000, 24000, 44100, 48000}
    - Return appropriate errors for invalid flags or values
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_
  
  - [x]* 4.2 Write property test for sample rate validation
    - **Property 7: Sample Rate Validation**
    - **Validates: Requirements 3.4**
    - Generate random integers (including valid and invalid sample rates)
    - Verify valid rates accepted, invalid rates rejected with error
    - Verify error message lists valid values
    - Run 100 iterations
  
  - [x]* 4.3 Write property test for device ID argument parsing
    - **Property 8: Device ID Argument Parsing**
    - **Validates: Requirements 2.3, 6.4**
    - Generate random device ID strings (valid IDs, invalid IDs, empty, special chars)
    - Verify configuration stores exact string provided
    - Run 100 iterations
  
  - [x]* 4.4 Write property test for invalid flag rejection
    - **Property 13: Invalid Flag Rejection**
    - **Validates: Requirements 6.7**
    - Generate random invalid flags (--invalid, --xyz, -z, etc.)
    - Verify parser rejects with error and suggests --help
    - Run 100 iterations
  
  - [x]* 4.5 Write unit tests for configuration properties
    - **Property 18: Default Configuration Values**
    - **Property 19: Mono Flag Configuration**
    - **Property 20: Sample Rate Flag Configuration**
    - **Validates: Requirements 6.1, 6.2, 6.3**
    - Test empty argument list produces defaults (16000Hz, stereo, nil device)
    - Test --mono flag sets outputMono=true
    - Test --sample-rate flag sets correct value

- [x] 5. Checkpoint - Ensure foundational components work
  - Run all tests for data models, ring buffer, and argument parser
  - Verify no compilation errors
  - Ask user if questions arise

- [x] 6. Implement audio format conversion
  - [x] 6.1 Create PCMConverter class in PCMConverter.swift
    - Initialize with CaptureConfiguration
    - Create two RingBuffer instances (mic and system, 2-second capacity)
    - Implement convert(_:) method using AVAudioConverter
    - Extract AudioBufferList from CMSampleBuffer
    - Convert any format to target: specified sample rate, s16le, mono, little-endian
    - Cache AVAudioConverter instances for reuse
    - Write converted bytes to appropriate ring buffer
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2_
  
  - [x]* 6.2 Write property test for audio format conversion completeness
    - **Property 1: Audio Format Conversion Completeness**
    - **Validates: Requirements 3.1, 3.2, 3.3, 5.4**
    - Generate random audio formats (varying sample rates, bit depths, channels)
    - Convert through PCMConverter
    - Verify output matches target format (sample rate, bit depth, mono, little-endian)
    - Run 100 iterations
  
  - [x]* 6.3 Write unit test for conversion error handling
    - Test that conversion failures log to stderr and continue processing
    - Verify error messages written to stderr only
    - _Requirements: 3.5, 8.2_

- [x] 7. Implement stereo interleaving and mono mixing
  - [x] 7.1 Add interleave(mic:system:) method to PCMConverter
    - Take two mono byte arrays (mic and system audio)
    - Produce stereo output with alternating L-R-L-R samples
    - Channel 0 (left) = microphone, Channel 1 (right) = system audio
    - _Requirements: 5.1, 5.2, 5.3_
  
  - [x] 7.2 Add mix(mic:system:) method to PCMConverter
    - Take two mono byte arrays
    - Average corresponding Int16 samples: (mic[i] + sys[i]) / 2
    - Clamp to Int16 range [-32768, 32767]
    - Return mono output
    - _Requirements: 5.6_
  
  - [x]* 7.3 Write property test for stereo interleaving correctness
    - **Property 4: Stereo Interleaving Correctness**
    - **Validates: Requirements 5.2, 5.3**
    - Generate random mono buffers with known patterns (e.g., incrementing values)
    - Interleave them
    - Extract left/right channels from output
    - Verify left matches microphone input, right matches system audio input
    - Verify total sample count preserved
    - Run 100 iterations
  
  - [x]* 7.4 Write property test for mono mixing correctness
    - **Property 5: Mono Mixing Correctness**
    - **Validates: Requirements 5.6**
    - Generate random mono buffers with known values
    - Mix them
    - Verify each output sample = (mic[i] + sys[i]) / 2, clamped
    - Test edge cases: max values, min values, opposite signs
    - Run 100 iterations
  
  - [x]* 7.5 Write property test for maximum value clamping
    - **Property 16: Maximum Value Clamping in Mono Mix**
    - **Validates: Requirements 5.6**
    - Generate Int16 pairs that sum outside Int16 range
    - Verify mixing clamps correctly without overflow (e.g., 32767 + 32767 → 32767)
    - Run 100 iterations
  
  - [x]* 7.6 Write property test for byte/Int16 round-trip
    - **Property 17: Byte Array to Int16 Conversion Round-Trip**
    - **Validates: Requirements 5.4**
    - Generate random Int16 values
    - Convert to [UInt8] little-endian pair
    - Convert back to Int16
    - Verify round-trip produces original value
    - Run 100 iterations

- [x] 8. Implement chunk generation with synchronization
  - [x] 8.1 Add generateChunk() method to PCMConverter
    - Calculate chunk size based on configuration (sampleRate × 0.1 × 2 × channels)
    - Read from micBuffer (full chunk size)
    - If insufficient data, use all zeros (silence) for entire channel
    - Read from systemBuffer (full chunk size)
    - If insufficient data, use all zeros (silence) for entire channel
    - If outputMono: call mix(), return mono chunk (3,200 bytes for 16kHz)
    - Else: call interleave(), return stereo chunk (6,400 bytes for 16kHz)
    - _Requirements: 4.4, 4.5, 5.5, 5.6_
  
  - [x] 8.2 Add flush() method to PCMConverter
    - Read all available data from both ring buffers
    - Generate final chunk(s) from remaining data
    - Return combined bytes for stdout output
    - _Requirements: 7.2_
  
  - [x]* 8.3 Write property test for empty buffer chunk generation
    - **Property 14: Empty Buffer Chunk Generation**
    - **Validates: Requirements 4.4**
    - Create empty ring buffers
    - Generate chunk
    - Verify correct size for configuration and all bytes are zero
    - Run 100 iterations
  
  - [x]* 8.4 Write property test for partial buffer handling
    - **Property 15: Full Silence on Buffer Underflow**
    - **Validates: Requirements 4.4, 4.5**
    - Generate buffers with partial data (e.g., 50% of chunk size)
    - Generate chunk
    - Verify underflow channel is entirely zeros
    - Verify other channel (if sufficient data) contains actual audio
    - Run 100 iterations

- [x] 9. Checkpoint - Ensure audio processing pipeline works
  - Run all tests for PCMConverter, interleaving, mixing, and chunk generation
  - Verify no compilation errors
  - Ask user if questions arise

- [x] 10. Implement ScreenCaptureKit audio capture
  - [x] 10.1 Create StreamDelegate class in AudioCapture.swift
    - Subclass NSObject, conform to SCStreamOutput
    - Store AsyncStream.Continuation in init
    - Implement stream(_:didOutputSampleBuffer:of:)
    - Determine source from type (.microphone or .audio)
    - Extract timestamp from CMSampleBuffer
    - Create AudioData and yield to continuation
    - _Requirements: 1.1, 1.2, 2.1_
  
  - [x] 10.2 Create AudioCapture actor in AudioCapture.swift
    - Conform to AudioCaptureProvider protocol
    - Store CaptureConfiguration
    - Use AsyncStream.makeStream(of: AudioData.self) in init to avoid race conditions
    - Create StreamDelegate with continuation
    - Store SCStream instance
    - _Requirements: 1.1, 2.1, 10.5_
  
  - [x] 10.3 Implement startCapture() method
    - Get SCShareableContent.current to discover displays
    - Select first display for SCContentFilter
    - Create SCStreamConfiguration with audio settings:
      * capturesAudio = true (system audio)
      * captureMicrophone = true (microphone)
      * excludesCurrentProcessAudio = true (prevent feedback)
      * width = 2, height = 2 (minimal video overhead)
      * minimumFrameInterval = CMTime(value: 1, timescale: 1) (1 FPS)
    - If microphoneDeviceID specified, set it in configuration
    - Create SCStream with filter and configuration
    - Add StreamDelegate as output
    - Call stream.startCapture()
    - On success, print startup message to stderr: "Capturing: mic=<device name>, format=<rate>Hz s16le <stereo|mono>, method=ScreenCaptureKit"
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.3, 8.6, 11.1, 11.2, 11.3_
  
  - [x] 10.4 Implement stopCapture() method
    - Call stream.stopCapture()
    - Release ScreenCaptureKit resources
    - _Requirements: 7.1, 7.6_
  
  - [x] 10.5 Implement listDevices() method
    - Query available audio input devices using AVCaptureDevice.DiscoverySession
    - Use device types: .builtInMicrophone and .externalUnknown
    - Return array of AudioDevice structs (id, name)
    - _Requirements: 2.4_
  
  - [x] 10.6 Implement device change handling
    - Monitor for microphone disconnect via SCStream error callbacks or CoreAudio
    - On disconnect: log warning to stderr, reconfigure with default device (nil), continue
    - Warning message: "Warning: Microphone device '<id>' disconnected. Falling back to default device."
    - System audio device changes handled automatically by ScreenCaptureKit
    - _Requirements: 2.6, 2.7_
  
  - [x]* 10.7 Write property test for device list formatting
    - **Property 9: Device List Formatting**
    - **Validates: Requirements 2.4, 6.5**
    - Generate random AudioDevice lists with various IDs and names
    - Format for output
    - Verify format "<id>: <name>", one line per device
    - Verify all devices included, order preserved
    - Run 100 iterations
  
  - [x]* 10.8 Write unit tests for SCStream configuration
    - Verify capturesAudio = true
    - Verify excludesCurrentProcessAudio = true
    - Verify captureMicrophone = true
    - Verify video dimensions = 2x2
    - Verify minimumFrameInterval = 1 second
    - _Requirements: 1.1, 1.2, 2.1, 11.1, 11.2, 11.3_
  
  - [x]* 10.9 Write unit test for video frame discarding
    - Verify video frames are ignored in delegate callback
    - Only .audio and .microphone types processed
    - _Requirements: 11.4_

- [x] 11. Implement signal handling
  - [x] 11.1 Create SignalHandler class in main.swift
    - Store shutdown handler closure
    - Implement setup(onShutdown:) to register signal handlers
    - Implement handleSIGINT() - call shutdown handler
    - Implement handleSIGTERM() - call shutdown handler
    - Implement handleSIGPIPE() - exit gracefully (downstream consumer closed pipe)
    - Use signal() or DispatchSource for signal handling
    - _Requirements: 7.1, 7.3, 7.4, 7.5_
  
  - [x]* 11.2 Write unit tests for signal handling
    - Test SIGINT triggers shutdown handler
    - Test SIGTERM triggers shutdown handler
    - Test SIGPIPE exits gracefully
    - _Requirements: 7.1, 7.3, 7.4, 7.5_

- [x] 12. Implement error handling and logging
  - [x] 12.1 Add error handling utilities to main.swift
    - Create helper functions for logging to stderr (FileHandle.standardError)
    - Create permission error message formatters
    - Implement exit code constants (0 = success, 1 = error)
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_
  
  - [x]* 12.2 Write property test for error output destination
    - **Property 10: Error Output Destination**
    - **Validates: Requirements 8.2, 8.3**
    - Trigger various errors and log events
    - Capture stdout and stderr separately
    - Verify all errors/logs go to stderr, stdout only has PCM data (or empty)
    - Run 100 iterations
  
  - [ ]* 12.3 Write property test for unrecoverable error exit codes
    - **Property 11: Unrecoverable Error Exit Codes**
    - **Validates: Requirements 8.4**
    - Trigger various unrecoverable errors (permission denied, invalid config, capture start failure)
    - Capture exit codes
    - Verify all are non-zero (typically 1)
    - Run 100 iterations
  
  - [ ]* 12.4 Write unit tests for permission error messages
    - Test screen recording permission denied message format
    - Verify message includes instructions: "System Settings > Privacy & Security > Screen & System Audio Recording"
    - Test microphone permission denied warning format
    - Verify message includes instructions: "System Settings > Privacy & Security > Microphone"
    - Test partial permission handling (screen recording granted, mic denied)
    - Verify system continues with system audio only, outputs silence on channel 0
    - _Requirements: 9.1, 9.2, 9.3, 9.4_
  
  - [ ]* 12.5 Write unit test for startup success message
    - Verify message format: "Capturing: mic=<device name>, format=<rate>Hz s16le <stereo|mono>, method=ScreenCaptureKit"
    - Test with various configurations (different sample rates, mono/stereo)
    - Verify message written to stderr only
    - _Requirements: 8.6_

- [x] 13. Checkpoint - Ensure all components are tested
  - Run all unit tests and property tests
  - Verify test coverage for core functionality
  - Ask user if questions arise

- [x] 14. Implement main entry point and run loop
  - [x] 14.1 Create main.swift entry point
    - Parse command-line arguments using ArgumentParser
    - Handle --help action: print usage information to stderr, exit 0
    - Handle --list-devices action: call listDevices(), format output as "<id>: <name>", print to stdout, exit 0
    - Handle capture action: proceed with audio capture setup
    - _Requirements: 6.1, 6.5, 6.6_
  
  - [x] 14.2 Implement capture run loop
    - Create CaptureConfiguration from parsed arguments
    - Create AudioCapture instance with configuration
    - Create PCMConverter instance with configuration
    - Set up SignalHandler with shutdown logic (stop capture, flush buffers, exit 0)
    - Start AudioCapture (await startCapture())
    - Create async Task for chunk generation timer (Task.sleep 100ms loop)
    - Create async Task for processing AudioData stream from audioDataStream
    - On AudioData: call pcmConverter.process() to convert and buffer
    - On timer tick: call pcmConverter.generateChunk(), write bytes to stdout
    - On shutdown signal: stop capture, flush buffers, write remaining data to stdout, exit 0
    - _Requirements: 1.3, 1.4, 2.2, 4.5, 5.1, 7.1, 7.2, 7.3_
  
  - [x] 14.3 Add error handling for capture failures
    - Catch ScreenCaptureKit permission errors, print helpful message to stderr
    - Message format: "Error: Screen Recording permission denied. Please enable it in System Settings > Privacy & Security > Screen & System Audio Recording"
    - Catch configuration errors (invalid sample rate, device not found), print error and exit with code 1
    - Catch runtime errors (capture start failure), log to stderr and exit with non-zero code
    - Handle microphone permission denial gracefully: warn to stderr, continue with system audio only
    - _Requirements: 8.1, 8.4, 8.5, 9.1, 9.2, 9.3, 9.4_

- [ ] 15. Integration and end-to-end testing
  - [ ]* 15.1 Write integration test for full capture pipeline
    - Mock AudioCapture with synthetic audio data (known patterns)
    - Verify data flows through PCMConverter to output
    - Verify chunk timing (100ms intervals) and sizes (6,400 bytes stereo, 3,200 bytes mono)
    - Verify correct channel assignment (mic=left, system=right)
    - _Requirements: 1.3, 1.4, 3.1, 3.2, 3.3, 4.5, 5.1, 5.5_
  
  - [ ]* 15.2 Write integration test for mono output mode
    - Test --mono flag end-to-end with mock audio
    - Verify output is mono (single channel)
    - Verify chunk size is 3,200 bytes for 16kHz
    - Verify samples are averaged correctly
    - _Requirements: 5.6, 6.2_
  
  - [ ]* 15.3 Write integration test for custom sample rate
    - Test --sample-rate flag with various valid values (8000, 24000, 44100, 48000)
    - Verify output has correct sample rate
    - Verify chunk size adjusts correctly (sampleRate × 0.1 × 2 × channels)
    - _Requirements: 3.4, 6.3_
  
  - [ ]* 15.4 Write integration test for device fallback
    - Simulate microphone device disconnection during capture
    - Verify system falls back to default device
    - Verify warning logged to stderr
    - Verify capture continues without interruption
    - _Requirements: 2.6_

- [x] 16. Final checkpoint - Ensure all tests pass
  - Run complete test suite (unit + property + integration)
  - Verify no compilation errors or warnings
  - Verify binary builds successfully for arm64
  - Test manual execution with actual ScreenCaptureKit (requires permissions)
  - Ask user if questions arise

## Notes

- Tasks marked with `*` are optional test tasks and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at logical breakpoints
- Property tests validate universal correctness properties (minimum 100 iterations each)
- Unit tests validate specific examples, edge cases, and error conditions
- Integration tests validate end-to-end flows with mocked dependencies
- The implementation follows a bottom-up approach: foundations → core logic → integration
- All tests use XCTest framework (Apple's standard) - no external dependencies
- Property-based testing implemented as simple loops with randomized inputs
- Tag format for property tests: `// Feature: jarvis-listen, Property N: <property title>`
- Zero external dependencies maintained throughout - Apple frameworks only
- All PCM data to stdout, all logs/errors to stderr
- Exit codes: 0 = success/graceful shutdown, 1 = error
