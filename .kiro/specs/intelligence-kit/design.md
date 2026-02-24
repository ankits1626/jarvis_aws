# Design Document: IntelligenceKit

## Overview

IntelligenceKit is a macOS Swift server that provides a generic gateway to Apple's Foundation Models framework (on-device 3B parameter LLM). It runs as a persistent local server over stdin/stdout using newline-delimited JSON (NDJSON), enabling the Jarvis Tauri app to leverage on-device AI capabilities without API keys or network requests.

The system is **task-agnostic** — it does not contain any tagging, summarization, or redaction logic. The Rust client sends a `prompt` (what to do), `content` (the input), and `output_format` (the shape of the result). The server executes the prompt through Foundation Models and returns structured results using guided generation. All intelligence about what to do lives in the Rust client; the Swift server is a generic execution gateway.

Key architectural features:
- **Persistent server process** with NDJSON wire protocol (stdin/stdout)
- **Session management** supporting multiple concurrent LanguageModelSession instances
- **Generic message execution** without task-specific logic
- **Guided generation** using @Generable/@Guide macros for structured output
- **Auto-close sessions** after 120s idle timeout
- **Availability checking** before session creation

The system operates as a request-response pipeline:
1. **Read**: Parse NDJSON commands from stdin
2. **Route**: Dispatch to appropriate handler (open-session, message, close-session, check-availability, shutdown)
3. **Execute**: Run prompts through Foundation Models with guided generation
4. **Respond**: Write NDJSON responses to stdout

This design follows the JarvisListen pattern (persistent server, stdin/stdout protocol) but adds session management and stateful conversation support.

## Build Configuration

### Package.swift

```swift
// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "IntelligenceKit",
    platforms: [
        .macOS(.v26)  // Foundation Models requires macOS 26.0+
    ],
    products: [
        .executable(
            name: "IntelligenceKit",
            targets: ["IntelligenceKit"]
        )
    ],
    targets: [
        .executableTarget(
            name: "IntelligenceKit",
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency")
            ]
        ),
        .testTarget(
            name: "IntelligenceKitTests",
            dependencies: ["IntelligenceKit"]
        )
    ]
)
```

**Configuration Details:**
- **Swift Tools Version**: 6.2 (required for Foundation Models framework)
- **Platform**: macOS 26.0+ (Foundation Models minimum requirement)
- **Product**: Executable named `IntelligenceKit`
- **StrictConcurrency**: Enabled to ensure proper actor isolation and thread safety
- **No Dependencies**: Only Apple frameworks (Foundation, FoundationModels) — no external packages
- **Target Architecture**: Apple Silicon (arm64) — enforced by macOS 26.0+ requirement

### Directory Structure

```
intelligence-kit/                              # sibling to jarvis-listen/
├── Package.swift                              # Build configuration
├── Sources/IntelligenceKit/
│   ├── main.swift                             # Entry point, signal handling, server loop
│   ├── Logging.swift                          # Logging utilities (stderr output)
│   ├── Models.swift                           # Request/Response Codable types
│   ├── SessionManager.swift                   # Session lifecycle management
│   ├── MessageExecutor.swift                  # Generic message execution
│   ├── CommandRouter.swift                    # Command routing and handlers
│   └── Availability.swift                     # SystemLanguageModel availability check
└── Tests/IntelligenceKitTests/
    ├── ModelsTests.swift                      # JSON encode/decode tests
    ├── SessionManagerTests.swift              # Session management tests
    ├── MessageExecutorTests.swift             # Message execution tests
    └── ProtocolTests.swift                    # NDJSON protocol tests
```

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         main.swift                          │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Signal       │  │ Read Loop    │  │ Command         │  │
│  │ Handler      │  │ (stdin)      │  │ Router          │  │
│  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    SessionManager.swift                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Session Lifecycle Management                         │  │
│  │ • Create/destroy LanguageModelSession instances      │  │
│  │ • Track session IDs and idle timeouts                │  │
│  │ • Concurrent session support                         │  │
│  │ • Auto-close after 120s idle                         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   MessageExecutor.swift                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Generic Message Execution                            │  │
│  │ • Construct prompts from client input                │  │
│  │ • Execute via LanguageModelSession                   │  │
│  │ • Apply guided generation (@Generable types)         │  │
│  │ • Return structured results                          │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Availability.swift                      │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Model Availability Checking                          │  │
│  │ • Check SystemLanguageModel.default availability     │  │
│  │ • Distinguish unavailability reasons                 │  │
│  │ • Return structured availability status              │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       Models.swift                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Request/Response Types                               │  │
│  │ • Command enums and request structs                  │  │
│  │ • Response structs (success/error)                   │  │
│  │ • @Generable output format types                     │  │
│  │ • Codable conformance for JSON serialization         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Foundation Models Framework                    │
│  • SystemLanguageModel.default                              │
│  • LanguageModelSession (multi-turn context)                │
│  • @Generable/@Guide macros (guided generation)             │
└─────────────────────────────────────────────────────────────┘
```

### Component Descriptions

#### 1. main.swift
**Responsibilities:**
- Set up signal handlers for SIGINT, SIGTERM
- Initialize SessionManager
- Run the read loop: continuously read NDJSON from stdin
- Parse each line as a Command
- Route commands to appropriate handlers
- Write NDJSON responses to stdout
- Handle graceful shutdown

**Key APIs:**
- `FileHandle.standardInput` for reading stdin
- `FileHandle.standardOutput` for writing stdout
- `FileHandle.standardError` for logging
- `signal()` for SIGINT/SIGTERM handling
- `JSONDecoder`/`JSONEncoder` for NDJSON parsing
- `Task { }` for async context
- `RunLoop.main.run()` to keep process alive

### Logging.swift

Provides logging utilities that automatically prefix all messages with `[IntelligenceKit]` and write to stderr:

```swift
import Foundation

