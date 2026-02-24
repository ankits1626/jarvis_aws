import Foundation
import FoundationModels

// MARK: - Execution Errors

/// Errors that can occur during message execution.
enum ExecutionError: Error {
    /// The requested output format is not "string_list" or "text"
    case unknownOutputFormat(String)
    
    /// Foundation Models content safety filter blocked the request
    case guardrailBlocked
    
    /// Foundation Models became unavailable during execution
    case modelUnavailable
}

// MARK: - MessageExecutor

/// Executes generic message requests against a LanguageModelSession.
///
/// MessageExecutor is responsible for:
/// - Truncating content to fit within the 4096-token context window (10,000 characters)
/// - Constructing prompts by combining the user's prompt with content
/// - Applying guided generation based on the requested output format
/// - Handling Foundation Models errors
///
/// The executor is task-agnostic - it doesn't contain any domain-specific logic.
/// All intelligence about what to do lives in the client's prompt.
///
/// Example usage:
/// ```swift
/// let executor = MessageExecutor()
/// let result = try await executor.execute(
///     session: session,
///     prompt: "Generate 3-5 topic tags for this content",
///     content: "Long article text...",
///     outputFormat: "string_list"
/// )
/// ```
struct MessageExecutor {
    private let maxContentLength = 10_000
    
    /// Truncates content to fit within the context window.
    ///
    /// - Parameter content: The content to truncate
    /// - Returns: Truncated content (maximum 10,000 characters)
    ///
    /// The 10,000 character limit ensures we stay well within the 4096-token
    /// context window, accounting for the prompt and model overhead.
    func truncateContent(_ content: String) -> String {
        String(content.prefix(maxContentLength))
    }
    
    /// Constructs the full prompt from user prompt and content.
    ///
    /// - Parameters:
    ///   - prompt: The user's prompt describing what to do
    ///   - content: The content to process
    /// - Returns: Formatted prompt string with content section
    ///
    /// The prompt format is:
    /// ```
    /// <user prompt>
    ///
    /// Content:
    /// <content>
    /// ```
    func constructPrompt(prompt: String, content: String) -> String {
        """
\(prompt)

Content:
\(content)
"""
    }
    
    /// Executes a message request with guided generation.
    ///
    /// - Parameters:
    ///   - session: The LanguageModelSession to use for execution
    ///   - prompt: What to do (e.g., "Generate 3-5 topic tags")
    ///   - content: Input text to process
    ///   - outputFormat: Either "string_list" or "text"
    /// - Returns: Structured result matching the requested format
    /// - Throws: `ExecutionError.unknownOutputFormat` if format is not recognized
    ///
    /// The method uses Foundation Models' guided generation feature to constrain
    /// the model's output to match specific struct shapes:
    /// - "string_list" → `StringListOutput` → Returns array of strings
    /// - "text" → `TextOutput` → Returns single string
    ///
    /// This ensures structured output without post-processing or parsing.
    func execute(
        session: LanguageModelSession,
        prompt: String,
        content: String,
        outputFormat: String
    ) async throws -> ResultValue {
        // Truncate content to fit context window
        let truncatedContent = truncateContent(content)
        
        // Construct full prompt
        let fullPrompt = constructPrompt(prompt: prompt, content: truncatedContent)
        
        // Execute with guided generation based on output format
        switch outputFormat {
        case "string_list":
            let response = try await session.respond(to: fullPrompt, generating: StringListOutput.self)
            return .stringList(response.content.items)
            
        case "text":
            let response = try await session.respond(to: fullPrompt, generating: TextOutput.self)
            return .text(response.content.text)
            
        default:
            throw ExecutionError.unknownOutputFormat(outputFormat)
        }
    }
}
