import Foundation
import os
@preconcurrency import AVFoundation
import CoreMedia

// MARK: - RingBuffer

/// A thread-safe circular buffer for audio synchronization.
/// Uses os_unfair_lock for high-performance locking in audio processing contexts.
///
/// - Important: This MUST be a class (not a struct) because os_unfair_lock
///   requires a stable memory address. Moving the lock in memory would corrupt it.
///   The lock is stored as a var in a heap-allocated class instance, which is safe.
final class RingBuffer {
    private var buffer: [UInt8]
    private var readIndex: Int
    private var writeIndex: Int
    private var availableBytes: Int
    private let capacity: Int
    
    /// os_unfair_lock for thread safety.
    /// - Warning: Must not be moved in memory. Safe here because RingBuffer is a final class.
    private var lock = os_unfair_lock()
    
    /// Creates a new ring buffer with the specified capacity.
    /// - Parameter capacity: Maximum number of bytes the buffer can hold.
    init(capacity: Int) {
        self.capacity = capacity
        self.buffer = [UInt8](repeating: 0, count: capacity)
        self.readIndex = 0
        self.writeIndex = 0
        self.availableBytes = 0
    }
    
    /// Writes data to the ring buffer.
    /// - Parameter data: Bytes to write to the buffer.
    /// - Returns: `true` if data fit without overflow, `false` if oldest data was discarded to make room.
    func write(_ data: [UInt8]) -> Bool {
        withLock {
            var dataToWrite = data
            var overflow = false
            
            // If data is larger than capacity, only keep the tail
            if dataToWrite.count > capacity {
                dataToWrite = Array(dataToWrite.suffix(capacity))
                overflow = true
                readIndex = 0
                writeIndex = 0
                availableBytes = 0
            } else if availableBytes + dataToWrite.count > capacity {
                overflow = true
                let excessBytes = (availableBytes + dataToWrite.count) - capacity
                readIndex = (readIndex + excessBytes) % capacity
                availableBytes -= excessBytes
            }
            
            // Write new data
            for byte in dataToWrite {
                buffer[writeIndex] = byte
                writeIndex = (writeIndex + 1) % capacity
            }
            
            availableBytes += dataToWrite.count
            
            return !overflow
        }
    }
    
    /// Reads data from the ring buffer.
    /// - Parameter count: Number of bytes to read.
    /// - Returns: Array of bytes if sufficient data available, `nil` otherwise.
    func read(_ count: Int) -> [UInt8]? {
        withLock {
            guard availableBytes >= count else {
                return nil
            }
            
            var result = [UInt8]()
            result.reserveCapacity(count)
            
            for _ in 0..<count {
                result.append(buffer[readIndex])
                readIndex = (readIndex + 1) % capacity
            }
            
            availableBytes -= count
            
            return result
        }
    }
    
    /// Returns the number of bytes currently available in the buffer.
    func availableData() -> Int {
        withLock {
            availableBytes
        }
    }
    
    /// Clears all data from the buffer.
    func clear() {
        withLock {
            readIndex = 0
            writeIndex = 0
            availableBytes = 0
        }
    }
    
    // MARK: - Private Helpers
    
    private func withLock<T>(_ body: () -> T) -> T {
        os_unfair_lock_lock(&lock)
        defer { os_unfair_lock_unlock(&lock) }
        return body()
    }
}


// MARK: - PCMConverter

/// Handles audio format conversion, synchronization, and output.
final class PCMConverter {
    private let configuration: CaptureConfiguration
    private let micBuffer: RingBuffer
    private let systemBuffer: RingBuffer
    private var micConverter: AVAudioConverter?
    private var systemConverter: AVAudioConverter?
    private var micSourceFormat: AVAudioFormat?
    private var systemSourceFormat: AVAudioFormat?
    
    /// Creates a PCMConverter with the specified configuration.
    /// - Parameter configuration: Capture configuration including sample rate and output mode.
    init(configuration: CaptureConfiguration) {
        self.configuration = configuration
        
        // Create ring buffers with 2-second capacity
        let bufferCapacity = configuration.sampleRate * 2 * 2  // 2 seconds * 2 bytes per sample
        self.micBuffer = RingBuffer(capacity: bufferCapacity)
        self.systemBuffer = RingBuffer(capacity: bufferCapacity)
    }
    
