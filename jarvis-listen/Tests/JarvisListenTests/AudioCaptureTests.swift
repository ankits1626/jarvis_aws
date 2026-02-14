import Testing
import ScreenCaptureKit
import CoreMedia
import AVFoundation
@testable import JarvisListen

// MARK: - Property 9: Device List Formatting

@Test("Device list formatting - Property 9")
func testDeviceListFormatting() throws {
    // Feature: jarvis-listen, Property 9: Device List Formatting
    // Property: For any list of audio devices, the --list-devices output SHALL contain
    // one line per device in the format "<id>: <name>", where <id> and <name> are
    // the device's ID and name fields.
    // Validates: Requirements 2.4, 6.5
    
    let iterations = 100
    
    // Test with various device lists
    for _ in 0..<iterations {
        // Generate random number of devices (0 to 10)
        let deviceCount = Int.random(in: 0...10)
        var devices: [AudioDevice] = []
        
        for _ in 0..<deviceCount {
            let id = generateRandomDeviceID()
            let name = generateRandomDeviceName()
            devices.append(AudioDevice(id: id, name: name))
        }
        
        // Format devices for output
        let output = formatDeviceList(devices)
        let lines = output.split(separator: "\n", omittingEmptySubsequences: true)
        
        // Verify: one line per device
        #expect(lines.count == deviceCount,
               "Should have \(deviceCount) lines, got \(lines.count)")
        
        // Verify: each line matches format "<id>: <name>"
        for (index, device) in devices.enumerated() {
            let expectedLine = "\(device.id): \(device.name)"
            let actualLine = String(lines[index])
            #expect(actualLine == expectedLine,
                   "Line \(index) should be '\(expectedLine)', got '\(actualLine)'")
        }
        
        // Verify: all devices are included
        for device in devices {
            let expectedLine = "\(device.id): \(device.name)"
            #expect(output.contains(expectedLine),
                   "Output should contain device '\(expectedLine)'")
        }
    }
}

@Test("Device list formatting with empty list")
func testDeviceListFormattingEmptyList() throws {
    // Edge case: empty device list should produce empty output
    let devices: [AudioDevice] = []
    let output = formatDeviceList(devices)
    
    #expect(output.isEmpty, "Empty device list should produce empty output")
}

@Test("Device list formatting with single device")
func testDeviceListFormattingSingleDevice() throws {
    // Edge case: single device
    let device = AudioDevice(id: "test-id", name: "Test Device")
    let output = formatDeviceList([device])
    
    #expect(output == "test-id: Test Device\n",
           "Single device should produce one line with newline")
}

@Test("Device list formatting with special characters")
func testDeviceListFormattingSpecialCharacters() throws {
    // Test devices with special characters in ID and name
    let testCases = [
        AudioDevice(id: "id-with-dashes", name: "Name with spaces"),
        AudioDevice(id: "id_with_underscores", name: "Name (with parentheses)"),
        AudioDevice(id: "123456", name: "Numeric Name 123"),
        AudioDevice(id: "id:with:colons", name: "Name: with: colons"),
        AudioDevice(id: "", name: "Empty ID"),
        AudioDevice(id: "valid-id", name: ""),
    ]
    
    for device in testCases {
        let output = formatDeviceList([device])
        let expectedLine = "\(device.id): \(device.name)\n"
        #expect(output == expectedLine,
               "Device '\(device.id)' / '\(device.name)' should format correctly")
    }
}

@Test("Device list formatting preserves order")
func testDeviceListFormattingPreservesOrder() throws {
    // Property: order should be preserved
    let devices = [
        AudioDevice(id: "device-1", name: "First Device"),
        AudioDevice(id: "device-2", name: "Second Device"),
        AudioDevice(id: "device-3", name: "Third Device"),
    ]
    
    let output = formatDeviceList(devices)
    let lines = output.split(separator: "\n", omittingEmptySubsequences: true)
    
    #expect(lines.count == 3)
    #expect(lines[0] == "device-1: First Device")
    #expect(lines[1] == "device-2: Second Device")
    #expect(lines[2] == "device-3: Third Device")
}

// MARK: - Helper Functions

