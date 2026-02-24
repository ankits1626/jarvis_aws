# Design Document: IntelligenceKit

## Overview

IntelligenceKit is a **generic Foundation Models gateway** — a persistent Swift server that accepts prompts from the Rust client and returns structured results. It contains zero task-specific logic. No tagging prompts, no summarization logic, no PII rules. All intelligence lives in the Rust client; the Swift server just executes prompts through Apple's on-device LLM.

The server manages multiple concurrent sessions over stdin/stdout using newline-delimited JSON. Each session wraps a `LanguageModelSession` with multi-turn context. Two pre-compiled output formats (`string_list` and `text`) cover all foreseeable use cases via guided generation.

**Adding a new capability (e.g., "extract action items") = add a prompt string in Rust. Zero Swift changes.**

Key design decisions:
- **Task-agnostic**: Server doesn't know what tagging or summarization means — it just runs prompts
- **Two output formats**: `string_list` → `[String]`, `text` → `String` — covers everything
- **Guided generation**: `@Generable` structs guarantee valid structured output
- **Persistent server**: Spawned once, handles many requests via sessions
- **Multiple concurrent sessions**: Client can tag one gem while summarizing another
- **Idle timeout**: Auto-closes stale sessions after 120s
- **No external dependencies**: Only Apple frameworks (Foundation, FoundationModels)

## Architecture

### Responsibility Split

```
Rust Client (the brains)                    Swift Server (the gateway)
━━━━━━━━━━━━━━━━━━━━━━━━                    ━━━━━━━━━━━━━━━━━━━━━━━━━

Knows about:                                Knows about:
• Tag prompt templates                      • Session lifecycle management
• Summary prompt templates                  • LanguageModelSession wrapper
• PII redaction prompt templates            • Guided generation (2 output formats)
• Chat prompt templates                     • Idle timeout cleanup
• Content chunking logic                    • Availability checking
• Which output_format to use                • stdin/stdout NDJSON protocol
• Tag merging / dedup                       • Error handling + recovery
• Post-processing results                   • Signal handling
• Future: any new prompts                   • Logging to stderr

Changes when: new task added                Changes when: new output_format needed
                                            (rarely — string_list and text cover most)
```

### Client-Server Diagram

```
┌──────────────────────────┐         stdin/stdout         ┌──────────────────────────────┐
│      Tauri / Rust        │         (NDJSON)             │      IntelligenceKit         │
│      (Client)            │◄────────────────────────────►│      (Server)                │
│                          │                               │                              │
│  IntelligenceKitProvider │                               │  ┌────────────────────────┐  │
│                          │  → check-availability         │  │   SessionManager       │  │
│  Holds prompt templates: │  ← {available: true}          │  │                        │  │
│  • TAG_PROMPT            │                               │  │  sessions: HashMap     │  │
│  • SUMMARY_PROMPT        │  → open-session               │  │   "s1" → Session {     │  │
│  • REDACT_PROMPT         │  ← {session_id: "s1"}        │  │     lm_session,        │  │
│  • CHAT_PROMPT           │                               │  │     last_active        │  │
│  • (future prompts...)   │  → message {                  │  │   }                    │  │
│                          │      prompt: TAG_PROMPT,       │  │                        │  │
│  Knows:                  │      content: "...",           │  │  Knows:                │  │
│  • When to chunk content │      output_format:            │  │  • string_list format  │  │
│  • How to merge results  │        "string_list"           │  │  • text format         │  │
│  • Post-processing       │    }                           │  │  • Session management  │  │
│                          │  ← {result: ["tag1","tag2"]}  │  │  • Idle timeout        │  │
│                          │                               │  └────────────────────────┘  │
│                          │  → close-session (s1)         │                              │
│                          │  ← {ok: true}                 │                              │
└──────────────────────────┘                               └──────────────────────────────┘
```

### Server Internal Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         main.swift                               │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────┐  │
│  │ Read Loop    │  │ Signal       │  │ Command Router        │  │
│  │ (stdin lines)│  │ Handler      │  │ open/message/close/   │  │
│  │              │  │ SIGTERM/INT  │  │ check-avail/shutdown  │  │
│  └──────────────┘  └──────────────┘  └───────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼──────────────────┐
              ▼               ▼                  ▼