    /// Processes incoming audio data by converting it to the target format and writing to ring buffers.
    /// - Parameter audioData: Audio data from microphone or system audio source.
    /// - Throws: Errors related to audio conversion.
    func process(_ audioData: AudioData) throws {
        let convertedBytes = try convert(audioData.buffer, source: audioData.source)
        
        let buffer = audioData.source == .microphone ? micBuffer : systemBuffer
        let writeSuccess = buffer.write(convertedBytes)
        
        if !writeSuccess {
            let sourceName = audioData.source == .microphone ? "microphone" : "system audio"
            logToStderr("Warning: Ring buffer overflow for \(sourceName). Discarding oldest data.")
        }
    }
    
    /// Generates a synchronized audio chunk from both ring buffers.
    /// - Returns: Audio chunk bytes (fills with zeros if insufficient data).
    func generateChunk() -> [UInt8] {
        let samplesPerChunk = (configuration.sampleRate * CaptureConfiguration.chunkDurationMs) / 1000
        let bytesPerChannel = samplesPerChunk * 2  // 2 bytes per Int16 sample
        
        // Read from both buffers
        let micData = micBuffer.read(bytesPerChannel) ?? [UInt8](repeating: 0, count: bytesPerChannel)
        let systemData = systemBuffer.read(bytesPerChannel) ?? [UInt8](repeating: 0, count: bytesPerChannel)
        
        // Interleave or mix based on configuration
        if configuration.outputMono {
            return mix(mic: micData, system: systemData)
        } else {
            return interleave(mic: micData, system: systemData)
        }
    }
    
    /// Flushes all remaining data from ring buffers.
    /// - Returns: All remaining audio data as bytes.
    func flush() -> [UInt8] {
        var micAvailable = micBuffer.availableData()
        var systemAvailable = systemBuffer.availableData()
        
        // Align to sample boundaries (2 bytes per Int16 sample)
        micAvailable = (micAvailable / 2) * 2
        systemAvailable = (systemAvailable / 2) * 2
        
        guard micAvailable > 0 || systemAvailable > 0 else {
            return []
        }
        
        // Read all available data
        let micData = micBuffer.read(micAvailable) ?? []
        let systemData = systemBuffer.read(systemAvailable) ?? []
        
        // Pad shorter buffer with zeros
        let maxLength = max(micData.count, systemData.count)
        let paddedMic = micData + [UInt8](repeating: 0, count: maxLength - micData.count)
        let paddedSystem = systemData + [UInt8](repeating: 0, count: maxLength - systemData.count)
        
        // Interleave or mix
        if configuration.outputMono {
            return mix(mic: paddedMic, system: paddedSystem)
        } else {
            return interleave(mic: paddedMic, system: paddedSystem)
        }
    }
    
    // MARK: - Private Helpers
    
    /// Converts a CMSampleBuffer to the target format (specified sample rate, s16le, mono).
    /// - Parameters:
    ///   - sampleBuffer: Input audio sample buffer.
    ///   - source: Audio source (microphone or system audio) for converter caching.
    /// - Returns: Converted audio as byte array.
    /// - Throws: Errors related to audio conversion.
    private func convert(_ sampleBuffer: CMSampleBuffer, source: AudioData.Source) throws -> [UInt8] {
        // Bug Fix 1: Query required size for AudioBufferList first
        // ScreenCaptureKit system audio arrives as non-interleaved stereo (2 separate AudioBuffer entries)
        var bufferListSizeNeeded: Int = 0
        var blockBuffer: CMBlockBuffer?
        
        // First call: Query the required size
        let queryStatus = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
            sampleBuffer,
            bufferListSizeNeededOut: &bufferListSizeNeeded,
            bufferListOut: nil,
            bufferListSize: 0,
            blockBufferAllocator: nil,
            blockBufferMemoryAllocator: nil,
            flags: 0,
            blockBufferOut: &blockBuffer
        )
        
        guard queryStatus == noErr else {
            throw ConversionError.failedToExtractAudioBufferList(queryStatus)
        }
        
        // Dynamically allocate memory for AudioBufferList with the required size
        // AudioBufferList is a variable-length C struct, so we must allocate the exact byte count
        let audioBufferListRawPointer = UnsafeMutableRawPointer.allocate(
            byteCount: bufferListSizeNeeded,
            alignment: MemoryLayout<AudioBufferList>.alignment
        )
        let audioBufferListPointer = audioBufferListRawPointer.assumingMemoryBound(to: AudioBufferList.self)
        defer {
            audioBufferListRawPointer.deallocate()
        }
        
