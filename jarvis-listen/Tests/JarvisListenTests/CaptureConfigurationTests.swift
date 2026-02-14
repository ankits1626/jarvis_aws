import Testing
@testable import JarvisListen

// MARK: - Property 6: Chunk Size Consistency

@Test("Chunk size consistency - Property 6")
func testChunkSizeConsistency() throws {
    // Feature: jarvis-listen, Property 6: Chunk Size Consistency
    // Property: For any valid configuration (sample rate, mono/stereo),
    // the generated audio chunks SHALL have size equal to:
    // (sampleRate × 0.1 seconds × 2 bytes × channelCount),
    // where channelCount is 1 for mono or 2 for stereo.
    // Validates: Requirements 5.5, 5.6
    
    let validSampleRates = [8000, 16000, 24000, 44100, 48000]
    let outputModes = [true, false]  // true = mono, false = stereo
    
    // Test all combinations of valid sample rates and output modes
    for sampleRate in validSampleRates {
        for outputMono in outputModes {
            let config = CaptureConfiguration(
                sampleRate: sampleRate,
                outputMono: outputMono,
                microphoneDeviceID: nil
            )
            
            let actualChunkSize = config.bytesPerChunk()
            
            // Calculate expected chunk size
            // Formula: sampleRate × 0.1 × 2 × channelCount
            let samplesPerChunk = (sampleRate * 100) / 1000  // 0.1 seconds = 100ms
            let bytesPerSample = 2  // s16le = 2 bytes
            let channelCount = outputMono ? 1 : 2
            let expectedChunkSize = samplesPerChunk * bytesPerSample * channelCount
            
            #expect(actualChunkSize == expectedChunkSize)
        }
    }
}

// MARK: - Specific Test Cases

@Test("16kHz stereo chunk size is 6400 bytes")
func test16kHzStereoChunkSize() throws {
    let config = CaptureConfiguration(
        sampleRate: 16000,
        outputMono: false,
        microphoneDeviceID: nil
    )
    
    let chunkSize = config.bytesPerChunk()
    
    // 16000 samples/sec × 0.1 sec × 2 bytes/sample × 2 channels = 6400 bytes
    #expect(chunkSize == 6400, "16kHz stereo should produce 6400 byte chunks")
}

@Test("16kHz mono chunk size is 3200 bytes")
func test16kHzMonoChunkSize() throws {
    let config = CaptureConfiguration(
        sampleRate: 16000,
        outputMono: true,
        microphoneDeviceID: nil
    )
    
    let chunkSize = config.bytesPerChunk()
    
    // 16000 samples/sec × 0.1 sec × 2 bytes/sample × 1 channel = 3200 bytes
    #expect(chunkSize == 3200, "16kHz mono should produce 3200 byte chunks")
}

@Test("48kHz stereo chunk size is 19200 bytes")
func test48kHzStereoChunkSize() throws {
    let config = CaptureConfiguration(
        sampleRate: 48000,
        outputMono: false,
        microphoneDeviceID: nil
    )
    
    let chunkSize = config.bytesPerChunk()
    
    // 48000 samples/sec × 0.1 sec × 2 bytes/sample × 2 channels = 19200 bytes
    #expect(chunkSize == 19200, "48kHz stereo should produce 19200 byte chunks")
}

@Test("8kHz mono chunk size is 1600 bytes")
func test8kHzMonoChunkSize() throws {
    let config = CaptureConfiguration(
        sampleRate: 8000,
        outputMono: true,
        microphoneDeviceID: nil
    )
    
    let chunkSize = config.bytesPerChunk()
    
    // 8000 samples/sec × 0.1 sec × 2 bytes/sample × 1 channel = 1600 bytes
    #expect(chunkSize == 1600, "8kHz mono should produce 1600 byte chunks")
}

@Test("44.1kHz stereo chunk size is 17640 bytes")
func test44_1kHzStereoChunkSize() throws {
    let config = CaptureConfiguration(
        sampleRate: 44100,
        outputMono: false,
        microphoneDeviceID: nil
    )
    
    let chunkSize = config.bytesPerChunk()
    
    // 44100 samples/sec × 0.1 sec × 2 bytes/sample × 2 channels = 17640 bytes
    #expect(chunkSize == 17640, "44.1kHz stereo should produce 17640 byte chunks")
}