func logToStderr(_ message: String) {
    let prefixed = "[IntelligenceKit] \(message)\n"
    if let data = prefixed.data(using: .utf8) {
        FileHandle.standardError.write(data)
    }
}

func logError(_ message: String) {
    logToStderr("Error: \(message)")
}

func logWarning(_ message: String) {
    logToStderr("Warning: \(message)")
}
```

**Design Notes:**
- Auto-prefixing ensures all log messages have `[IntelligenceKit]` prefix without manual inclusion at call sites
- Prevents forgetting the prefix in individual log calls
- Follows requirement 9.6: ALL log messages SHALL be prefixed
- All logging goes to stderr (stdout reserved for JSON responses)

**Async Context Setup:**
Since `readLine()` is synchronous but command handlers need `await`, the main.swift file uses top-level code (no `@main` attribute) to create an async context, following the JarvisListen pattern:

```swift
// main.swift (top-level code)
import Foundation
import FoundationModels

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
```

**Read Loop Pattern:**
```swift
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
        guard !line.isEmpty else { continue }
        
        // Check signal handler flag
        if let handler = signalHandler, handler.shouldShutdown {
            logToStderr("Signal received, shutting down")
            await sessionManager.closeAllSessions()
            exit(0)
        }
        
        do {
            let request = try JSONDecoder().decode(Request.self, from: line.data(using: .utf8)!)
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
    
    // Clean exit after shutdown command
    await sessionManager.closeAllSessions()
    logToStderr("Shutdown complete")
    exit(0)
}

func writeResponse(_ response: Response) {
    guard let data = try? JSONEncoder().encode(response) else {
        logError("Failed to encode response")
        return
    }
    FileHandle.standardOutput.write(data + Data("\n".utf8))
}
```

**Signal Handler Setup:**

IntelligenceKit uses DispatchSource.makeSignalSource (following JarvisListen pattern) to set a shutdown flag on a background queue. This is async-signal-safe and avoids blocking issues.

**Important Trade-off:** Since `readLine()` blocks, the shutdown flag won't be checked until the next line arrives. However, in normal operation, Tauri should send a `shutdown` command before sending SIGTERM, allowing graceful cleanup. The signal handler is a safety net for abnormal termination (e.g., parent process dies). In this case, `exit(0)` is acceptable since:
1. Swift's ARC automatically cleans up LanguageModelSession instances
2. Foundation Models framework handles its own cleanup
3. No file handles or persistent state need explicit cleanup

```swift
// Global signal handler instance
var signalHandler: SignalHandler?
var shouldShutdown = false

class SignalHandler {
    private var _shouldShutdown = false
    private var lock = os_unfair_lock()
    private var sigintSource: DispatchSourceSignal?
    private var sigtermSource: DispatchSourceSignal?
    private let signalQueue = DispatchQueue(label: "com.intelligencekit.signals")
    
    var shouldShutdown: Bool {
        os_unfair_lock_lock(&lock)
        defer { os_unfair_lock_unlock(&lock) }
        return _shouldShutdown
    }
    
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
    
    private func setShutdown() {
        os_unfair_lock_lock(&lock)
        _shouldShutdown = true
        os_unfair_lock_unlock(&lock)
    }
    
    deinit {
        sigintSource?.cancel()
        sigtermSource?.cancel()
    }
}

func setupSignalHandlers() {
    signalHandler = SignalHandler()
    signalHandler?.setup()
}
```

#### 2. SessionManager.swift
**Responsibilities:**
- Create new LanguageModelSession instances with unique session IDs
- Store active sessions in a dictionary [String: SessionState]
- Track last activity time for each session
- Automatically close sessions idle for >120 seconds
- Support concurrent sessions (thread-safe access)
- Clean up resources on session close

**Key APIs:**
- `SystemLanguageModel.default` to create sessions
- `LanguageModelSession` for stateful conversations
- `Task` for idle timeout monitoring
- Actor isolation for thread safety

**Session State:**
```swift
struct SessionState {
    let session: LanguageModelSession
    var lastActivity: Date
}
```

**Idle Timeout Strategy:**
- Use a background Task that checks all sessions every 30 seconds
- For each session, if `Date.now - lastActivity > 120s`, close it
- Log warning to stderr when auto-closing

#### 3. MessageExecutor.swift
**Responsibilities:**
- Execute generic message requests against a LanguageModelSession
- Construct the full prompt by combining client's `prompt` + `content`
- Truncate content to 10,000 characters to fit 4096-token context
- Apply guided generation based on `output_format`
- Return structured results (string_list or text)
- Handle Foundation Models errors (guardrails, unavailability)

**Key APIs:**
- `LanguageModelSession.respond(to:generating:)` for message execution with guided generation
- `@Generable` types for structured output
- Error handling for guardrail blocks

**Prompt Construction:**
```
<client prompt>

