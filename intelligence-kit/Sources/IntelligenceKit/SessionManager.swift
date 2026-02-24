import Foundation
import FoundationModels

// MARK: - SessionState

/// Internal state for a LanguageModelSession.
///
/// Tracks both the session instance and the last time it was accessed,
/// enabling idle timeout detection.
struct SessionState {
    /// The Foundation Models session instance
    let session: LanguageModelSession
    
    /// Timestamp of the last activity (session creation or message execution)
    var lastActivity: Date
}

// MARK: - SessionManager

/// Manages the lifecycle of LanguageModelSession instances.
///
/// SessionManager is an actor that provides thread-safe access to multiple concurrent
/// sessions. Key responsibilities:
///
/// - Create new sessions with unique UUIDs
/// - Track session activity for idle timeout
/// - Automatically close sessions idle for more than `idleTimeout` seconds (default 120s)
/// - Support graceful shutdown by closing all sessions
///
/// The idle monitoring task runs every 30 seconds in the background and closes any
/// sessions that haven't been accessed within the timeout period.
///
/// Example usage:
/// ```swift
/// let manager = SessionManager()
/// let sessionId = try await manager.openSession(instructions: "You are a helpful assistant")
/// if let session = await manager.getSession(sessionId) {
///     // Use session for message execution
/// }
/// await manager.closeSession(sessionId)
/// ```
actor SessionManager {
    private var sessions: [String: SessionState] = [:]
    private nonisolated(unsafe) var idleCheckTask: Task<Void, Never>?
    private let idleTimeout: TimeInterval
    
    /// Creates a new SessionManager with configurable idle timeout.
    ///
    /// - Parameter idleTimeout: Time in seconds before an idle session is automatically closed (default: 120.0)
    ///
    /// The idle monitoring task starts immediately upon initialization.
    init(idleTimeout: TimeInterval = 120.0) {
        self.idleTimeout = idleTimeout
        startIdleMonitoring()
    }
    
    /// Creates a new LanguageModelSession with a unique session ID.
    ///
    /// - Parameter instructions: Optional system instructions for the session.
    ///   If nil, defaults to "You are a helpful assistant."
    /// - Returns: A unique session ID (UUID string)
    /// - Throws: Any errors from LanguageModelSession initialization
    ///
    /// The session is immediately added to the active sessions dictionary with
    /// `lastActivity` set to the current time.
    func openSession(instructions: String?) async throws -> String {
        let sessionId = UUID().uuidString
        let session = LanguageModelSession(
            instructions: instructions ?? "You are a helpful assistant."
        )
        
        sessions[sessionId] = SessionState(
            session: session,
            lastActivity: Date.now
        )
        
        logToStderr("Session opened: \(sessionId)")
        return sessionId
    }
    
    /// Retrieves a session by ID and updates its last activity timestamp.
    ///
    /// - Parameter sessionId: The session ID to retrieve
    /// - Returns: The LanguageModelSession if found, nil otherwise
    ///
    /// Calling this method resets the idle timeout for the session by updating
    /// `lastActivity` to the current time.
    func getSession(_ sessionId: String) -> LanguageModelSession? {
        guard var state = sessions[sessionId] else { return nil }
        state.lastActivity = Date.now
        sessions[sessionId] = state
        return state.session
    }
    
    /// Closes a session and removes it from the active sessions dictionary.
    ///
    /// - Parameter sessionId: The session ID to close
    ///
    /// If the session doesn't exist, this method does nothing. The LanguageModelSession
    /// instance is automatically cleaned up by Swift's ARC.
    func closeSession(_ sessionId: String) {
        guard sessions.removeValue(forKey: sessionId) != nil else { return }
        logToStderr("Session closed: \(sessionId)")
    }
    
    /// Closes all active sessions.
    ///
    /// Used during server shutdown to ensure clean resource cleanup.
    /// Iterates over a snapshot of session IDs to avoid mutation during iteration.
    func closeAllSessions() {
        let sessionIds = Array(sessions.keys)
        for sessionId in sessionIds {
            closeSession(sessionId)
        }
    }
    
    /// Starts the background idle monitoring task.
    ///
    /// The task runs every 30 seconds and calls `checkIdleSessions()` to close
    /// any sessions that have exceeded the idle timeout.
    ///
    /// Marked `nonisolated` to allow calling from the synchronous `init()`.
    private nonisolated func startIdleMonitoring() {
        idleCheckTask = Task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(30))
                await checkIdleSessions()
            }
        }
    }
    
    /// Checks all sessions for idle timeout and closes stale ones.
    ///
    /// Uses a collect-then-close pattern to avoid mutation during iteration:
    /// 1. Collect session IDs that have exceeded the timeout
    /// 2. Close each stale session
    ///
    /// Logs a warning for each session closed due to idle timeout.
    private func checkIdleSessions() {
        let now = Date.now
        // Collect stale session IDs first to avoid mutation during iteration
        let staleSessionIds = sessions.filter { (_, state) in
            now.timeIntervalSince(state.lastActivity) > idleTimeout
        }.map { $0.key }
        
        // Close stale sessions
        for sessionId in staleSessionIds {
            logWarning("Session \(sessionId) idle timeout, closing")
            closeSession(sessionId)
        }
    }
}