┌──────────────────┐ ┌────────────────┐ ┌─────────────────────────┐
│ SessionManager   │ │ Availability   │ │ IdleTimeoutMonitor      │
│ .swift           │ │ .swift         │ │ (background Task)       │
│                  │ │                │ │                         │
│ open(instr)→id   │ │ checkAvail()   │ │ Runs every 30s          │
│ get(id)→session  │ │ → Response     │ │ Closes sessions idle    │
│ close(id)        │ │                │ │ > 120s                  │
│ closeAll()       │ │                │ │                         │
└──────────────────┘ └────────────────┘ └─────────────────────────┘
         │
         ▼
┌──────────────────────────────────────┐
│ Session (per session_id)             │
│                                      │
│  lmSession: LanguageModelSession     │
│  lastActive: Date                    │
│                                      │
│  execute(prompt, content, format)    │
│    → runs prompt through lmSession   │
│    → uses @Generable output format   │
│    → returns structured result       │
│                                      │
│  Two output formats:                 │
│  ┌─────────────────────────────────┐ │
│  │ "string_list" → StringListResult│ │
│  │   @Generable { items: [String] }│ │
│  │                                 │ │
│  │ "text" → TextResult             │ │
│  │   @Generable { text: String }   │ │
│  └─────────────────────────────────┘ │
└──────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────┐
│  Apple Foundation Models              │
│  (on-device 3B LLM via ANE)          │
└──────────────────────────────────────┘
```

## Components and Interfaces

### main.swift — Server Entry Point

**Responsibilities:**
- Run the stdin read loop
- Route commands to handlers
- Manage signal handling (SIGTERM, SIGINT, SIGPIPE)
- Write responses to stdout
- Log diagnostics to stderr

```swift
import Foundation
import FoundationModels

// MARK: - Exit Codes

enum ExitCode {
    static let success: Int32 = 0
    static let error: Int32 = 1
}

// MARK: - Logging

func logToStderr(_ message: String) {
    let data = Data(("[IntelligenceKit] " + message + "\n").utf8)
    FileHandle.standardError.write(data)
}

func logError(_ message: String) {
    logToStderr("Error: \(message)")
}

func logWarning(_ message: String) {
    logToStderr("Warning: \(message)")
}

// MARK: - Signal Handler

class SignalHandler {
    static var shutdownHandler: (() -> Void)?

    static func setup(onShutdown: @escaping () -> Void) {
        shutdownHandler = onShutdown
        signal(SIGTERM) { _ in SignalHandler.shutdownHandler?(); exit(0) }
        signal(SIGINT)  { _ in SignalHandler.shutdownHandler?(); exit(0) }
        signal(SIGPIPE) { _ in SignalHandler.shutdownHandler?(); exit(0) }
    }
}

// MARK: - Response Writer

func writeResponse<T: Encodable>(_ response: T) {
    let encoder = JSONEncoder()
    if let data = try? encoder.encode(response) {
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }
}

// MARK: - Main

@main
struct IntelligenceKitServer {
    static func main() async {
        let sessionManager = SessionManager()

        SignalHandler.setup {
            logToStderr("Shutting down...")
            sessionManager.closeAll()
        }

        Task { await sessionManager.startIdleMonitor() }

        logToStderr("Server ready")

        while let line = readLine() {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }

            guard let data = trimmed.data(using: .utf8),
                  let request = try? JSONDecoder().decode(Request.self, from: data) else {
                writeResponse(ErrorResponse(error: "invalid_json"))
                continue
            }

            await handleCommand(request, sessionManager: sessionManager)
        }