Content:
<truncated content>
```

**Guided Generation:**
- For `output_format: "string_list"`, use `@Generable struct StringListOutput { let items: [String] }`
- For `output_format: "text"`, use `@Generable struct TextOutput { let text: String }`
- Foundation Models constrains token generation to match the struct shape

#### 4. Availability.swift
**Responsibilities:**
- Check if SystemLanguageModel.default is available
- Distinguish between unavailability reasons:
  - Apple Intelligence not enabled by user
  - macOS version too old (<26.0)
  - Hardware not supported (not Apple Silicon)
- Return structured availability status
- Complete check within 2 seconds

**Key APIs:**
- `SystemLanguageModel.default` availability check
- `ProcessInfo.operatingSystemVersion` for macOS version
- `#if arch(arm64)` for hardware check

**Availability Reasons:**
```swift
enum UnavailabilityReason: String, Codable {
    case notEnabled = "Apple Intelligence not enabled by user"
    case oldMacOS = "macOS 26.0 or later required"
    case unsupportedHardware = "Apple Silicon required"
    case unknown = "Unknown reason"
}
```

#### 5. Models.swift
**Responsibilities:**
- Define all request and response types
- Implement Codable for JSON serialization
- Define @Generable output format types
- Provide type-safe command routing

**Key Types:**
- `Request` - discriminated union of all command types
- `Response` - discriminated union of all response types
- `OpenSessionRequest`, `MessageRequest`, `CloseSessionRequest`, etc.
- `SuccessResponse`, `ErrorResponse`, `AvailabilityResponse`
- `StringListOutput`, `TextOutput` (@Generable types)

## Data Models

### Request Types

#### Request (Discriminated Union)
```swift
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
```

#### OpenSessionRequest
```swift
struct OpenSessionRequest {
    let instructions: String?  // Optional system instructions
}
```

#### MessageRequest
```swift
struct MessageRequest {
    let sessionId: String
    let prompt: String        // What to do (e.g., "Generate 3-5 topic tags")
    let content: String       // Input text to process
    let outputFormat: String  // "string_list" or "text"
}
```

#### CloseSessionRequest
```swift
struct CloseSessionRequest {
    let sessionId: String
}
```

### Response Types

#### Response (Discriminated Union)
```swift
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
```

#### SuccessResponse
```swift
struct SuccessResponse: Codable {
    let ok: Bool = true
    let sessionId: String?    // For open-session
    let result: ResultValue?  // For message
    
    enum CodingKeys: String, CodingKey {
        case ok
        case sessionId = "session_id"
        case result
    }
}

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
```

#### ErrorResponse
```swift
struct ErrorResponse: Codable {
    let ok: Bool = false
    let error: String
}
```

#### AvailabilityResponse
```swift
struct AvailabilityResponse: Codable {
    let ok: Bool = true
    let available: Bool
    let reason: String?  // Present when available=false
}
```

### Output Format Types

These are the @Generable types used for guided generation:

#### StringListOutput
```swift
@Generable
struct StringListOutput {
    @Guide("A list of strings")
    let items: [String]
}
```

#### TextOutput
```swift
@Generable
struct TextOutput {
    @Guide("The text response")
    let text: String
}
```

### Session State

#### SessionState
```swift
struct SessionState {
    let session: LanguageModelSession
    var lastActivity: Date
}
```

### Internal Models

#### Command
```swift
enum Command: String, Codable {
    case openSession = "open-session"
    case message = "message"
    case closeSession = "close-session"
    case checkAvailability = "check-availability"
    case shutdown = "shutdown"
}
```

## Components and Interfaces

### SessionManager

```swift
actor SessionManager {
    private var sessions: [String: SessionState] = [:]
    private var idleCheckTask: Task<Void, Never>?
    private let idleTimeout: TimeInterval = 120.0
    
    init() {
        startIdleMonitoring()
    }
    
    // Create a new session with optional instructions
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
    
    // Get a session and update its last activity time
    func getSession(_ sessionId: String) -> LanguageModelSession? {
        guard var state = sessions[sessionId] else { return nil }
        state.lastActivity = Date.now
        sessions[sessionId] = state
        return state.session
    }
    
    // Close a session and free resources
    func closeSession(_ sessionId: String) {
        guard sessions.removeValue(forKey: sessionId) != nil else { return }
        logToStderr("Session closed: \(sessionId)")
    }
    
    // Close all sessions (for shutdown)
    func closeAllSessions() {
        let sessionIds = Array(sessions.keys)
        for sessionId in sessionIds {
            closeSession(sessionId)
        }
    }
    
    // Background task to monitor idle sessions
    private func startIdleMonitoring() {
        idleCheckTask = Task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(30))
                await checkIdleSessions()
            }
        }
    }
    
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
```

### MessageExecutor