        // Second call: Actually extract the audio data
        let extractStatus = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
            sampleBuffer,
            bufferListSizeNeededOut: nil,
            bufferListOut: audioBufferListPointer,
            bufferListSize: bufferListSizeNeeded,
            blockBufferAllocator: nil,
            blockBufferMemoryAllocator: nil,
            flags: 0,
            blockBufferOut: &blockBuffer
        )
        
        guard extractStatus == noErr else {
            throw ConversionError.failedToExtractAudioBufferList(extractStatus)
        }
        
        // Get source format from CMSampleBuffer
        guard let formatDescription = CMSampleBufferGetFormatDescription(sampleBuffer) else {
            throw ConversionError.missingFormatDescription
        }
        
        guard let sourceFormat = CMAudioFormatDescriptionGetStreamBasicDescription(formatDescription)?.pointee else {
            throw ConversionError.invalidSourceFormat
        }
        
        var sourceStreamDescription = sourceFormat
        guard let sourceAVFormat = AVAudioFormat(streamDescription: &sourceStreamDescription) else {
            throw ConversionError.failedToCreateSourceFormat
        }
        
        // Create target format (specified sample rate, s16le, mono)
        guard let targetFormat = AVAudioFormat(
            commonFormat: .pcmFormatInt16,
            sampleRate: Double(configuration.sampleRate),
            channels: 1,
            interleaved: true
        ) else {
            throw ConversionError.failedToCreateTargetFormat
        }
        
        // Get or create cached converter for this source
        let converter: AVAudioConverter
        let cachedSourceFormat = source == .microphone ? micSourceFormat : systemSourceFormat
        let cachedConverter = source == .microphone ? micConverter : systemConverter
        
        if let cachedSourceFormat = cachedSourceFormat,
           cachedSourceFormat == sourceAVFormat,
           let cachedConverter = cachedConverter {
            // Reuse cached converter
            converter = cachedConverter
        } else {
            // Create new converter and cache it
            guard let newConverter = AVAudioConverter(from: sourceAVFormat, to: targetFormat) else {
                throw ConversionError.failedToCreateConverter
            }
            converter = newConverter
            
            // Cache the converter and source format
            if source == .microphone {
                micConverter = newConverter
                micSourceFormat = sourceAVFormat
            } else {
                systemConverter = newConverter
                systemSourceFormat = sourceAVFormat
            }
        }
        
        // Calculate frame capacity
        let sourceFrameCount = CMSampleBufferGetNumSamples(sampleBuffer)
        let ratio = targetFormat.sampleRate / sourceAVFormat.sampleRate
        let targetFrameCapacity = AVAudioFrameCount(Double(sourceFrameCount) * ratio)
        
        // Create source buffer
        guard let sourceBuffer = AVAudioPCMBuffer(pcmFormat: sourceAVFormat, frameCapacity: AVAudioFrameCount(sourceFrameCount)) else {
            throw ConversionError.failedToCreateSourceBuffer
        }
        
        // Bug Fix 2: Copy ALL audio buffers (handles non-interleaved stereo)
        // Use UnsafeMutableAudioBufferListPointer to iterate all buffers
        let sourceAudioBufferList = UnsafeMutableAudioBufferListPointer(audioBufferListPointer)
        let destAudioBufferList = UnsafeMutableAudioBufferListPointer(sourceBuffer.mutableAudioBufferList)
        
        // Copy each channel's data
        for (index, sourceAudioBuffer) in sourceAudioBufferList.enumerated() {
            guard index < destAudioBufferList.count else { break }
            
            if let sourceData = sourceAudioBuffer.mData,
               let destData = destAudioBufferList[index].mData {
                memcpy(destData, sourceData, Int(sourceAudioBuffer.mDataByteSize))
                destAudioBufferList[index].mDataByteSize = sourceAudioBuffer.mDataByteSize
            }
        }
        
        sourceBuffer.frameLength = AVAudioFrameCount(sourceFrameCount)
        
        // Create target buffer
        guard let targetBuffer = AVAudioPCMBuffer(pcmFormat: targetFormat, frameCapacity: targetFrameCapacity) else {
            throw ConversionError.failedToCreateTargetBuffer
        }
        
        // Convert
        var error: NSError?
        let inputBlock: AVAudioConverterInputBlock = { inNumPackets, outStatus in
            outStatus.pointee = .haveData
            return sourceBuffer
        }
        
        let conversionStatus = converter.convert(to: targetBuffer, error: &error, withInputFrom: inputBlock)
        
        guard conversionStatus != .error, error == nil else {
            throw ConversionError.conversionFailed(error)
        }
        
        // Extract bytes from target buffer
        guard let channelData = targetBuffer.int16ChannelData else {
            throw ConversionError.missingChannelData
        }
        
        let frameLength = Int(targetBuffer.frameLength)
        let byteCount = frameLength * 2  // 2 bytes per Int16
        
        var bytes = [UInt8](repeating: 0, count: byteCount)
        memcpy(&bytes, channelData[0], byteCount)
        
        return bytes
    }
    
    /// Interleaves two mono audio buffers into stereo (L-R-L-R).
    /// - Parameters:
    ///   - mic: Microphone audio bytes (mono, s16le).
    ///   - system: System audio bytes (mono, s16le).
    /// - Returns: Stereo interleaved bytes.
    private func interleave(mic: [UInt8], system: [UInt8]) -> [UInt8] {
        let sampleCount = mic.count / 2  // Number of Int16 samples
        var result = [UInt8]()
        result.reserveCapacity(mic.count + system.count)
        
        for i in 0..<sampleCount {
            let micIndex = i * 2
            let systemIndex = i * 2
            
            // Append mic sample (left channel)
            result.append(mic[micIndex])
            result.append(mic[micIndex + 1])
            
            // Append system sample (right channel)
            result.append(system[systemIndex])
            result.append(system[systemIndex + 1])
        }
        
        return result
    }
    
    /// Mixes two mono audio buffers into a single mono channel.
    /// - Parameters:
    ///   - mic: Microphone audio bytes (mono, s16le).
    ///   - system: System audio bytes (mono, s16le).
    /// - Returns: Mixed mono bytes.
    private func mix(mic: [UInt8], system: [UInt8]) -> [UInt8] {
        let sampleCount = mic.count / 2  // Number of Int16 samples
        var result = [UInt8]()
        result.reserveCapacity(mic.count)
        
        for i in 0..<sampleCount {
            let micIndex = i * 2
            let systemIndex = i * 2
            
            // Read Int16 samples (little-endian)
            let micSample = Int16(mic[micIndex]) | (Int16(mic[micIndex + 1]) << 8)
            let systemSample = Int16(system[systemIndex]) | (Int16(system[systemIndex + 1]) << 8)
            
            // Mix (average) and clamp
            let mixed = (Int32(micSample) + Int32(systemSample)) / 2
            let clamped = Int16(max(Int32(Int16.min), min(Int32(Int16.max), mixed)))
            
            // Write back as little-endian bytes
            result.append(UInt8(clamped & 0xFF))
            result.append(UInt8((clamped >> 8) & 0xFF))
        }
        
        return result
    }
    
    /// Logs a message to stderr.
    private func logToStderr(_ message: String) {
        FileHandle.standardError.write(Data("\(message)\n".utf8))
    }
}