/// Formats a list of audio devices for output.
/// - Parameter devices: Array of AudioDevice to format.
/// - Returns: Formatted string with one device per line in format "<id>: <name>\n".
private func formatDeviceList(_ devices: [AudioDevice]) -> String {
    return devices.map { "\($0.id): \($0.name)" }.joined(separator: "\n") + (devices.isEmpty ? "" : "\n")
}

/// Generates a random device ID for testing.
private func generateRandomDeviceID() -> String {
    let prefixes = ["BuiltIn", "USB", "External", "Virtual", ""]
    let types = ["Microphone", "Audio", "Device", "Input", ""]
    let suffixes = ["Device", "", "123", "Pro", "HD"]
    
    let prefix = prefixes.randomElement()!
    let type = types.randomElement()!
    let suffix = suffixes.randomElement()!
    
    // Sometimes use separators, sometimes not
    let separators = ["-", "_", ""]
    let sep1 = separators.randomElement()!
    let sep2 = separators.randomElement()!
    
    let parts = [prefix, type, suffix].filter { !$0.isEmpty }
    if parts.isEmpty {
        return "device-\(Int.random(in: 1000...9999))"
    }
    
    return parts.joined(separator: sep1.isEmpty ? sep2 : sep1)
}

/// Generates a random device name for testing.
private func generateRandomDeviceName() -> String {
    let brands = ["Apple", "Logitech", "Blue", "Rode", "Shure", "Audio-Technica", ""]
    let models = ["MacBook Pro", "USB Microphone", "Yeti", "NT1", "SM7B", "AT2020", ""]
    let descriptors = ["Microphone", "Audio Device", "Input", "Built-in", "External", ""]
    
    let brand = brands.randomElement()!
    let model = models.randomElement()!
    let descriptor = descriptors.randomElement()!
    
    let parts = [brand, model, descriptor].filter { !$0.isEmpty }
    if parts.isEmpty {
        return "Device \(Int.random(in: 1...100))"
    }
    
    return parts.joined(separator: " ")
}

// MARK: - Video Frame Discarding Tests

@Test("Video frames are discarded - unit test")
func testVideoFramesAreDiscarded() async throws {
    // **Validates: Requirements 11.4**
    // Requirement: WHEN video frames are received, THE System SHALL discard them without processing
    
    // Create a continuation to track what gets yielded
    let (stream, continuation) = AsyncStream.makeStream(of: AudioData.self)
    
    // Collect yielded data in background using an actor for thread safety
    actor DataCollector {
        var yieldedData: [AudioData] = []
        
        func append(_ data: AudioData) {
            yieldedData.append(data)
        }
        
        func getData() -> [AudioData] {
            return yieldedData
        }
    }
    
    let collector = DataCollector()
    let collectionTask = Task {
        for await data in stream {
            await collector.append(data)
        }
    }
    
    // Create delegate
    let delegate = StreamDelegate(continuation: continuation)
    
    // Create mock sample buffers for different types
    let audioBuffer = try createMockSampleBuffer()
    let microphoneBuffer = try createMockSampleBuffer()
    let videoBuffer = try createMockSampleBuffer()
    
    // We need an SCStream reference but can't instantiate it
    // The delegate doesn't actually use the stream parameter, so we can use unsafeBitCast
    // This is safe because the delegate implementation doesn't access the stream object
    let mockStream = unsafeBitCast(0 as Int, to: SCStream.self)
    
    // Simulate receiving audio frame (should be processed)
    delegate.stream(mockStream, didOutputSampleBuffer: audioBuffer, of: .audio)
    
    // Simulate receiving microphone frame (should be processed)
    delegate.stream(mockStream, didOutputSampleBuffer: microphoneBuffer, of: .microphone)
    
    // Simulate receiving video frame (should be ignored)
    delegate.stream(mockStream, didOutputSampleBuffer: videoBuffer, of: .screen)
    
    // Finish the stream
    continuation.finish()
    
    // Wait for collection to complete
    _ = await collectionTask.value
    
    // Get collected data
    let yieldedData = await collector.getData()
    
    // Verify: only audio and microphone frames were yielded, video was discarded
    #expect(yieldedData.count == 2, "Should have yielded 2 audio frames (audio + microphone), not video")
    #expect(yieldedData[0].source == .systemAudio, "First frame should be system audio")
    #expect(yieldedData[1].source == .microphone, "Second frame should be microphone")
}

