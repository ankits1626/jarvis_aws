import Foundation

// MARK: - Exit Codes

enum ExitCode {
    static let success: Int32 = 0
    static let configurationError: Int32 = 1
    static let permissionDenied: Int32 = 1
    static let captureFailure: Int32 = 1
}

// MARK: - Logging Utilities

/// Writes a message to stderr.
/// - Parameter message: The message to write.
func logToStderr(_ message: String) {
    let data = Data((message + "\n").utf8)
    FileHandle.standardError.write(data)
}

/// Writes an error message to stderr with "Error: " prefix.
/// - Parameter message: The error message to write.
func logError(_ message: String) {
    logToStderr("Error: \(message)")
}

/// Writes a warning message to stderr with "Warning: " prefix.
/// - Parameter message: The warning message to write.
func logWarning(_ message: String) {
    logToStderr("Warning: \(message)")
}

// MARK: - Permission Error Messages

/// Returns the error message for denied Screen Recording permission.
func screenRecordingPermissionDeniedMessage() -> String {
    return "Screen Recording permission denied. Please enable it in System Settings > Privacy & Security > Screen & System Audio Recording"
}

/// Returns the warning message for denied Microphone permission.
func microphonePermissionDeniedMessage() -> String {
    return "Microphone permission denied. Please enable it in System Settings > Privacy & Security > Microphone. Continuing with system audio only."
}

/// Returns the error message for capture start failure.
/// - Parameter error: The underlying error.
/// - Returns: A descriptive error message.
func captureStartFailureMessage(error: Error) -> String {
    return "Failed to start audio capture: \(error.localizedDescription)"
}

/// Returns the error message for invalid microphone device.
/// - Parameter deviceID: The invalid device ID.
/// - Returns: A descriptive error message.
func invalidMicrophoneDeviceMessage(deviceID: String) -> String {
    return "Microphone device '\(deviceID)' not found. Use --list-devices to see available devices."
}

// MARK: - SignalHandler

/// Manages Unix signal handling for graceful shutdown using DispatchSourceSignal.
/// This approach is async-signal-safe and uses a background queue to avoid
/// blocking on the MainActor executor.
class SignalHandler {
    private var _shouldShutdown = false
    private var lock = os_unfair_lock()
    private var sigintSource: DispatchSourceSignal?
    private var sigtermSource: DispatchSourceSignal?
    private let signalQueue = DispatchQueue(label: "com.jarvis.signals")
    
    /// Thread-safe getter for shutdown flag
    var shouldShutdown: Bool {
        os_unfair_lock_lock(&lock)
        defer { os_unfair_lock_unlock(&lock) }
        return _shouldShutdown
    }
    
    /// Sets up signal handlers for SIGINT, SIGTERM, and SIGPIPE.
    func setup() {
        // Disable default signal handling for SIGINT and SIGTERM
        // This prevents the process from being terminated immediately
        Darwin.signal(SIGINT, SIG_IGN)
        Darwin.signal(SIGTERM, SIG_IGN)
        
        // Ignore SIGPIPE (broken pipe) - just let it be ignored silently
        Darwin.signal(SIGPIPE, SIG_IGN)
        
        // Create DispatchSource for SIGINT on background queue
        sigintSource = DispatchSource.makeSignalSource(signal: SIGINT, queue: signalQueue)
        sigintSource?.setEventHandler { [weak self] in
            self?.setShutdown()
        }
        sigintSource?.resume()
        
        // Create DispatchSource for SIGTERM on background queue
        sigtermSource = DispatchSource.makeSignalSource(signal: SIGTERM, queue: signalQueue)
        sigtermSource?.setEventHandler { [weak self] in
            self?.setShutdown()
        }
        sigtermSource?.resume()
        
        logToStderr("DEBUG: SignalHandler.setup() complete - SIGINT/SIGTERM dispatch sources active")
    }
    
    /// Thread-safe setter for shutdown flag
    private func setShutdown() {
        logToStderr("DEBUG: Signal received, setting shouldShutdown=true")
        os_unfair_lock_lock(&lock)
        _shouldShutdown = true
        os_unfair_lock_unlock(&lock)
    }
    
    deinit {
        // Cancel dispatch sources on cleanup
        sigintSource?.cancel()
        sigtermSource?.cancel()
    }
}

// MARK: - ParsedArguments

struct ParsedArguments {
    enum Action {
        case capture(CaptureConfiguration, outputPath: String?)
        case listDevices
        case showHelp
    }
    
    let action: Action
}

// MARK: - ArgumentParser

struct ArgumentParser {
    enum ParseError: Error, CustomStringConvertible {
        case invalidFlag(String)
        case invalidSampleRate(String)
        case missingSampleRateValue
        case missingMicDeviceValue
        case missingOutputValue
        