```swift
struct MessageExecutor {
    private let maxContentLength = 10_000
    
    func execute(
        session: LanguageModelSession,
        prompt: String,
        content: String,
        outputFormat: String
    ) async throws -> ResultValue {
        // Truncate content to fit context window
        let truncatedContent = String(content.prefix(maxContentLength))
        
        // Construct full prompt
        let fullPrompt = """
\(prompt)

Content:
\(truncatedContent)
"""
        
        // Execute with guided generation based on output format
        switch outputFormat {
        case "string_list":
            let output = try await session.respond(to: fullPrompt, generating: StringListOutput.self)
            return .stringList(output.items)
            
        case "text":
            let output = try await session.respond(to: fullPrompt, generating: TextOutput.self)
            return .text(output.text)
            
        default:
            throw ExecutionError.unknownOutputFormat(outputFormat)
        }
    }
}

enum ExecutionError: Error {
    case unknownOutputFormat(String)
    case guardrailBlocked
    case modelUnavailable
}
```

### AvailabilityChecker

```swift
struct AvailabilityChecker {
    func check() -> (available: Bool, reason: String?) {
        // Check hardware
        #if !arch(arm64)
        return (false, "Apple Silicon required")
        #endif
        
        // Check macOS version
        let osVersion = ProcessInfo.processInfo.operatingSystemVersion
        if osVersion.majorVersion < 26 {
            return (false, "macOS 26.0 or later required")
        }
        
        // Check model availability
        // Note: SystemLanguageModel.default.availability is a synchronous property,
        // so no timeout is needed. Req 5.6 specifies 2-second completion, but in
        // practice this is a simple property access that returns immediately.
        let availability = SystemLanguageModel.default.availability
        
        switch availability {
        case .available:
            return (true, nil)
        case .unavailable(let reason):
            return (false, reason.localizedDescription)
        @unknown default:
            return (false, "Unknown availability status")
        }
    }
}
```

### Command Router

```swift
struct CommandRouter {
    let sessionManager: SessionManager
    let messageExecutor: MessageExecutor
    let availabilityChecker: AvailabilityChecker
    
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
    
    private func handleCloseSession(_ request: Request) async -> Response {
        guard let sessionId = request.sessionId else {
            return .error(ErrorResponse(error: "session_id_required"))
        }
        
        await sessionManager.closeSession(sessionId)
        return .success(SuccessResponse(sessionId: nil, result: nil))
    }
    
    private func handleCheckAvailability() -> Response {
        let (available, reason) = availabilityChecker.check()
        return .availability(AvailabilityResponse(available: available, reason: reason))
    }
    
    private func handleShutdown() async -> Response {
        await sessionManager.closeAllSessions()
        logToStderr("Shutting down")
        // Note: After returning this response and writing it to stdout,
        // the main loop should exit. The shutdown command sets a flag
        // that breaks the read loop after the response is written.
        return .success(SuccessResponse(sessionId: nil, result: nil))
    }
}
```

## Algorithms

### Server Lifecycle

```
1. Startup:
   - Set up signal handlers (SIGINT, SIGTERM)
   - Log to stderr: "IntelligenceKit server ready"
   - Create async context using Task { }
   - Start RunLoop.main.run() to keep process alive
   
2. Async Server Initialization (inside Task):
   - Initialize SessionManager (starts idle monitoring task)
   - Initialize MessageExecutor
   - Initialize AvailabilityChecker
   - Initialize CommandRouter
   
3. Read Loop (async context):
   while true:
     - Read line from stdin using readLine() (synchronous)
     - If line is empty, continue
     - If stdin reaches EOF, break (parent died)
     - Try to parse line as JSON Request
     - If parse fails, write {"ok":false,"error":"invalid_json"} and continue
     - Route request to appropriate handler (await)
     - Write response as JSON to stdout
     - Flush stdout
     - If command was "shutdown", set flag and break loop
   
4. Shutdown:
   - Close all active sessions via SessionManager
   - Cancel idle monitoring task
   - Log to stderr: "IntelligenceKit shutdown complete"
   - Exit with code 0
```

### Session Management

```
Open Session:
  1. Generate unique session ID (UUID)
  2. Create LanguageModelSession via direct initialization
  3. Set instructions (use client's or default)
  4. Store in sessions dictionary with current timestamp
  5. Log session ID to stderr
  6. Return session ID to client

Get Session:
  1. Look up session ID in dictionary
  2. If not found, return nil
  3. Update lastActivity to current time
  4. Return LanguageModelSession

Close Session:
  1. Remove session from dictionary
  2. Log closure to stderr
  3. LanguageModelSession is automatically cleaned up (ARC)

Idle Monitoring (background task, runs every 30s):
  1. Get current time
  2. For each session in dictionary:
     - Calculate idle time: now - lastActivity
     - If idle time > 120 seconds:
       * Log warning to stderr
       * Close session
```

### Message Execution