        // EOF — parent process died
        logToStderr("stdin EOF — shutting down")
        sessionManager.closeAll()
        exit(ExitCode.success)
    }

    static func handleCommand(_ request: Request, sessionManager: SessionManager) async {
        switch request.command {
        case "open-session":
            let sessionId = sessionManager.open(instructions: request.instructions)
            logToStderr("Opened session: \(sessionId)")
            writeResponse(OpenSessionResponse(sessionId: sessionId))

        case "message":
            guard let sessionId = request.sessionId else {
                writeResponse(ErrorResponse(error: "session_id_required"))
                return
            }
            guard let session = sessionManager.get(id: sessionId) else {
                writeResponse(ErrorResponse(error: "session_not_found"))
                return
            }
            session.touch()

            guard let prompt = request.prompt, !prompt.isEmpty else {
                writeResponse(ErrorResponse(error: "prompt_required"))
                return
            }
            guard let content = request.content, !content.isEmpty else {
                writeResponse(ErrorResponse(error: "content_required"))
                return
            }
            guard let outputFormat = request.outputFormat, !outputFormat.isEmpty else {
                writeResponse(ErrorResponse(error: "output_format_required"))
                return
            }

            let startTime = CFAbsoluteTimeGetCurrent()
            do {
                let result = try await session.execute(
                    prompt: prompt,
                    content: content,
                    outputFormat: outputFormat
                )
                let elapsed = CFAbsoluteTimeGetCurrent() - startTime
                logToStderr("Session \(sessionId): \(outputFormat), \(content.count) chars, \(String(format: "%.1f", elapsed))s")
                writeResponse(result)
            } catch SessionError.unknownOutputFormat(let fmt) {
                writeResponse(ErrorResponse(error: "unknown_output_format: \(fmt)"))
            } catch {
                logError("Session \(sessionId): \(error)")
                writeResponse(ErrorResponse(error: "\(error)"))
            }

        case "close-session":
            guard let sessionId = request.sessionId else {
                writeResponse(ErrorResponse(error: "session_id_required"))
                return
            }
            sessionManager.close(id: sessionId)
            logToStderr("Closed session: \(sessionId)")
            writeResponse(OkResponse())

        case "check-availability":
            writeResponse(checkAvailability())

        case "shutdown":
            logToStderr("Shutdown requested")
            sessionManager.closeAll()
            writeResponse(OkResponse())
            exit(ExitCode.success)

        default:
            writeResponse(ErrorResponse(error: "unknown_command"))
        }
    }
}
```

### Models.swift — JSON Request/Response Types

```swift
import Foundation

// MARK: - Request

struct Request: Codable, Equatable {
    let command: String           // open-session, message, close-session, check-availability, shutdown
    let sessionId: String?        // required for message, close-session
    let instructions: String?     // optional for open-session
    let prompt: String?           // required for message: the full instructions for the model
    let content: String?          // required for message: the input text
    let outputFormat: String?     // required for message: "string_list" or "text"

    enum CodingKeys: String, CodingKey {
        case command, instructions, prompt, content
        case sessionId = "session_id"
        case outputFormat = "output_format"
    }
}

// MARK: - Responses

struct OkResponse: Codable, Equatable {
    let ok: Bool = true
}

struct OpenSessionResponse: Codable, Equatable {
    let ok: Bool = true
    let sessionId: String

    enum CodingKeys: String, CodingKey {
        case ok
        case sessionId = "session_id"
    }
}

struct StringListResponse: Codable, Equatable {
    let ok: Bool = true
    let result: [String]
}

struct TextResponse: Codable, Equatable {
    let ok: Bool = true
    let result: String
}

struct AvailabilityResponse: Codable, Equatable {
    let ok: Bool = true
    let available: Bool
    let reason: String?
}

struct ErrorResponse: Codable, Equatable {
    let ok: Bool = false
    let error: String
}
```

### Session.swift — Generic Prompt Execution

**Responsibilities:**
- Wrap a `LanguageModelSession` with idle tracking
- Execute any prompt with any supported output format
- No task-specific logic — the prompt contains all instructions

```swift
import Foundation
import FoundationModels

// MARK: - Guided Generation Types

@Generable
struct StringListResult {
    @Guide(description: "list of text items as requested in the prompt")
    var items: [String]
}

@Generable
struct TextResult {
    @Guide(description: "the text output as requested in the prompt")
    var text: String
}

// MARK: - Session Errors

enum SessionError: Error {
    case unknownOutputFormat(String)
}