        var description: String {
            switch self {
            case .invalidFlag(let flag):
                return "Error: Unknown flag '\(flag)'. Use --help for usage information."
            case .invalidSampleRate(let value):
                let validRates = CaptureConfiguration.validSampleRates.sorted().map(String.init).joined(separator: ", ")
                return "Error: Invalid sample rate '\(value)'. Valid values: \(validRates)"
            case .missingSampleRateValue:
                return "Error: --sample-rate requires a value"
            case .missingMicDeviceValue:
                return "Error: --mic-device requires a value"
            case .missingOutputValue:
                return "Error: --output requires a file path"
            }
        }
    }
    
    static func parse(_ arguments: [String]) throws -> ParsedArguments {
        // Skip the first argument (program name)
        let args = Array(arguments.dropFirst())
        
        // Check for help or list-devices first
        if args.contains("--help") {
            return ParsedArguments(action: .showHelp)
        }
        
        if args.contains("--list-devices") {
            return ParsedArguments(action: .listDevices)
        }
        
        // Parse capture configuration
        var outputMono = false
        var sampleRate = CaptureConfiguration.defaultSampleRate
        var microphoneDeviceID: String? = nil
        var outputPath: String? = nil
        
        var i = 0
        while i < args.count {
            let arg = args[i]
            
            switch arg {
            case "--mono":
                outputMono = true
                i += 1
                
            case "--sample-rate":
                guard i + 1 < args.count else {
                    throw ParseError.missingSampleRateValue
                }
                let rateString = args[i + 1]
                guard let rate = Int(rateString),
                      CaptureConfiguration.validSampleRates.contains(rate) else {
                    throw ParseError.invalidSampleRate(rateString)
                }
                sampleRate = rate
                i += 2
                
            case "--mic-device":
                guard i + 1 < args.count else {
                    throw ParseError.missingMicDeviceValue
                }
                microphoneDeviceID = args[i + 1]
                i += 2
                
            case "--output":
                guard i + 1 < args.count else {
                    throw ParseError.missingOutputValue
                }
                outputPath = args[i + 1]
                i += 2
                
            default:
                if arg.hasPrefix("--") {
                    throw ParseError.invalidFlag(arg)
                }
                // Unknown non-flag argument
                throw ParseError.invalidFlag(arg)
            }
        }
        
        let config = CaptureConfiguration(
            sampleRate: sampleRate,
            outputMono: outputMono,
            microphoneDeviceID: microphoneDeviceID
        )
        
        return ParsedArguments(action: .capture(config, outputPath: outputPath))
    }
}

// MARK: - Usage

func printUsage() {
    let usage = """
    JarvisListen - Capture system audio and microphone to stdout
    
    USAGE:
        JarvisListen [options]
    
    OPTIONS:
        --mono              Mix both streams into single channel
        --sample-rate N     Output sample rate (default: 16000)
                           Valid values: 8000, 16000, 24000, 44100, 48000
        --mic-device ID     Microphone device ID (default: system default)
        --output PATH       Write PCM data to file instead of stdout
        --list-devices      List available microphones and exit
        --help              Show this usage information
    
    OUTPUT:
        Stereo PCM data to stdout (Channel 0=mic, Channel 1=system audio)
        Or to file if --output is specified
        All log messages to stderr
    
    EXAMPLES:
        JarvisListen > output.pcm
        JarvisListen --output recording.pcm
        JarvisListen --mono --sample-rate 48000 | transcriber
        JarvisListen --list-devices
    """
    
    logToStderr(usage)
}

// MARK: - Main Entry Point