```
Input: session, prompt, content, outputFormat
Output: ResultValue (stringList or text)

1. Validate inputs:
   - prompt must be non-empty
   - content must be non-empty
   - outputFormat must be "string_list" or "text"
   
2. Truncate content:
   - Take first 10,000 characters
   - This ensures we fit within 4096-token context window
   
3. Construct full prompt:
   <client prompt>
   
   Content:
   <truncated content>
   
4. Select @Generable type based on outputFormat:
   - "string_list" → StringListOutput
   - "text" → TextOutput
   
5. Execute via LanguageModelSession:
   - Call session.respond(to: fullPrompt, generating: OutputType.self)
   - Foundation Models applies guided generation
   - Token selection is constrained to match struct shape
   
6. Extract result:
   - For StringListOutput: return .stringList(output.items)
   - For TextOutput: return .text(output.text)
   
7. Handle errors:
   - Guardrail block → throw ExecutionError.guardrailBlocked
   - Model unavailable → throw ExecutionError.modelUnavailable
   - Unknown format → throw ExecutionError.unknownOutputFormat
```

### Availability Checking

```
1. Check hardware architecture:
   - If not arm64, return (false, "Apple Silicon required")
   
2. Check macOS version:
   - Get ProcessInfo.operatingSystemVersion
   - If majorVersion < 26, return (false, "macOS 26.0 or later required")
   
3. Check model availability:
   - Get SystemLanguageModel.default.availability (synchronous property)
   - Switch on availability enum:
     * .available → return (true, nil)
     * .unavailable(reason) → return (false, reason.localizedDescription)
     * @unknown default → return (false, "Unknown availability status")
   
Note: No timeout needed — availability is a synchronous property that returns
immediately. Req 5.6 specifies 2-second completion, but this is satisfied by
the synchronous nature of the property access.
```

### NDJSON Protocol

```
Message Format:
  - Each message is a single JSON object on one line
  - Terminated by \n
  - No pretty-printing, no extra whitespace
  
Reading:
  - Use readLine() to read one line at a time
  - Parse line as JSON
  - Decode to Request struct
  - Handle parse errors gracefully (return error response, don't crash)
  
Writing:
  - Encode Response struct to JSON
  - Write to stdout as single line
  - Append \n
  - Flush stdout immediately
  
Error Handling:
  - Invalid JSON → {"ok":false,"error":"invalid_json"}
  - Empty line → ignore, continue reading
  - EOF → graceful shutdown
```

### Signal Handling

```
Setup:
  - Create SignalHandler instance with DispatchSource.makeSignalSource
  - Disable default signal handling (SIG_IGN) for SIGINT, SIGTERM, SIGPIPE
  - Create dispatch sources on background queue
  - Set event handlers to update shutdown flag (thread-safe with os_unfair_lock)
  
On SIGINT or SIGTERM:
  - DispatchSource event handler sets shutdown flag to true
  - Flag is checked at the start of each read loop iteration
  - When flag is detected:
    * Log shutdown message to stderr
    * Close all sessions via SessionManager
    * Exit with code 0
  
Important Trade-off:
  - readLine() blocks, so flag won't be checked until next line arrives
  - Normal operation: Tauri sends "shutdown" command before SIGTERM
  - Signal handler is a safety net for abnormal termination
  - In abnormal cases, ARC handles cleanup automatically:
    * LanguageModelSession instances are released
    * Foundation Models framework cleans up internally
    * No file handles or persistent state require explicit cleanup
  
On EOF (stdin closed):
  - readLine() returns nil
  - Treat as shutdown signal
  - Close all sessions via SessionManager
  - Exit with code 0
```

## Error Handling

### Error Categories

#### 1. Protocol Errors
**Invalid JSON:**
- Error: Line cannot be parsed as JSON
- Handling: Write `{"ok":false,"error":"invalid_json"}` to stdout, continue reading
- Do not crash or exit

**Missing Required Field:**
- Error: Request missing `command`, `session_id`, `prompt`, `content`, or `output_format`
- Handling: Return `{"ok":false,"error":"<field>_required"}`
- Example: `{"ok":false,"error":"prompt_required"}`

**Unknown Command:**
- Error: `command` field value not recognized
- Handling: Return `{"ok":false,"error":"unknown_command"}`

#### 2. Session Errors
**Session Not Found:**
- Error: `message` or `close-session` references non-existent session ID
- Handling: Return `{"ok":false,"error":"session_not_found"}`
- Session may have been closed manually or by idle timeout

**Session Creation Failure:**
- Error: LanguageModelSession initialization fails
- Handling: Return `{"ok":false,"error":"failed_to_open_session: <details>"}`
- Log full error to stderr

#### 3. Execution Errors
**Unknown Output Format:**
- Error: `output_format` is not "string_list" or "text"
- Handling: Return `{"ok":false,"error":"unknown_output_format"}`

**Guardrail Blocked:**
- Error: Foundation Models content safety filter blocks the request
- Handling: Return `{"ok":false,"error":"guardrail_blocked"}`
- Log to stderr: prompt and content (truncated) for debugging
- Note: Guardrails cannot be disabled, may block benign content

**Model Unavailable:**
- Error: SystemLanguageModel becomes unavailable during execution
- Handling: Return `{"ok":false,"error":"model_unavailable"}`
- Session remains valid, client can retry

**Execution Failure:**
- Error: Any other error during message execution
- Handling: Return `{"ok":false,"error":"execution_failed: <details>"}`
- Log full error to stderr