// MARK: - Session

class Session {
    let id: String
    let lmSession: LanguageModelSession
    private(set) var lastActive: Date

    static let maxContentLength = 10_000

    init(id: String, lmSession: LanguageModelSession) {
        self.id = id
        self.lmSession = lmSession
        self.lastActive = Date()
    }

    func touch() {
        lastActive = Date()
    }

    /// Execute a prompt against the session's LanguageModelSession.
    /// The prompt comes from the client — the server is task-agnostic.
    func execute(prompt: String, content: String, outputFormat: String) async throws -> Encodable {
        let truncated = String(content.prefix(Self.maxContentLength))
        let fullPrompt = "\(prompt)\n\nContent:\n\(truncated)"

        switch outputFormat {
        case "string_list":
            let result = try await lmSession.respond(to: fullPrompt, generating: StringListResult.self)
            return StringListResponse(result: result.items)
        case "text":
            let result = try await lmSession.respond(to: fullPrompt, generating: TextResult.self)
            return TextResponse(result: result.text)
        default:
            throw SessionError.unknownOutputFormat(outputFormat)
        }
    }
}
```

### SessionManager.swift — Session Lifecycle

```swift
import Foundation
import FoundationModels

class SessionManager {
    private var sessions: [String: Session] = [:]
    private let idleTimeout: TimeInterval = 120
    private let cleanupInterval: TimeInterval = 30

    func open(instructions: String? = nil) -> String {
        let id = String(UUID().uuidString.prefix(8).lowercased())
        let sessionInstructions = instructions ?? "You are a helpful assistant. Follow the prompt instructions precisely."
        let lmSession = LanguageModelSession(instructions: sessionInstructions)
        let session = Session(id: id, lmSession: lmSession)
        sessions[id] = session
        return id
    }

    func get(id: String) -> Session? {
        return sessions[id]
    }

    func close(id: String) {
        sessions.removeValue(forKey: id)
    }

    func closeAll() {
        let count = sessions.count
        sessions.removeAll()
        logToStderr("Closed \(count) session(s)")
    }

    func startIdleMonitor() async {
        while true {
            try? await Task.sleep(for: .seconds(cleanupInterval))
            let now = Date()
            let stale = sessions.filter { now.timeIntervalSince($0.value.lastActive) > idleTimeout }
            for (id, _) in stale {
                sessions.removeValue(forKey: id)
                logWarning("Auto-closed idle session: \(id)")
            }
        }
    }
}
```

### Availability.swift — System Check

```swift
import FoundationModels

func checkAvailability() -> AvailabilityResponse {
    let model = SystemLanguageModel.default

    switch model.availability {
    case .available:
        return AvailabilityResponse(available: true, reason: nil)
    case .unavailable(let reason):
        let reasonString: String
        switch reason {
        case .deviceNotEligible:
            reasonString = "Device not eligible — Apple Silicon required"
        case .appleIntelligenceNotEnabled:
            reasonString = "Apple Intelligence not enabled in System Settings"
        case .modelNotReady:
            reasonString = "Model not ready — may still be downloading"
        @unknown default:
            reasonString = "Unknown unavailability reason"
        }
        return AvailabilityResponse(available: false, reason: reasonString)
    @unknown default:
        return AvailabilityResponse(available: false, reason: "Unknown availability status")
    }
}
```

## Data Models

### Wire Protocol Examples

**Tagging (prompt comes from Rust):**
```
→ {"command":"open-session"}
← {"ok":true,"session_id":"a1b2c3d4"}

→ {"command":"message","session_id":"a1b2c3d4","prompt":"Generate 3-5 topic tags for this content. Each tag should be 1-3 words, lowercase, specific to the content. Avoid generic tags like 'article' or 'content'.","content":"AI agents are autonomous systems that can reason...","output_format":"string_list"}
← {"ok":true,"result":["ai agents","autonomous systems","productivity"]}

→ {"command":"close-session","session_id":"a1b2c3d4"}
← {"ok":true}
```

**Summarization (same server, different prompt from Rust):**
```
→ {"command":"open-session"}
← {"ok":true,"session_id":"e5f6g7h8"}