/// Handles the capture action - runs the audio capture loop.
@MainActor
func handleCapture(config: CaptureConfiguration, outputPath: String?) async throws {
    // Create AudioCapture instance
    let audioCapture = AudioCapture(configuration: config)
    
    // Create PCMConverter instance
    let pcmConverter = PCMConverter(configuration: config)
    
    // Determine output file handle
    let outputHandle: FileHandle
    if let path = outputPath {
        // Create or open file for writing
        let fileURL = URL(fileURLWithPath: path)
        if !FileManager.default.fileExists(atPath: path) {
            FileManager.default.createFile(atPath: path, contents: nil)
        }
        guard let handle = try? FileHandle(forWritingTo: fileURL) else {
            logError("Failed to open output file: \(path)")
            exit(ExitCode.configurationError)
        }
        outputHandle = handle
        logToStderr("DEBUG: Output file opened: \(path)")
    } else {
        // Use stdout
        outputHandle = FileHandle.standardOutput
        logToStderr("DEBUG: Using stdout for output")
    }
    
    // Shutdown flag
    let signalHandler = SignalHandler()
    signalHandler.setup()
    
    logToStderr("DEBUG: About to call audioCapture.startCapture()")
    
    // Start audio capture
    do {
        try await audioCapture.startCapture()
        logToStderr("DEBUG: audioCapture.startCapture() succeeded")
    } catch let error as AudioCaptureError {
        // Clean up continuation before exiting
        await audioCapture.stopCapture()
        
        // Handle specific AudioCapture errors with helpful messages
        switch error {
        case .permissionDenied:
            logError(screenRecordingPermissionDeniedMessage())
            exit(ExitCode.permissionDenied)
        case .deviceNotFound(let deviceID):
            logError(invalidMicrophoneDeviceMessage(deviceID: deviceID))
            exit(ExitCode.configurationError)
        case .noDisplaysAvailable:
            logError("No displays available for capture")
            exit(ExitCode.captureFailure)
        }
    } catch {
        // Clean up continuation before exiting
        await audioCapture.stopCapture()
        
        // Handle other errors
        logError(captureStartFailureMessage(error: error))
        exit(ExitCode.captureFailure)
    }
    
    // Create async task for processing AudioData stream
    let processingTask = Task {
        var processedCount = 0
        for await audioData in audioCapture.audioDataStream {
            do {
                try pcmConverter.process(audioData)
                processedCount += 1
                if processedCount == 1 {
                    logToStderr("DEBUG: First audio data received from \(audioData.source)")
                }
            } catch {
                logWarning("Audio conversion failed for \(audioData.source): \(error). Skipping buffer.")
            }
            
            // Check shutdown flag
            if signalHandler.shouldShutdown {
                break
            }
        }
        logToStderr("DEBUG: Processing task exiting, processed \(processedCount) buffers")
    }
    
    // Create async task for chunk generation timer (100ms loop)
    let chunkTask = Task {
        var chunkCount = 0
        while !signalHandler.shouldShutdown {
            // Sleep for 100ms
            try? await Task.sleep(for: .milliseconds(100))
            
            // Generate and write chunk
            let chunk = pcmConverter.generateChunk()
            if !chunk.isEmpty {
                let data = Data(chunk)
                outputHandle.write(data)
                chunkCount += 1
                if chunkCount == 1 {
                    logToStderr("DEBUG: First chunk written (\(data.count) bytes)")
                }
                if chunkCount % 100 == 0 {
                    logToStderr("DEBUG: Chunk task alive, \(chunkCount) chunks written so far")
                }
            }
        }
        logToStderr("DEBUG: Chunk task exiting after \(chunkCount) chunks")
    }
    
    logToStderr("DEBUG: Entering main shutdown-check loop")
    
    // Wait for shutdown signal
    while !signalHandler.shouldShutdown {
        try? await Task.sleep(for: .milliseconds(100))
    }
    
    logToStderr("DEBUG: shouldShutdown detected, starting shutdown sequence")
    
    // Shutdown sequence
    // 1. Stop audio capture (calls continuation.finish())
    logToStderr("DEBUG: [Shutdown 1/5] Calling audioCapture.stopCapture()")
    await audioCapture.stopCapture()
    
    logToStderr("DEBUG: [Shutdown 2/5] stopCapture() complete, waiting for processingTask")
    // 2. Wait for processingTask to drain remaining buffered audio
    // When stopCapture() calls continuation.finish(), AsyncStream allows
    // buffered elements to drain. We must wait for this to complete before
    // cancelling, otherwise buffered audio data will be lost.
    _ = await processingTask.value
    
    logToStderr("DEBUG: [Shutdown 3/5] processingTask drained")
    // 3. Cancel chunk generation task
    chunkTask.cancel()
    
    logToStderr("DEBUG: [Shutdown 4/5] chunkTask cancelled")
    // 4. Flush remaining buffers to output file/stdout
    let flushedData = pcmConverter.flush()
    logToStderr("DEBUG: [Shutdown 5/5] Flushed \(flushedData.count) bytes")
    if !flushedData.isEmpty {
        outputHandle.write(Data(flushedData))
    }
    
    logToStderr("DEBUG: Shutdown complete, exiting with success")
    // 5. Exit successfully
    exit(ExitCode.success)
}

// MARK: - Main Entry Point

/// Handles the --list-devices action.
func handleListDevices() async throws {
    // Create a temporary AudioCapture instance to list devices
    let tempConfig = CaptureConfiguration(
        sampleRate: CaptureConfiguration.defaultSampleRate,
        outputMono: false,
        microphoneDeviceID: nil
    )
    let audioCapture = AudioCapture(configuration: tempConfig)
    
    // Get available devices
    do {
        let devices = try await audioCapture.listDevices()
        
        // Format and print to stdout (as per requirement 2.4)
        for device in devices {
            print("\(device.id): \(device.name)")
        }
        
        // Clean up: stop capture to finish the continuation and prevent leak
        await audioCapture.stopCapture()
    } catch {
        // Clean up even on error to prevent continuation leak
        await audioCapture.stopCapture()
        logError("Failed to list devices: \(error.localizedDescription)")
        exit(ExitCode.captureFailure)
    }
}

// Main execution
Task {
    do {
        let parsed = try ArgumentParser.parse(CommandLine.arguments)
        
        switch parsed.action {
        case .showHelp:
            printUsage()
            exit(ExitCode.success)
            
        case .listDevices:
            try await handleListDevices()
            exit(ExitCode.success)
            
        case .capture(let config, let outputPath):
            try await handleCapture(config: config, outputPath: outputPath)
        }
    } catch let error as ArgumentParser.ParseError {
        logToStderr(error.description)
        exit(ExitCode.configurationError)
    } catch {
        logError(error.localizedDescription)
        exit(ExitCode.configurationError)
    }
}

// Keep the program running
RunLoop.main.run()