#### 4. Availability Errors
**Availability Check Failure:**
- Error: Unexpected error during availability check (e.g., unknown availability status)
- Handling: Return `{"ok":true,"available":false,"reason":"Unknown availability status"}`
- Log full error to stderr
- Note: Availability check is synchronous (no timeout needed)

### Error Handling Strategy

1. **Never Crash**: All errors are returned as JSON responses. The server continues reading and processing messages.

2. **Structured Errors**: All error responses follow the format `{"ok":false,"error":"<error_code>"}` for easy parsing by clients.

3. **Detailed Logging**: Full error details (stack traces, exception messages) are logged to stderr for debugging, while concise error codes are returned to clients.

4. **Session Resilience**: Errors within a session do not invalidate the session. Clients can send another message on the same session after an error.

5. **Graceful Degradation**: If a session is auto-closed due to idle timeout, the client receives `session_not_found` and can open a new session.

6. **All Errors to stderr**: Diagnostic information never goes to stdout (reserved for JSON responses).

7. **Exit Codes**:
   - 0: Normal shutdown (shutdown command, SIGTERM, SIGINT, EOF)
   - Non-zero: Should never occur (server is designed to handle all errors gracefully)


## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

After analyzing all acceptance criteria, I identified several areas where properties can be consolidated:

1. **Response format properties (2.2, 2.3, 9.7)** can be combined into a single property that validates all stdout output is valid NDJSON and all non-JSON output goes to stderr.

2. **Field validation properties (4.1, 4.10, 4.11)** follow the same pattern and can be tested with a single comprehensive property about required field validation.

3. **Error response structure properties (6.3, 6.5)** can be combined into one property that validates all error responses have the correct structure.

4. **Logging properties (6.6, 9.6)** can be combined into a property about log message format and destination.

5. **Session lifecycle properties (3.1, 3.5)** test complementary behaviors (create/destroy) and can be tested together as a round-trip property.

6. **Output format properties (4.5, 4.6)** test the same behavior (guided generation produces correct structure) for different formats and can be combined.

### Core Properties

#### Property 1: One Response Per Request
*For any* valid request message, the server SHALL write exactly one response message to stdout.

**Validates: Requirements 1.2**

**Testing approach:** Generate random valid requests (open-session, message, close-session, check-availability). Send each to the server. Count responses. Verify count equals request count.

#### Property 2: Sequential Message Processing
*For any* sequence of request messages, the server SHALL process them in order and return responses in the same order.

**Validates: Requirements 1.4**

**Testing approach:** Generate a sequence of requests with identifiable markers (e.g., session IDs, unique prompts). Send all requests. Verify responses match the request order by checking markers.

#### Property 3: NDJSON Response Format
*For any* request, the server's response SHALL be a single valid JSON object terminated by `\n`, and all output to stdout SHALL be valid JSON.

**Validates: Requirements 2.2, 2.3, 9.7**

**Testing approach:** Generate random requests. Capture stdout. Verify: (1) each line is valid JSON, (2) each line ends with `\n`, (3) no non-JSON content appears on stdout.

#### Property 4: Invalid JSON Handling
*For any* line containing invalid JSON, the server SHALL return `{"ok":false,"error":"invalid_json"}` and continue processing subsequent messages without crashing.

**Validates: Requirements 2.4, 6.4**

**Testing approach:** Generate random invalid JSON strings (malformed brackets, missing quotes, etc.). Send to server. Verify: (1) response is the expected error, (2) server continues processing valid messages afterward.

#### Property 5: Empty Line Handling
*For any* sequence of messages containing empty lines, the server SHALL ignore empty lines and process only non-empty lines.

**Validates: Requirements 2.5**

**Testing approach:** Generate message sequences with random empty lines interspersed. Send to server. Verify: (1) empty lines produce no responses, (2) non-empty messages are processed correctly.

#### Property 6: Required Command Field
*For any* request missing the `command` field, the server SHALL return an error response.

**Validates: Requirements 2.6**

**Testing approach:** Generate JSON objects without a `command` field. Send to server. Verify error response is returned.

#### Property 7: Unique Session IDs
*For any* sequence of `open-session` requests, the server SHALL return unique session IDs (no duplicates).

**Validates: Requirements 3.1**

**Testing approach:** Send multiple open-session requests. Collect all returned session IDs. Verify no duplicates exist in the set.

#### Property 8: Concurrent Session Independence
*For any* two concurrent sessions, operations on one session SHALL NOT affect the state or behavior of the other session.

**Validates: Requirements 3.2**

**Testing approach:** Open two sessions. Send different messages to each (different prompts, content). Verify: (1) responses are independent, (2) closing one session doesn't affect the other.

#### Property 9: Session Not Found Error
*For any* message request with a non-existent session ID, the server SHALL return `{"ok":false,"error":"session_not_found"}`.

**Validates: Requirements 3.4**

**Testing approach:** Generate random session IDs that were never created. Send message requests with these IDs. Verify error response.

#### Property 10: Session Closure
*For any* session, after sending `close-session`, subsequent message requests to that session SHALL return `session_not_found`.

**Validates: Requirements 3.5**

**Testing approach:** Open a session, send a message (verify it works), close the session, send another message. Verify the second message returns session_not_found.

