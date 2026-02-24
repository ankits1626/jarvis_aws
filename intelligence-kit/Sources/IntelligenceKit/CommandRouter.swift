import Foundation
import FoundationModels

// MARK: - CommandRouter

/// Routes incoming NDJSON commands to appropriate handlers.
///
/// CommandRouter is the central dispatch point for all server operations.
/// It validates commands, delegates to specialized components, and returns
/// structured responses.
///
/// The router handles five command types:
/// - `open-session`: Create a new LanguageModelSession
/// - `message`: Execute a prompt against an existing session
/// - `close-session`: Explicitly close a session
/// - `check-availability`: Check if Foundation Models is available
/// - `shutdown`: Gracefully shut down the server
///
/// Example usage:
/// ```swift
/// let router = CommandRouter(
///     sessionManager: sessionManager,
///     messageExecutor: executor,
///     availabilityChecker: checker
/// )
/// let response = await router.route(request)
/// ```
struct CommandRouter {
    let sessionManager: SessionManager
    let messageExecutor: MessageExecutor
    let availabilityChecker: AvailabilityChecker
    
    /// Routes a request to the appropriate handler based on the command field.
    ///
    /// - Parameter request: The incoming request from the client
    /// - Returns: A response to send back to the client
    ///
    /// If the command is not recognized, returns an error response with
    /// `error: "unknown_command"`.
    func route(_ request: Request) async -> Response {
        guard let command = Command(rawValue: request.command) else {
            return .error(ErrorResponse(error: "unknown_command"))
        }
        
        switch command {
        case .openSession:
            return await handleOpenSession(request)
        case .message:
            return await handleMessage(request)
        case .closeSession:
            return await handleCloseSession(request)
        case .checkAvailability:
            return handleCheckAvailability()
        case .shutdown:
            return await handleShutdown()
        }
    }
    
    // MARK: - Command Handlers
    
    /// Handles the `open-session` command.
    ///
    /// Creates a new LanguageModelSession with optional instructions.
    ///
    /// - Parameter request: The request containing optional `instructions` field
    /// - Returns: Success response with `sessionId`, or error response
    ///
    /// Success response format:
    /// ```json
    /// {"ok": true, "session_id": "<uuid>"}
    /// ```
    private func handleOpenSession(_ request: Request) async -> Response {
        do {
            let sessionId = try await sessionManager.openSession(
                instructions: request.instructions
            )
            return .success(SuccessResponse(sessionId: sessionId, result: nil))
        } catch {
            return .error(ErrorResponse(error: "failed_to_open_session: \(error)"))
        }
    }
    
    /// Handles the `message` command.
    ///
    /// Executes a prompt against an existing session with guided generation.
    ///
    /// - Parameter request: The request containing required fields:
    ///   - `sessionId`: The session to use
    ///   - `prompt`: What to do (e.g., "Generate 3-5 topic tags")
    ///   - `content`: Input text to process
    ///   - `outputFormat`: Either "string_list" or "text"
    /// - Returns: Success response with `result`, or error response
    ///
    /// Validates all required fields and returns specific error codes:
    /// - `session_id_required`: Missing session ID
    /// - `prompt_required`: Missing or empty prompt
    /// - `content_required`: Missing or empty content
    /// - `output_format_required`: Missing output format
    /// - `session_not_found`: Session ID doesn't exist
    /// - `unknown_output_format`: Format not "string_list" or "text"
    /// - `guardrail_blocked`: Content safety filter blocked the request
    /// - `model_unavailable`: Foundation Models became unavailable
    ///
    /// Logs execution time and content length to stderr for monitoring.
    private func handleMessage(_ request: Request) async -> Response {
        // Validate required fields
        guard let sessionId = request.sessionId else {
            return .error(ErrorResponse(error: "session_id_required"))
        }
        guard let prompt = request.prompt, !prompt.isEmpty else {
            return .error(ErrorResponse(error: "prompt_required"))
        }
        guard let content = request.content, !content.isEmpty else {
            return .error(ErrorResponse(error: "content_required"))
        }
        guard let outputFormat = request.outputFormat else {
            return .error(ErrorResponse(error: "output_format_required"))
        }
        
        // Get session
        guard let session = await sessionManager.getSession(sessionId) else {
            return .error(ErrorResponse(error: "session_not_found"))
        }
        
        // Execute message with timing
        let startTime = CFAbsoluteTimeGetCurrent()
        
        do {
            let result = try await messageExecutor.execute(
                session: session,
                prompt: prompt,
                content: content,
                outputFormat: outputFormat
            )
            
            let elapsed = CFAbsoluteTimeGetCurrent() - startTime
            logToStderr("Message processed: session=\(sessionId), format=\(outputFormat), content_length=\(content.count), time=\(String(format: "%.2f", elapsed))s")
            
            return .success(SuccessResponse(sessionId: nil, result: result))
        } catch ExecutionError.unknownOutputFormat {
            return .error(ErrorResponse(error: "unknown_output_format"))
        } catch ExecutionError.guardrailBlocked {
            return .error(ErrorResponse(error: "guardrail_blocked"))
        } catch ExecutionError.modelUnavailable {
            return .error(ErrorResponse(error: "model_unavailable"))
        } catch {
            return .error(ErrorResponse(error: "execution_failed: \(error)"))
        }
    }
    
    /// Handles the `close-session` command.
    ///
    /// Explicitly closes a session and frees its resources.
    ///
    /// - Parameter request: The request containing required `sessionId` field
    /// - Returns: Success response, or error if session ID is missing
    ///
    /// Note: If the session doesn't exist, this still returns success (idempotent).
    /// Sessions are also automatically closed after 120 seconds of inactivity.
    private func handleCloseSession(_ request: Request) async -> Response {
        guard let sessionId = request.sessionId else {
            return .error(ErrorResponse(error: "session_id_required"))
        }
        
        await sessionManager.closeSession(sessionId)
        return .success(SuccessResponse(sessionId: nil, result: nil))
    }
    
    /// Handles the `check-availability` command.
    ///
    /// Checks if Foundation Models is available on this system.
    ///
    /// - Returns: Availability response with `available` boolean and optional `reason`
    ///
    /// Response format when available:
    /// ```json
    /// {"ok": true, "available": true}
    /// ```
    ///
    /// Response format when unavailable:
    /// ```json
    /// {"ok": true, "available": false, "reason": "Apple Intelligence not enabled by user"}
    /// ```
    ///
    /// This is a synchronous operation that completes immediately.
    private func handleCheckAvailability() -> Response {
        let (available, reason) = availabilityChecker.check()
        return .availability(AvailabilityResponse(available: available, reason: reason))
    }
    
    /// Handles the `shutdown` command.
    ///
    /// Closes all active sessions and prepares for graceful shutdown.
    ///
    /// - Returns: Success response
    ///
    /// After returning this response and writing it to stdout, the main loop
    /// checks the shutdown flag and exits cleanly. This ensures the client
    /// receives confirmation before the server terminates.
    private func handleShutdown() async -> Response {
        await sessionManager.closeAllSessions()
        logToStderr("Shutting down")
        // Note: After returning this response and writing it to stdout,
        // the main loop should exit. The shutdown command sets a flag
        // that breaks the read loop after the response is written.
        return .success(SuccessResponse(sessionId: nil, result: nil))
    }
}
