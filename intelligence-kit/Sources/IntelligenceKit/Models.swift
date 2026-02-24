import Foundation
import FoundationModels

// MARK: - Command Enum

/// Supported command types for the NDJSON protocol.
///
/// Each command corresponds to a specific server operation:
/// - `openSession`: Create a new LanguageModelSession
/// - `message`: Execute a prompt against an existing session
/// - `closeSession`: Explicitly close a session
/// - `checkAvailability`: Check if Foundation Models is available
/// - `shutdown`: Gracefully shut down the server
enum Command: String, Codable {
    case openSession = "open-session"
    case message = "message"
    case closeSession = "close-session"
    case checkAvailability = "check-availability"
    case shutdown = "shutdown"
}

// MARK: - Request

/// A request message in the NDJSON protocol.
///
/// Requests are discriminated unions based on the `command` field.
/// Different commands require different optional fields:
///
/// - `open-session`: Optional `instructions` field
/// - `message`: Requires `sessionId`, `prompt`, `content`, `outputFormat`
/// - `close-session`: Requires `sessionId`
/// - `check-availability`: No additional fields
/// - `shutdown`: No additional fields
///
/// Field names use snake_case in JSON (via CodingKeys) but camelCase in Swift.
struct Request: Codable {
    let command: String
    
    // Command-specific fields (optional)
    let sessionId: String?
    let instructions: String?
    let prompt: String?
    let content: String?
    let outputFormat: String?
    
    enum CodingKeys: String, CodingKey {
        case command
        case sessionId = "session_id"
        case instructions
        case prompt
        case content
        case outputFormat = "output_format"
    }
}

// MARK: - Response Types

/// A response message in the NDJSON protocol.
///
/// Responses are discriminated unions that encode to flat JSON structures:
/// - `success`: Contains `ok: true` plus optional `sessionId` or `result`
/// - `error`: Contains `ok: false` plus `error` message
/// - `availability`: Contains `ok: true`, `available` boolean, and optional `reason`
///
/// Custom encoding ensures each case produces the correct JSON structure without
/// a discriminator key.
enum Response: Codable {
    case success(SuccessResponse)
    case error(ErrorResponse)
    case availability(AvailabilityResponse)
    
    // Custom encoding to produce the correct JSON structure
    func encode(to encoder: Encoder) throws {
        switch self {
        case .success(let response):
            try response.encode(to: encoder)
        case .error(let response):
            try response.encode(to: encoder)
        case .availability(let response):
            try response.encode(to: encoder)
        }
    }
    
    // Custom decoding (not used by server, but required for Codable conformance)
    init(from decoder: Decoder) throws {
        // Try to decode as each response type
        if let success = try? SuccessResponse(from: decoder) {
            self = .success(success)
            return
        }
        if let availability = try? AvailabilityResponse(from: decoder) {
            self = .availability(availability)
            return
        }
        if let error = try? ErrorResponse(from: decoder) {
            self = .error(error)
            return
        }
        
        throw DecodingError.dataCorrupted(
            DecodingError.Context(
                codingPath: decoder.codingPath,
                debugDescription: "Unable to decode Response"
            )
        )
    }
}

/// Success response for open-session and message commands.
///
/// - For `open-session`: Contains `sessionId`
/// - For `message`: Contains `result` (either string array or string)
/// - For `close-session` and `shutdown`: Both fields are nil
///
/// Custom encoding omits nil optional fields to produce clean JSON.
struct SuccessResponse: Codable {
    let ok: Bool = true
    let sessionId: String?    // For open-session
    let result: ResultValue?  // For message
    
    enum CodingKeys: String, CodingKey {
        case ok
        case sessionId = "session_id"
        case result
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(ok, forKey: .ok)
        
        // Only encode non-nil optional fields
        if let sessionId = sessionId {
            try container.encode(sessionId, forKey: .sessionId)
        }
        if let result = result {
            try container.encode(result, forKey: .result)
        }
    }
}

/// Error response for any failed operation.
///
/// Contains a machine-readable error code string. Common error codes:
/// - `invalid_json`: Request could not be parsed
/// - `unknown_command`: Command field not recognized
/// - `session_not_found`: Session ID does not exist
/// - `session_id_required`, `prompt_required`, etc.: Missing required field
/// - `unknown_output_format`: Output format not "string_list" or "text"
/// - `guardrail_blocked`: Foundation Models content filter blocked the request
/// - `model_unavailable`: Foundation Models became unavailable during execution
struct ErrorResponse: Codable {
    let ok: Bool = false
    let error: String
}

/// Availability check response.
///
/// Indicates whether Foundation Models is available on this system.
/// When `available` is false, `reason` explains why (e.g., "Apple Intelligence not enabled",
/// "macOS 26.0 or later required", "Apple Silicon required").
struct AvailabilityResponse: Codable {
    let ok: Bool = true
    let available: Bool
    let reason: String?  // Present when available=false
}

// MARK: - Result Value

/// The result value from a message execution.
///
/// Encodes to flat JSON without a discriminator key:
/// - `stringList`: Encodes as `["item1", "item2", ...]`
/// - `text`: Encodes as `"text content"`
///
/// The shape is determined by the `outputFormat` field in the message request.
enum ResultValue: Codable {
    case stringList([String])
    case text(String)
    
    // Custom encoding to produce flat JSON without discriminator key
    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .stringList(let items):
            try container.encode(items)
        case .text(let string):
            try container.encode(string)
        }
    }
    
    // Custom decoding (not used by server, but required for Codable conformance)
    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        
        // Try to decode as array first
        if let items = try? container.decode([String].self) {
            self = .stringList(items)
            return
        }
        
        // Otherwise decode as string
        if let string = try? container.decode(String.self) {
            self = .text(string)
            return
        }
        
        throw DecodingError.dataCorruptedError(
            in: container,
            debugDescription: "ResultValue must be either [String] or String"
        )
    }
}

// MARK: - Output Format Types (@Generable)

/// Output format types for guided generation using Foundation Models framework.
///
/// These types use `@Generable` and `@Guide` macros to constrain the model's token
/// generation to match specific struct shapes. This ensures structured output without
/// post-processing or parsing.
///
/// - `StringListOutput`: For `output_format: "string_list"` - produces an array of strings
/// - `TextOutput`: For `output_format: "text"` - produces a single string
///
/// The `@Guide` macro provides hints to the model about what each field represents.

/// Guided generation output for string list format.
///
/// Used when `outputFormat` is "string_list". The model is constrained to produce
/// an array of strings matching this structure.
@Generable
struct StringListOutput {
    @Guide(description: "A list of strings")
    let items: [String]
}

/// Guided generation output for text format.
///
/// Used when `outputFormat` is "text". The model is constrained to produce
/// a single string matching this structure.
@Generable
struct TextOutput {
    @Guide(description: "The text response")
    let text: String
}