#### Property 11: Idle Timeout
*For any* session that remains idle (no messages sent) for more than 120 seconds, the server SHALL automatically close it.

**Validates: Requirements 3.6**

**Testing approach:** Open a session, wait 121 seconds, send a message. Verify session_not_found error. (Note: This is time-dependent and may be slow to test.)

#### Property 12: Optional Instructions Parameter
*For any* open-session request, the server SHALL accept an optional `instructions` field and create a session successfully regardless of whether it's provided.

**Validates: Requirements 3.8**

**Testing approach:** Send open-session requests with and without `instructions` field. Verify both succeed and return session IDs.

#### Property 13: Required Message Fields
*For any* message request missing `session_id`, `prompt`, `content`, or `output_format`, the server SHALL return an error indicating which field is required.

**Validates: Requirements 4.1, 4.10, 4.11**

**Testing approach:** Generate message requests with each required field missing (one at a time). Verify appropriate error responses: `session_id_required`, `prompt_required`, `content_required`, `output_format_required`.

#### Property 14: Output Format Structure
*For any* valid message request with `output_format` set to `"string_list"`, the response SHALL have structure `{"ok":true,"result":[...]}` where result is an array; for `"text"`, the response SHALL have structure `{"ok":true,"result":"..."}` where result is a string.

**Validates: Requirements 4.5, 4.6**

**Testing approach:** Send message requests with both output formats. Verify: (1) string_list returns array, (2) text returns string, (3) both have ok:true.

#### Property 15: Unknown Output Format Error
*For any* message request with an unrecognized `output_format` value, the server SHALL return `{"ok":false,"error":"unknown_output_format"}`.

**Validates: Requirements 4.7**

**Testing approach:** Generate random invalid output format strings. Send message requests with these formats. Verify error response.

#### Property 16: Content Truncation
*For any* message request with `content` longer than 10,000 characters, the server SHALL truncate it to 10,000 characters before processing.

**Validates: Requirements 4.8**

**Testing approach:** Generate content strings of varying lengths (including >10,000 chars). Send messages. For long content, verify the model only sees the first 10,000 characters (can check by examining the response or logs).

#### Property 17: Prompt Construction
*For any* message request, the server SHALL construct a prompt that includes both the client's `prompt` field and the `content` field.

**Validates: Requirements 4.9**

**Testing approach:** Send message requests with known prompt and content. Examine server logs or responses to verify both are included in the constructed prompt.

#### Property 18: Error Response Structure
*For any* error condition, the server SHALL return a response with structure `{"ok":false,"error":"<error_code>"}` where error is a non-empty string.

**Validates: Requirements 6.3, 6.5**

**Testing approach:** Trigger various error conditions (invalid JSON, missing fields, non-existent session, unknown format). Verify all error responses have ok:false and a non-empty error field.

#### Property 19: Session Resilience After Error
*For any* session, after an error occurs during message processing, the session SHALL remain valid and accept subsequent messages.

**Validates: Requirements 6.7**

**Testing approach:** Open a session, send a message with content that triggers a guardrail block (model-level error that occurs during execution), verify error response, then send a valid message on the same session. Verify the second message succeeds, proving the session remained valid despite the error.

#### Property 20: Error Logging
*For any* error condition, the server SHALL write diagnostic information to stderr, and all log messages SHALL be prefixed with `[IntelligenceKit]`.

**Validates: Requirements 6.6, 9.6**

**Testing approach:** Trigger various errors. Capture stderr. Verify: (1) error details appear in stderr, (2) all log lines start with `[IntelligenceKit]`.

#### Property 21: Server Continuity
*For any* sequence of messages including some that cause errors, the server SHALL continue processing all messages without crashing or exiting.

**Validates: Requirements 1.3, 6.4**

**Testing approach:** Generate a sequence of mixed valid and invalid messages. Send all to server. Verify: (1) all messages receive responses, (2) server doesn't exit prematurely.

### Round-Trip Properties

#### Property 22: Session Lifecycle Round-Trip
*For any* open-session request, the returned session ID SHALL be usable for sending messages, and after close-session, that ID SHALL no longer be usable.

**Validates: Requirements 3.1, 3.5**

**Testing approach:** Open session → get session_id → send message (verify success) → close session → send message (verify session_not_found). This tests the complete lifecycle.

### Edge Case Properties

#### Property 23: Empty Prompt Rejection
*For any* message request with an empty string as `prompt`, the server SHALL return `{"ok":false,"error":"prompt_required"}`.

**Validates: Requirements 4.10**

**Testing approach:** Send message requests with `prompt: ""`. Verify error response.

#### Property 24: Empty Content Rejection
*For any* message request with an empty string as `content`, the server SHALL return `{"ok":false,"error":"content_required"}`.

**Validates: Requirements 4.11**

**Testing approach:** Send message requests with `content: ""`. Verify error response.

#### Property 25: Maximum Content Length Boundary
*For any* message request with `content` of exactly 10,000 characters, the server SHALL process it without truncation.

**Validates: Requirements 4.8**

**Testing approach:** Generate content of exactly 10,000 characters. Send message. Verify no truncation occurs (all content is processed).

## Testing Strategy

### Dual Testing Approach

This project requires both unit tests and property-based tests to ensure comprehensive correctness:

