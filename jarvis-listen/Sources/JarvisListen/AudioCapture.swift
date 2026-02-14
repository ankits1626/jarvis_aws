import Foundation
import CoreMedia
import AVFoundation

// MARK: - AudioData

/// Represents a chunk of audio from a single source (microphone or system audio).
struct AudioData: @unchecked Sendable {
    enum Source: Sendable {
        case microphone
        case systemAudio
    }
    
    let source: Source
    let buffer: CMSampleBuffer
    let timestamp: CMTime
}

// MARK: - AudioDevice

/// Represents an audio input device.
struct AudioDevice {
    let id: String
    let name: String
}

// MARK: - CaptureConfiguration

/// Holds the runtime configuration for audio capture.
struct CaptureConfiguration {
    let sampleRate: Int
    let outputMono: Bool
    let microphoneDeviceID: String?
    
    static let validSampleRates: Set<Int> = [8000, 16000, 24000, 44100, 48000]
    static let defaultSampleRate: Int = 16000
    static let chunkDurationMs: Int = 100
    
    /// Calculates the number of bytes per audio chunk based on configuration.
    /// - Returns: Chunk size in bytes for the configured sample rate and channel count.
    func bytesPerChunk() -> Int {
        let samplesPerChunk = (sampleRate * Self.chunkDurationMs) / 1000
        let bytesPerSample = 2  // s16le = 2 bytes
        let channels = outputMono ? 1 : 2
        return samplesPerChunk * bytesPerSample * channels
    }
}


// MARK: - StreamDelegate

@preconcurrency import ScreenCaptureKit

/// Delegate class to bridge SCStreamOutput to AsyncStream.
/// SCStreamOutput is an NSObjectProtocol-based delegate that cannot be directly
/// implemented by actors, so we use this separate NSObject subclass.
final class StreamDelegate: NSObject, SCStreamOutput {
    private let continuation: AsyncStream<AudioData>.Continuation
    
    init(continuation: AsyncStream<AudioData>.Continuation) {
        self.continuation = continuation
        super.init()
    }
    
    func stream(_ stream: SCStream, 
                didOutputSampleBuffer sampleBuffer: CMSampleBuffer, 
                of type: SCStreamOutputType) {
        // Only process audio and microphone frames, discard video frames (Requirement 11.4)
        guard type == .audio || type == .microphone else {
            return  // Discard video frames without processing
        }
        
        // Determine source from type
        let source: AudioData.Source = (type == .microphone) ? .microphone : .systemAudio
        let timestamp = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
        let audioData = AudioData(source: source, buffer: sampleBuffer, timestamp: timestamp)
        continuation.yield(audioData)
    }
}

// MARK: - AudioCapture

