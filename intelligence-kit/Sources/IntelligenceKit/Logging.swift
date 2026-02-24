import Foundation

/// Logging utilities for IntelligenceKit.
///
/// All logging functions automatically prefix messages with `[IntelligenceKit]` and
/// write to stderr (stdout is reserved for JSON responses).
///
/// Design rationale: Auto-prefixing ensures consistency and prevents forgetting the
/// prefix at call sites.

/// Writes a message to stderr with automatic `[IntelligenceKit]` prefix.
///
/// - Parameter message: The message to log (without prefix or newline)
///
/// Example:
/// ```swift
/// logToStderr("Server started")
/// // Output to stderr: [IntelligenceKit] Server started\n
/// ```
func logToStderr(_ message: String) {
    let prefixed = "[IntelligenceKit] \(message)\n"
    if let data = prefixed.data(using: .utf8) {
        FileHandle.standardError.write(data)
    }
}

/// Logs an error message to stderr with "Error:" prefix.
///
/// - Parameter message: The error message to log
///
/// Example:
/// ```swift
/// logError("Failed to encode response")
/// // Output to stderr: [IntelligenceKit] Error: Failed to encode response\n
/// ```
func logError(_ message: String) {
    logToStderr("Error: \(message)")
}

/// Logs a warning message to stderr with "Warning:" prefix.
///
/// - Parameter message: The warning message to log
///
/// Example:
/// ```swift
/// logWarning("Session idle timeout")
/// // Output to stderr: [IntelligenceKit] Warning: Session idle timeout\n
/// ```
func logWarning(_ message: String) {
    logToStderr("Warning: \(message)")
}