**Unit Tests** focus on:
- Specific command examples (open-session, message, close-session, check-availability, shutdown)
- Signal handling scenarios (SIGINT, SIGTERM, EOF)
- Logging output verification (startup message, session logs, error logs)
- Availability check responses for different unavailability reasons
- Specific error cases (guardrail blocks, model unavailable)

**Property-Based Tests** focus on:
- Universal properties that hold across all inputs (e.g., one response per request, NDJSON format)
- Randomized message sequences to find edge cases
- Invariants that must be maintained (e.g., session ID uniqueness, error response structure)
- Field validation across all request types
- Session lifecycle and state management

Together, these approaches provide comprehensive coverage: unit tests catch concrete bugs in specific scenarios, while property tests verify general correctness across the input space.

### Property-Based Testing Configuration

**Framework:** Manual implementation using XCTest with randomized input generation. No external dependencies required—property-based tests are implemented as simple loops generating random inputs.

**Test Configuration:**
- Minimum 100 iterations per property test (due to randomization)
- Each test must reference its design document property in a comment
- Tag format: `// Feature: intelligence-kit, Property N: <property title>`
- Use `UUID().uuidString` for random session IDs
- Use `Int.random(in:)` and `String.random(length:)` for random values

**Example:**
```swift
// Feature: intelligence-kit, Property 1: One Response Per Request
func testOneResponsePerRequest() async throws {
    let server = try await TestServer.start()
    
    // Test 100 random requests
    for _ in 0..<100 {
        let request = generateRandomRequest()
        let responses = try await server.send(request)
        
        XCTAssertEqual(responses.count, 1, "Should receive exactly one response per request")
    }
    
    try await server.shutdown()
}

// Helper to generate random valid requests
func generateRandomRequest() -> String {
    let commands = ["open-session", "check-availability"]
    let command = commands.randomElement()!
    
    switch command {
    case "open-session":
        return """
        {"command":"open-session","instructions":"Test instructions"}
        """
    case "check-availability":
        return """
        {"command":"check-availability"}
        """
    default:
        fatalError()
    }
}
```

**Note:** This approach maintains the zero-dependency principle—even test targets use only Apple frameworks (XCTest, Foundation). Property-based testing is achieved through simple randomized loops rather than external PBT frameworks.

### Unit Testing Strategy

**Framework:** Use XCTest (Apple's standard testing framework).

**Test Organization:**
- `SessionManagerTests.swift` - Session creation, closure, idle timeout, concurrent sessions
- `MessageExecutorTests.swift` - Prompt construction, content truncation, output formats, guided generation
- `AvailabilityTests.swift` - Availability checking, reason categorization
- `ProtocolTests.swift` - NDJSON parsing, response formatting, error handling
- `CommandRouterTests.swift` - Command routing, field validation, error responses
- `IntegrationTests.swift` - End-to-end flows (open session → send message → close session)

**Key Test Cases:**
- Startup message logging
- Session ID uniqueness
- Idle timeout (121 seconds)
- Content truncation at 10,000 characters
- Output format validation (string_list, text, invalid)
- Error response structure
- Signal handling (SIGINT, SIGTERM, EOF)
- Availability check responses (available, not enabled, old macOS, unsupported hardware)
- Guardrail error handling
- Session resilience after errors

### Testing Challenges and Mitigations

**Challenge 1: Foundation Models Dependency**
- Mitigation: Mock SystemLanguageModel and LanguageModelSession for unit tests
- Use protocol abstraction for testability
- Test with real Foundation Models in integration tests (requires macOS 26+)
- Document that some tests require Apple Intelligence to be enabled

**Challenge 2: Time-Dependent Behavior (Idle Timeout)**
- Mitigation: Make timeout configurable for testing (e.g., 2 seconds in tests vs 120 in production)
- Use dependency injection for time source (allow mocking Date.now)
- Test idle monitoring logic separately from actual timing

**Challenge 3: Async/Await and Actors**
- Mitigation: Use async test methods (`func testX() async throws`)
- Ensure proper task cancellation in teardown
- Test actor isolation with concurrent operations

**Challenge 4: Signal Handling**
- Mitigation: Test signal handler logic separately from actual signal delivery
- Use flag-based shutdown triggers in tests
- Verify cleanup logic is called correctly

**Challenge 5: NDJSON Protocol Testing**
- Mitigation: Use in-memory pipes or temporary files for stdin/stdout in tests
- Capture stdout/stderr separately
- Verify line-by-line parsing and response formatting

### Test Coverage Goals

- **Line Coverage:** >80% for core logic (SessionManager, MessageExecutor, CommandRouter)
- **Branch Coverage:** >70% for error handling paths
- **Property Coverage:** 100% of identified correctness properties implemented as tests
- **Integration Coverage:** Key end-to-end flows tested (session lifecycle, message execution, availability check)

### Continuous Testing

- Run unit tests on every commit
- Run property tests (100 iterations) on every commit
- Run extended property tests (1000 iterations) nightly
- Monitor for flaky tests (especially time-dependent and async tests)
- Track test execution time (property tests and async tests may be slower)
- Require Apple Silicon Mac with macOS 26+ for full test suite

