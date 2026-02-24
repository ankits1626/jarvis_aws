// main.swift - Entry point, signal handling, and server loop
//
// IntelligenceKit is a macOS Swift server that provides a generic gateway to
// Apple's Foundation Models framework. It runs as a persistent local server
// over stdin/stdout using newline-delimited JSON (NDJSON).
//
// Architecture:
// - Signal handling via DispatchSource for async-signal-safe shutdown
// - NDJSON protocol for request/response communication
// - Session management with automatic idle timeout
// - Generic message execution with guided generation
//
// Trade-offs:
// - readLine() blocks, so signal handler flag won't be checked until next line arrives
// - Normal operation: Tauri sends "shutdown" command before SIGTERM (graceful)
// - Abnormal termination: Signal handler sets flag, but may not be checked immediately
// - This is acceptable because Swift's ARC handles cleanup automatically
//
import Foundation
import FoundationModels
import Darwin

// MARK: - Signal Handler

/// Thread-safe signal handler using DispatchSource.
///
/// Uses DispatchSource.makeSignalSource for async-signal-safe handling of SIGINT,
/// SIGTERM, and SIGPIPE. The handler sets a shutdown flag on a background queue,
/// which is checked in the main read loop.
///
/// Important trade-off: Since readLine() blocks, the shutdown flag won't be checked
/// until the next line arrives. In normal operation, Tauri sends a "shutdown" command
/// before SIGTERM, allowing graceful cleanup. The signal handler is a safety net for
/// abnormal termination (e.g., parent process dies).
///
/// Marked `@unchecked Sendable` because the class uses os_unfair_lock for thread safety,
/// which is not automatically recognized by Swift's concurrency system.
final class SignalHandler: @unchecked Sendable {
    private var _shouldShutdown = false
    private var lock = os_unfair_lock()
    private var sigintSource: DispatchSourceSignal?
    private var sigtermSource: DispatchSourceSignal?
    private let signalQueue = DispatchQueue(label: "com.intelligencekit.signals")
    
    /// Thread-safe access to the shutdown flag.
    ///
    /// Uses os_unfair_lock to ensure atomic reads across threads.
    var shouldShutdown: Bool {
        os_unfair_lock_lock(&lock)
        defer { os_unfair_lock_unlock(&lock) }
        return _shouldShutdown
    }
    
    /// Sets up signal handlers for SIGINT, SIGTERM, and SIGPIPE.
    ///
    /// Disables default signal handling and creates DispatchSource handlers
    /// that set the shutdown flag when signals are received.
    func setup() {
        // Disable default signal handling
        Darwin.signal(SIGINT, SIG_IGN)
        Darwin.signal(SIGTERM, SIG_IGN)
        Darwin.signal(SIGPIPE, SIG_IGN)
        
        // Create DispatchSource for SIGINT
        sigintSource = DispatchSource.makeSignalSource(signal: SIGINT, queue: signalQueue)
        sigintSource?.setEventHandler { [weak self] in
            logToStderr("Received SIGINT")
            self?.setShutdown()
        }
        sigintSource?.resume()
        
        // Create DispatchSource for SIGTERM
        sigtermSource = DispatchSource.makeSignalSource(signal: SIGTERM, queue: signalQueue)
        sigtermSource?.setEventHandler { [weak self] in
            logToStderr("Received SIGTERM")
            self?.setShutdown()
        }
        sigtermSource?.resume()
    }
    
    /// Thread-safe setter for the shutdown flag.
    ///
    /// Called by signal handlers on the background queue when SIGINT or SIGTERM is received.
    private func setShutdown() {
        os_unfair_lock_lock(&lock)
        _shouldShutdown = true
        os_unfair_lock_unlock(&lock)
    }
    
    /// Cancels signal sources on deinitialization.
    deinit {
        sigintSource?.cancel()
        sigtermSource?.cancel()
    }
}

// Global signal handler instance (nonisolated to avoid actor isolation issues)
nonisolated(unsafe) var signalHandler: SignalHandler?

/// Sets up the global signal handler.
///
/// Must be called on the main actor before starting the server loop.
@MainActor
func setupSignalHandlers() {
    signalHandler = SignalHandler()
    signalHandler?.setup()
}

// MARK: - Server Loop

/// Writes a response as NDJSON to stdout.
///
/// - Parameter response: The response to encode and write
///
/// Encodes the response to JSON, appends a newline, and writes to stdout.
/// If encoding fails, logs an error to stderr but does not crash.
func writeResponse(_ response: Response) {
    guard let data = try? JSONEncoder().encode(response) else {
        logError("Failed to encode response")
        return
    }
    FileHandle.standardOutput.write(data + Data("\n".utf8))
}

/// Main server loop - reads NDJSON from stdin and processes commands.
///
/// The server operates as a request-response pipeline:
/// 1. Read a line from stdin (blocks until line arrives)
/// 2. Skip empty lines
/// 3. Check signal handler flag for shutdown
/// 4. Parse line as JSON Request
/// 5. Route request to appropriate handler
/// 6. Write response as NDJSON to stdout
/// 7. Check if shutdown command was received
/// 8. Repeat until EOF or shutdown
///
/// Error handling:
/// - Invalid JSON → Returns `{"ok":false,"error":"invalid_json"}` and continues
/// - Empty lines → Skipped silently
/// - EOF → Triggers graceful shutdown
/// - Signal received → Triggers immediate shutdown
///
/// After shutdown (command or EOF), closes all sessions and exits cleanly.
func runServer() async {
    let sessionManager = SessionManager()
    let messageExecutor = MessageExecutor()
    let availabilityChecker = AvailabilityChecker()
    let router = CommandRouter(
        sessionManager: sessionManager,
        messageExecutor: messageExecutor,
        availabilityChecker: availabilityChecker
    )
    
    var shouldShutdown = false
    
    while let line = readLine(), !shouldShutdown {
        // Skip empty lines
        guard !line.isEmpty else { continue }
        
        // Check signal handler flag
        if let handler = signalHandler, handler.shouldShutdown {
            logToStderr("Signal received, shutting down")
            await sessionManager.closeAllSessions()
            exit(0)
        }
        
        // Parse request
        guard let data = line.data(using: .utf8) else {
            writeResponse(.error(ErrorResponse(error: "invalid_json")))
            continue
        }
        
        do {
            let request = try JSONDecoder().decode(Request.self, from: data)
            let response = await router.route(request)
            writeResponse(response)
            
            // Check if this was a shutdown command
            if request.command == "shutdown" {
                shouldShutdown = true
            }
        } catch {
            writeResponse(.error(ErrorResponse(error: "invalid_json")))
        }
    }
    
    // Clean exit after shutdown command or EOF
    await sessionManager.closeAllSessions()
    logToStderr("Shutdown complete")
    exit(0)
}

// MARK: - Entry Point (top-level code)

// Set up signal handlers
setupSignalHandlers()

// Log startup
logToStderr("IntelligenceKit server ready")

// Create async context for the read loop
Task {
    await runServer()
}

// Keep the process alive
RunLoop.main.run()