// MARK: - ConversionError

enum ConversionError: Error, CustomStringConvertible {
    case failedToExtractAudioBufferList(OSStatus)
    case missingFormatDescription
    case invalidSourceFormat
    case failedToCreateSourceFormat
    case failedToCreateTargetFormat
    case failedToCreateConverter
    case failedToCreateSourceBuffer
    case failedToCreateTargetBuffer
    case conversionFailed(Error?)
    case missingChannelData
    
    var description: String {
        switch self {
        case .failedToExtractAudioBufferList(let status):
            return "Failed to extract audio buffer list: OSStatus \(status)"
        case .missingFormatDescription:
            return "Missing format description"
        case .invalidSourceFormat:
            return "Invalid source format"
        case .failedToCreateSourceFormat:
            return "Failed to create source AVAudioFormat"
        case .failedToCreateTargetFormat:
            return "Failed to create target AVAudioFormat"
        case .failedToCreateConverter:
            return "Failed to create AVAudioConverter"
        case .failedToCreateSourceBuffer:
            return "Failed to create source PCM buffer"
        case .failedToCreateTargetBuffer:
            return "Failed to create target PCM buffer"
        case .conversionFailed(let error):
            return "Audio conversion failed: \(error?.localizedDescription ?? "unknown error")"
        case .missingChannelData:
            return "Missing channel data in converted buffer"
        }
    }
}