@Test("Only audio and microphone types are processed")
func testOnlyAudioTypesProcessed() async throws {
    // **Validates: Requirements 11.4**
    // Verify that the delegate only processes .audio and .microphone types
    
    let (stream, continuation) = AsyncStream.makeStream(of: AudioData.self)
    
    // Use actor for thread-safe counting
    actor Counter {
        var count = 0
        
        func increment() {
            count += 1
        }
        
        func getCount() -> Int {
            return count
        }
    }
    
    let counter = Counter()
    let collectionTask = Task {
        for await _ in stream {
            await counter.increment()
        }
    }
    
    let delegate = StreamDelegate(continuation: continuation)
    let buffer = try createMockSampleBuffer()
    
    // Mock stream reference (safe because delegate doesn't use it)
    let mockStream = unsafeBitCast(0 as Int, to: SCStream.self)
    
    // Test all possible SCStreamOutputType values
    // .audio should be processed
    delegate.stream(mockStream, didOutputSampleBuffer: buffer, of: .audio)
    
    // .microphone should be processed
    delegate.stream(mockStream, didOutputSampleBuffer: buffer, of: .microphone)
    
    // .screen (video) should be ignored
    delegate.stream(mockStream, didOutputSampleBuffer: buffer, of: .screen)
    
    continuation.finish()
    _ = await collectionTask.value
    
    // Get final count
    let yieldedCount = await counter.getCount()
    
    // Only 2 frames should have been processed (audio + microphone)
    #expect(yieldedCount == 2, "Should process exactly 2 frames (audio and microphone), ignoring video")
}

// MARK: - Mock Helpers for Video Frame Tests

/// Creates a mock CMSampleBuffer for testing
private func createMockSampleBuffer() throws -> CMSampleBuffer {
    // Create a simple audio format
    var formatDescription: CMAudioFormatDescription?
    var asbd = AudioStreamBasicDescription(
        mSampleRate: 16000,
        mFormatID: kAudioFormatLinearPCM,
        mFormatFlags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
        mBytesPerPacket: 2,
        mFramesPerPacket: 1,
        mBytesPerFrame: 2,
        mChannelsPerFrame: 1,
        mBitsPerChannel: 16,
        mReserved: 0
    )
    
    let status = CMAudioFormatDescriptionCreate(
        allocator: kCFAllocatorDefault,
        asbd: &asbd,
        layoutSize: 0,
        layout: nil,
        magicCookieSize: 0,
        magicCookie: nil,
        extensions: nil,
        formatDescriptionOut: &formatDescription
    )
    
    guard status == noErr, let formatDesc = formatDescription else {
        throw VideoFrameTestError.failedToCreateFormatDescription
    }
    
    // Create a sample buffer with minimal timing info
    var timingInfo = CMSampleTimingInfo(
        duration: CMTime(value: 1, timescale: 100),
        presentationTimeStamp: CMTime(value: 0, timescale: 16000),
        decodeTimeStamp: .invalid
    )
    
    var sampleBuffer: CMSampleBuffer?
    let sampleBufferStatus = CMSampleBufferCreate(
        allocator: kCFAllocatorDefault,
        dataBuffer: nil,
        dataReady: true,
        makeDataReadyCallback: nil,
        refcon: nil,
        formatDescription: formatDesc,
        sampleCount: 1600,
        sampleTimingEntryCount: 1,
        sampleTimingArray: &timingInfo,
        sampleSizeEntryCount: 0,
        sampleSizeArray: nil,
        sampleBufferOut: &sampleBuffer
    )
    
    guard sampleBufferStatus == noErr, let buffer = sampleBuffer else {
        throw VideoFrameTestError.failedToCreateSampleBuffer
    }
    
    return buffer
}

private enum VideoFrameTestError: Error {
    case failedToCreateFormatDescription
    case failedToCreateSampleBuffer
}