→ {"command":"message","session_id":"e5f6g7h8","prompt":"Summarize this content in 2-3 sentences. Be concise and capture the key points.","content":"AI agents are autonomous systems...","output_format":"text"}
← {"ok":true,"result":"AI agents are autonomous systems capable of reasoning and taking actions. They are increasingly used in productivity tools and software automation."}

→ {"command":"close-session","session_id":"e5f6g7h8"}
← {"ok":true}
```

**PII redaction (same server, different prompt from Rust):**
```
→ {"command":"message","session_id":"x1y2z3","prompt":"Replace all personal information (names, emails, phone numbers, addresses) with [REDACTED]. Return the full text with redactions applied.","content":"John Smith (john@example.com) called about...","output_format":"text"}
← {"ok":true,"result":"[REDACTED] ([REDACTED]) called about..."}
```

**Chunked tags (multi-message session, model has context):**
```
→ {"command":"open-session"}
← {"ok":true,"session_id":"c1d2e3f4"}

→ {"command":"message","session_id":"c1d2e3f4","prompt":"Generate 3-5 topic tags for this content chunk. Tags should be 1-3 words, lowercase.","content":"<first 8000 chars>","output_format":"string_list"}
← {"ok":true,"result":["machine learning","neural networks","training"]}

→ {"command":"message","session_id":"c1d2e3f4","prompt":"Generate 3-5 more topic tags for this next chunk. Avoid repeating tags from previous chunks.","content":"<next 8000 chars>","output_format":"string_list"}
← {"ok":true,"result":["deployment","inference","edge computing"]}

→ {"command":"close-session","session_id":"c1d2e3f4"}
← {"ok":true}
```

**Error — server continues:**
```
→ not valid json
← {"ok":false,"error":"invalid_json"}
→ {"command":"message","session_id":"expired123","prompt":"...","content":"...","output_format":"string_list"}
← {"ok":false,"error":"session_not_found"}
(server keeps running)
```

### File System Layout

```
intelligence-kit/                              # sibling to jarvis-listen/
├── Package.swift                              # Swift 6.2, macOS 26+
├── Sources/IntelligenceKit/
│   ├── main.swift                             # Server: read loop, routing, logging, signals
│   ├── Models.swift                           # Request/Response Codable types
│   ├── SessionManager.swift                   # Session lifecycle + idle monitor
│   ├── Session.swift                          # Generic prompt execution + output formats
│   └── Availability.swift                     # SystemLanguageModel availability check
└── Tests/IntelligenceKitTests/
    ├── ModelsTests.swift                      # JSON encode/decode round-trip tests
    └── SessionManagerTests.swift              # Session lifecycle tests