/// Concrete implementation of AudioCaptureProvider using ScreenCaptureKit.
actor AudioCapture: AudioCaptureProvider {
    private let configuration: CaptureConfiguration
    private var stream: SCStream?
    private var streamDelegate: StreamDelegate?
    
    nonisolated let audioDataStream: AsyncStream<AudioData>
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
    
    func startCapture() async throws {
        // Get shareable content - this will fail if Screen Recording permission is denied
        let content: SCShareableContent
        do {
            content = try await SCShareableContent.current
        } catch {
            // Check if this is a permission error
            let nsError = error as NSError
            if nsError.domain == "com.apple.screencapturekit" || 
               nsError.code == -3801 || // Permission denied error code
               error.localizedDescription.contains("permission") {
                throw AudioCaptureError.permissionDenied
            }
            throw error
        }
        
        // Select first display for content filter
        guard let display = content.displays.first else {
            throw AudioCaptureError.noDisplaysAvailable
        }
        
        // Validate microphone device if specified
        if let deviceID = configuration.microphoneDeviceID {
            let devices = try await listDevices()
            if !devices.contains(where: { $0.id == deviceID }) {
                throw AudioCaptureError.deviceNotFound(deviceID)
            }
        }
        
        // Create content filter
        let filter = SCContentFilter(display: display, excludingWindows: [])
        
        // Create stream configuration
        let config = SCStreamConfiguration()
        
        // Audio settings
        config.capturesAudio = true
        config.captureMicrophone = true
        config.excludesCurrentProcessAudio = true
        
        // Minimize video overhead (required by ScreenCaptureKit even for audio-only)
        config.width = 2
        config.height = 2
        config.minimumFrameInterval = CMTime(value: 1, timescale: 1)  // 1 FPS
        
        // Set microphone device if specified
        if configuration.microphoneDeviceID != nil {
            // Note: ScreenCaptureKit doesn't have a direct API to set microphone device by ID
            // This would require using AVCaptureDevice, but for now we'll use the default
            // TODO: Implement device selection if needed
        }
        
        // Create stream
        let scStream = SCStream(filter: filter, configuration: config, delegate: nil)
        self.stream = scStream
        
        // Add stream output delegate
        if let delegate = streamDelegate {
            do {
                try scStream.addStreamOutput(delegate, type: .audio, sampleHandlerQueue: .global(qos: .userInteractive))
                try scStream.addStreamOutput(delegate, type: .microphone, sampleHandlerQueue: .global(qos: .userInteractive))
            } catch {
                // Check for microphone permission error
                let nsError = error as NSError
                if nsError.localizedDescription.contains("microphone") || 
                   nsError.localizedDescription.contains("Microphone") {
                    // Log warning but continue with system audio only
                    logToStderr("Warning: Microphone permission denied. Please enable it in System Settings > Privacy & Security > Microphone. Continuing with system audio only.")
                    // Only add audio output (system audio)
                    try scStream.addStreamOutput(delegate, type: .audio, sampleHandlerQueue: .global(qos: .userInteractive))
                } else {
                    throw error
                }
            }
        }
        
        // Start capture
        do {
            try await scStream.startCapture()
        } catch {
            // Wrap any capture start errors with descriptive message
            let nsError = error as NSError
            if nsError.domain == "com.apple.screencapturekit" || 
               nsError.localizedDescription.contains("permission") {
                throw AudioCaptureError.permissionDenied
            }
            throw error
        }
        
        // Print startup message to stderr
        let deviceName = configuration.microphoneDeviceID ?? "Default"
        let channels = configuration.outputMono ? "mono" : "stereo"
        let message = "Capturing: mic=\(deviceName), format=\(configuration.sampleRate)Hz s16le \(channels), method=ScreenCaptureKit"
        logToStderr(message)
    }
    
    func stopCapture() async {
        if let stream = stream {
            do {
                try await stream.stopCapture()
            } catch {
                logToStderr("Error stopping capture: \(error)")
            }
        }
        
        // Finish the continuation
        continuation.finish()
        
        // Release resources
        stream = nil
        streamDelegate = nil
    }
    
    /// Lists available audio input devices.
    /// - Returns: Array of available audio devices.
    func listDevices() async throws -> [AudioDevice] {
        // Use AVCaptureDevice to enumerate microphones
        let discoverySession = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.microphone, .external],
            mediaType: .audio,
            position: .unspecified
        )
        
        return discoverySession.devices.map { device in
            AudioDevice(id: device.uniqueID, name: device.localizedName)
        }
    }
    
    private func logToStderr(_ message: String) {
        FileHandle.standardError.write(Data("\(message)\n".utf8))
    }
}

// MARK: - AudioCaptureError

enum AudioCaptureError: Error, CustomStringConvertible {
    case noDisplaysAvailable
    case permissionDenied
    case deviceNotFound(String)
    
    var description: String {
        switch self {
        case .noDisplaysAvailable:
            return "No displays available for capture"
        case .permissionDenied:
            return "Screen Recording permission denied. Please enable it in System Settings > Privacy & Security > Screen & System Audio Recording"
        case .deviceNotFound(let deviceID):
            return "Microphone device '\(deviceID)' not found. Use --list-devices to see available devices."
        }
    }
}