```

## Correctness Properties

### Property 1: Server Continuity

*For any* malformed input (invalid JSON, unknown command, bad session_id, unknown output_format), the server SHALL return an error response and continue reading — it never crashes or exits from a bad message.

**Validates: Requirements 2.4, 6.4, 6.7**

### Property 2: NDJSON Protocol Integrity

*For any* message received, the server SHALL write exactly one JSON line to stdout. stdout never contains non-JSON content. All diagnostics go to stderr.

**Validates: Requirements 2.1, 2.2, 2.3**

### Property 3: Session Isolation

*For any* two concurrent sessions, messages on session "s1" SHALL NOT affect the state or responses of session "s2".

**Validates: Requirements 3.2**

### Property 4: Session Lifecycle

*For any* session_id returned by `open-session`, that session SHALL be accessible via `message` and `close-session` until it is explicitly closed or auto-cleaned by idle timeout.

**Validates: Requirements 3.1, 3.5, 3.6**

### Property 5: Output Format Guarantee

*For any* message with `output_format: "string_list"`, the response `result` SHALL be a JSON array of strings. *For any* message with `output_format: "text"`, the response `result` SHALL be a JSON string.

**Validates: Requirements 4.5, 4.6**

### Property 6: Task Agnosticism

*For any* valid prompt string, the server SHALL execute it without knowledge of what task the prompt represents. The server SHALL contain no hardcoded prompts or task-specific logic.

**Validates: Requirements 4.12**

### Property 7: Content Truncation Safety

*For any* content string longer than 10,000 characters, the Session SHALL only send the first 10,000 characters to the model.

**Validates: Requirements 4.8**

### Property 8: Codable Round-Trip

*For any* valid Request or Response struct, encoding to JSON and decoding back SHALL produce an identical struct.

**Validates: Requirements 2.1, 2.2**

### Property 9: Error Response Structure

*For any* error condition, the JSON response SHALL contain `{"ok": false, "error": "<string>"}` with a non-empty error string.

**Validates: Requirements 6.5**

### Property 10: Idle Timeout Enforcement

*For any* session that has not received a message for longer than the idle timeout, the server SHALL automatically close it and free resources.

**Validates: Requirements 3.6, 3.7**

### Property 11: Graceful Shutdown Completeness

*When* shutdown is triggered (via command, SIGTERM, or stdin EOF), ALL active sessions SHALL be closed before the process exits.

**Validates: Requirements 7.1, 7.2, 7.4**

## Error Handling

### Error Categories

#### 1. Protocol Errors (server continues)

| Error | Cause | Response |
|-------|-------|----------|
| `invalid_json` | Line is not valid JSON | `{"ok":false,"error":"invalid_json"}` |
| `unknown_command` | `command` field not recognized | `{"ok":false,"error":"unknown_command"}` |
| `session_id_required` | `message`/`close-session` without `session_id` | `{"ok":false,"error":"session_id_required"}` |
| `session_not_found` | `session_id` doesn't exist | `{"ok":false,"error":"session_not_found"}` |
| `prompt_required` | `message` without `prompt` | `{"ok":false,"error":"prompt_required"}` |
| `content_required` | `message` without `content` | `{"ok":false,"error":"content_required"}` |
| `output_format_required` | `message` without `output_format` | `{"ok":false,"error":"output_format_required"}` |
| `unknown_output_format` | `output_format` not `string_list` or `text` | `{"ok":false,"error":"unknown_output_format: xyz"}` |

#### 2. Model Errors (server continues, session remains valid)

| Error | Cause | Response |
|-------|-------|----------|
| `model_unavailable` | Foundation Models not available | `{"ok":false,"error":"model_unavailable"}` |
| `guardrail_blocked` | Content safety filter rejected input/output | `{"ok":false,"error":"guardrail_blocked"}` |

#### 3. Shutdown Triggers (server exits)

| Trigger | Behavior |
|---------|----------|
| `shutdown` command | Close all sessions, respond `{"ok":true}`, exit 0 |
| SIGTERM / SIGINT | Close all sessions, exit 0 |
| stdin EOF | Close all sessions, exit 0 |

## Testing Strategy

### Unit Tests

**ModelsTests.swift:**
- JSON round-trip for all Request/Response types
- CodingKeys mapping (`session_id`, `output_format`)
- Missing optional fields decode as nil
- ErrorResponse always has `ok: false`
- StringListResponse result is array, TextResponse result is string

**SessionManagerTests.swift:**
- `open()` returns unique session IDs
- `get()` returns nil for unknown IDs
- `close()` removes session
- `closeAll()` empties all sessions
- Multiple concurrent sessions are independent

### Manual Testing

```bash
cd intelligence-kit && swift build
.build/debug/IntelligenceKit

# Interactive — type one line at a time:
{"command":"check-availability"}
{"command":"open-session"}
{"command":"message","session_id":"<id>","prompt":"Generate 3-5 topic tags, each 1-3 words, lowercase","content":"AI agents are autonomous systems...","output_format":"string_list"}
{"command":"message","session_id":"<id>","prompt":"Summarize in 2 sentences","content":"AI agents are autonomous systems...","output_format":"text"}
{"command":"close-session","session_id":"<id>"}

# Error cases — server should continue after each:
not json
{"command":"message"}
{"command":"message","session_id":"bad","prompt":"x","content":"y","output_format":"string_list"}
{"command":"message","session_id":"<valid>","prompt":"x","content":"y","output_format":"invalid_format"}

{"command":"shutdown"}
```
