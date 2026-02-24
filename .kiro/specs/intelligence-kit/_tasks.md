# Implementation Plan: IntelligenceKit

## Overview

IntelligenceKit is a generic Foundation Models gateway — a persistent Swift server that accepts prompts from the Rust client and returns structured results via two output formats (`string_list` and `text`). It contains zero task-specific logic. All intelligence about what to do (tagging, summarizing, redacting) lives in the Rust client; the Swift server just executes prompts through Apple's on-device LLM.

## Prerequisites

Before starting, verify:
- macOS 26 beta installed (Foundation Models requirement)
- Apple Silicon Mac (M1 or later)
- Apple Intelligence enabled in System Settings
- Xcode 26 beta with Swift 6.2
- `jarvis-listen/` exists as reference for sidecar patterns

## Tasks

- [ ] 1. Set up Swift package structure
  - Create `intelligence-kit/` directory as sibling to `jarvis-listen/`
  - Create `Package.swift`: Swift 6.2, platform macOS v26, executable target `IntelligenceKit`
  - Enable StrictConcurrency in swiftSettings (matching JarvisListen)
  - No external dependencies — FoundationModels is a system framework, no package dependency needed
  - Create `Sources/IntelligenceKit/` directory
  - Create `Tests/IntelligenceKitTests/` directory
  - Verify `swift build` compiles (empty main.swift with just `print("hello")`)
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8_

- [ ] 2. Implement JSON request/response models
  - [ ] 2.1 Create Models.swift with Codable structs
    - Implement `Request` struct with fields: `command` (String), `sessionId` (String?), `instructions` (String?), `prompt` (String?), `content` (String?), `outputFormat` (String?)
    - Add `CodingKeys` enum mapping `sessionId` → `"session_id"`, `outputFormat` → `"output_format"`
    - Implement `OkResponse` with `ok: Bool = true`
    - Implement `OpenSessionResponse` with `ok: Bool = true`, `sessionId: String`, CodingKeys for `session_id`
    - Implement `StringListResponse` with `ok: Bool = true`, `result: [String]`
    - Implement `TextResponse` with `ok: Bool = true`, `result: String`
    - Implement `AvailabilityResponse` with `ok: Bool = true`, `available: Bool`, `reason: String?`
    - Implement `ErrorResponse` with `ok: Bool = false`, `error: String`
    - Add `Equatable` conformance to all types (needed for round-trip tests)
    - _Requirements: 2.1, 2.2, 2.6, 2.7, 4.5, 4.6_

  - [ ]* 2.2 Write unit tests for JSON serialization (ModelsTests.swift)
    - Test `Request` encode/decode round-trip with all fields populated
    - Test `Request` with missing optional fields — all decode as nil
    - Test `OpenSessionResponse` — verify `session_id` key in JSON output
    - Test `StringListResponse` — verify `result` is JSON array
    - Test `TextResponse` — verify `result` is JSON string
    - Test `AvailabilityResponse` for both available=true and available=false with reason
    - Test `ErrorResponse` — verify `ok` is always false
    - **Property 8: Codable Round-Trip**
    - Run 100 iterations with randomized field values
    - _Requirements: 2.1, 2.2_

- [ ] 3. Implement logging and signal handling utilities
  - [ ] 3.1 Add logging functions to main.swift
    - Implement `logToStderr(_ message: String)` — writes `[IntelligenceKit] <message>\n` to stderr
    - Implement `logError(_ message: String)` — calls `logToStderr("Error: <message>")`
    - Implement `logWarning(_ message: String)` — calls `logToStderr("Warning: <message>")`
    - Implement `writeResponse<T: Encodable>(_ response: T)` — JSON encode → stdout → newline
    - Define `ExitCode.success = 0`, `ExitCode.error = 1`
    - _Requirements: 9.6, 9.7, 9.8_

  - [ ] 3.2 Add SignalHandler class to main.swift
    - Implement `SignalHandler.setup(onShutdown:)` — registers handlers for SIGTERM, SIGINT, SIGPIPE
    - Store shutdown closure in static var
    - On signal: call shutdown closure, then `exit(0)`
    - _Requirements: 7.1, 7.2_

- [ ] 4. Implement Session with generic prompt execution
  - [ ] 4.1 Create Session.swift
    - Define `@Generable struct StringListResult` with `@Guide(description: "list of text items as requested in the prompt") var items: [String]`
    - Define `@Generable struct TextResult` with `@Guide(description: "the text output as requested in the prompt") var text: String`
    - Define `enum SessionError: Error` with case `unknownOutputFormat(String)`
    - Define `Session` class with properties: `id: String`, `lmSession: LanguageModelSession`, `lastActive: Date`
    - Define `static let maxContentLength = 10_000`
    - Implement `init(id:lmSession:)` — sets lastActive to `Date()`
    - Implement `touch()` — updates `lastActive` to `Date()`
    - Implement `execute(prompt:content:outputFormat:) async throws -> Encodable`:
      - Truncate content to `maxContentLength`
      - Combine: `"\(prompt)\n\nContent:\n\(truncated)"`
      - Switch on `outputFormat`:
        - `"string_list"` → `lmSession.respond(to:generating: StringListResult.self)` → return `StringListResponse(result: result.items)`
        - `"text"` → `lmSession.respond(to:generating: TextResult.self)` → return `TextResponse(result: result.text)`
        - default → throw `SessionError.unknownOutputFormat(outputFormat)`
    - **Key design point:** Session contains NO task-specific logic. No tagging prompts, no summarization instructions. The `prompt` parameter contains all instructions from the client.
    - _Requirements: 4.1, 4.2, 4.3, 4.5, 4.6, 4.7, 4.8, 4.9, 4.12_

  - [ ]* 4.2 Write unit test for content truncation
    - **Property 7: Content Truncation Safety**
    - Test with content < 10,000 chars (no truncation)
    - Test with content = 10,000 chars (boundary)
    - Test with content > 10,000 chars (verify only first 10,000 sent)
    - _Requirements: 4.8_

- [ ] 5. Implement SessionManager
  - [ ] 5.1 Create SessionManager.swift
    - Define `sessions: [String: Session]` dictionary
    - Define `idleTimeout: TimeInterval = 120`
    - Define `cleanupInterval: TimeInterval = 30`
    - Implement `open(instructions: String?) -> String`:
      - Generate ID: `String(UUID().uuidString.prefix(8).lowercased())`
      - Default instructions: `"You are a helpful assistant. Follow the prompt instructions precisely."`
      - Create `LanguageModelSession(instructions:)` with provided or default instructions
      - Create `Session(id:lmSession:)`, store in dictionary, return id
    - Implement `get(id: String) -> Session?` — lookup in dictionary
    - Implement `close(id: String)` — remove from dictionary
    - Implement `closeAll()` — remove all, log count
    - Implement `startIdleMonitor() async`:
      - Loop: `try? await Task.sleep(for: .seconds(cleanupInterval))`
      - Find sessions where `Date().timeIntervalSince(lastActive) > idleTimeout`
      - Remove stale sessions, log warning for each
    - _Requirements: 3.1, 3.2, 3.5, 3.6, 3.7, 3.8, 3.9_

  - [ ]* 5.2 Write unit tests for SessionManager (SessionManagerTests.swift)
    - Test `open()` returns unique session IDs (open 10, all different)
    - Test `get()` returns session after open
    - Test `get()` returns nil for unknown ID
    - Test `close()` removes session — subsequent `get()` returns nil
    - Test `closeAll()` empties all sessions
    - Test multiple concurrent sessions are independent
    - **Property 3: Session Isolation**
    - **Property 4: Session Lifecycle**
    - _Requirements: 3.1, 3.2, 3.4, 3.5_

- [ ] 6. Implement availability checking
  - [ ] 6.1 Create Availability.swift
    - Implement `checkAvailability() -> AvailabilityResponse`
    - Check `SystemLanguageModel.default.availability`
    - Handle `.available` → `AvailabilityResponse(available: true, reason: nil)`
    - Handle `.unavailable(.deviceNotEligible)` → "Device not eligible — Apple Silicon required"
    - Handle `.unavailable(.appleIntelligenceNotEnabled)` → "Apple Intelligence not enabled in System Settings"
    - Handle `.unavailable(.modelNotReady)` → "Model not ready — may still be downloading"
    - Handle `@unknown default` for future availability states
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [ ] 7. Implement server main loop and command routing
  - [ ] 7.1 Create main entry point with read loop
    - Define `@main struct IntelligenceKitServer` with `static func main() async`
    - Create `SessionManager` instance
    - Set up `SignalHandler` with shutdown closure that calls `sessionManager.closeAll()`
    - Start idle monitor: `Task { await sessionManager.startIdleMonitor() }`
    - Log "Server ready" to stderr
    - Enter `while let line = readLine()` loop:
      - Trim whitespace, skip empty lines
      - Parse JSON — if invalid, respond `invalid_json` error and **continue** (don't exit)
      - Call `handleCommand()` with parsed request
    - On loop exit (EOF): log, `closeAll()`, `exit(0)`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 7.4_

  - [ ] 7.2 Implement command routing in handleCommand()
    - `"open-session"` → `sessionManager.open(instructions:)` → `OpenSessionResponse`
    - `"message"`:
      - Validate `session_id` present → `session_id_required`
      - Validate session exists → `session_not_found`
      - Call `session.touch()`
      - Validate `prompt` present and non-empty → `prompt_required`
      - Validate `content` present and non-empty → `content_required`
      - Validate `output_format` present and non-empty → `output_format_required`
      - Call `session.execute(prompt:content:outputFormat:)`
      - Catch `SessionError.unknownOutputFormat` → `unknown_output_format` error
      - Catch any other error → log to stderr, return `ErrorResponse`
      - Log session_id, output_format, content length, processing time to stderr
    - `"close-session"` → validate session_id, `sessionManager.close()` → `OkResponse`
    - `"check-availability"` → `checkAvailability()`
    - `"shutdown"` → `sessionManager.closeAll()` → `OkResponse` → `exit(0)`
    - Unknown → `unknown_command` error
    - _Requirements: 2.7, 4.4, 4.10, 4.11, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 9.2, 9.3_

  - [ ]* 7.3 Write property test for server continuity
    - **Property 1: Server Continuity**
    - Send various malformed inputs (invalid JSON, unknown commands, bad session_ids, unknown output_formats)
    - Verify server responds with error and keeps running
    - _Requirements: 2.4, 6.4, 6.7_

  - [ ]* 7.4 Write property test for NDJSON protocol integrity
    - **Property 2: NDJSON Protocol Integrity**
    - Send various messages, capture stdout
    - Verify each response is exactly one JSON line
    - Verify no non-JSON content on stdout
    - _Requirements: 2.1, 2.2, 2.3_

  - [ ]* 7.5 Write property test for error response structure
    - **Property 9: Error Response Structure**
    - Trigger all known error paths
    - Verify each has ok=false and non-empty error string
    - _Requirements: 6.5_

- [ ] 8. Checkpoint — Server compiles and unit tests pass
  - Run `swift build` in `intelligence-kit/` — no errors
  - Run `swift test` — all tests pass
  - Verify no compilation warnings
  - Ask user if questions arise

- [ ] 9. Manual testing with real Foundation Models
  - [ ] 9.1 Test server lifecycle
    - Start server: `.build/debug/IntelligenceKit`
    - Verify "Server ready" logged to stderr
    - Type `{"command":"shutdown"}` — verify clean exit
    - Start again, Ctrl+C — verify graceful shutdown
    - _Requirements: 1.1, 1.5, 1.6, 7.1, 7.2_

  - [ ] 9.2 Test availability
    - Send `{"command":"check-availability"}`
    - Verify response indicates availability
    - _Requirements: 5.1, 5.2, 5.3_

  - [ ] 9.3 Test string_list output (tagging use case)
    - Open session
    - Send: `{"command":"message","session_id":"<id>","prompt":"Generate 3-5 topic tags for this content. Each tag should be 1-3 words, lowercase.","content":"AI agents are autonomous systems...","output_format":"string_list"}`
    - Verify `result` is array of 3-5 lowercase strings
    - Close session
    - _Requirements: 4.5_

  - [ ] 9.4 Test text output (summarization use case)
    - Open session
    - Send: `{"command":"message","session_id":"<id>","prompt":"Summarize in 2-3 sentences.","content":"AI agents are autonomous systems...","output_format":"text"}`
    - Verify `result` is a string summary
    - Close session
    - _Requirements: 4.6_

  - [ ] 9.5 Test multi-message session (chunked content)
    - Open session
    - Send message with chunk 1 (string_list output)
    - Send message with chunk 2 on same session — prompt says "avoid repeating tags from previous chunks"
    - Verify tags across responses are diverse (model has context from chunk 1)
    - Close session
    - _Requirements: 3.3_

  - [ ] 9.6 Test different prompts on same server (task-agnostic verification)
    - Without restarting server:
    - Open session → send tagging prompt → verify string_list result
    - Open different session → send summarization prompt → verify text result
    - Open another session → send PII redaction prompt → verify text result
    - All three work without any server code changes
    - **Property 6: Task Agnosticism**
    - _Requirements: 4.12_

  - [ ] 9.7 Test error handling
    - Send invalid JSON → verify `invalid_json`, server continues
    - Send `{"command":"message"}` (no session_id) → verify `session_id_required`
    - Send message with bad session_id → verify `session_not_found`
    - Send message without prompt → verify `prompt_required`
    - Send message without content → verify `content_required`
    - Send message without output_format → verify `output_format_required`
    - Send message with `output_format: "invalid"` → verify `unknown_output_format`
    - Send unknown command → verify `unknown_command`
    - Verify server stays alive through all errors
    - _Requirements: 2.4, 6.4, 6.5, 6.7_

  - [ ] 9.8 Test logging
    - Verify all log messages on stderr (not stdout)
    - Verify `[IntelligenceKit]` prefix on all log lines
    - Verify session open/close logged
    - Verify message processing time logged
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

  - [ ] 9.9 Test idle timeout
    - Open a session
    - Wait >120 seconds without sending messages
    - Attempt to use the session → verify `session_not_found` (auto-closed)
    - Check stderr for auto-close warning log
    - _Requirements: 3.6, 3.7_

- [ ] 10. Final checkpoint
  - Run `swift build` — no errors or warnings
  - Run `swift test` — all tests pass
  - All manual tests from task 9 pass
  - All correctness properties from design document hold
  - Ask user if questions arise

## Files Created

| File | Purpose |
|------|---------|
| `intelligence-kit/Package.swift` | Swift package manifest |
| `intelligence-kit/Sources/IntelligenceKit/main.swift` | Server: read loop, routing, logging, signals |
| `intelligence-kit/Sources/IntelligenceKit/Models.swift` | Request/Response Codable types (task-agnostic) |
| `intelligence-kit/Sources/IntelligenceKit/SessionManager.swift` | Session lifecycle + idle monitor |
| `intelligence-kit/Sources/IntelligenceKit/Session.swift` | Generic prompt execution + 2 output formats |
| `intelligence-kit/Sources/IntelligenceKit/Availability.swift` | SystemLanguageModel availability check |
| `intelligence-kit/Tests/IntelligenceKitTests/ModelsTests.swift` | JSON round-trip tests |
| `intelligence-kit/Tests/IntelligenceKitTests/SessionManagerTests.swift` | Session lifecycle tests |

## Notes

- Tasks marked with `*` are optional test tasks and can be skipped for faster MVP
- The server is task-agnostic: NO tagging prompts, NO summarization logic in Swift code
- All task intelligence lives in Rust client (prompts, chunking, post-processing)
- Adding a new capability = add a prompt string in Rust. Zero Swift changes
- Two output formats (`string_list`, `text`) cover all foreseeable use cases
- Only add a new output format if a completely new result shape is needed (rare)
- Property tests requiring actual model inference are validated in manual testing (Task 9)
- This is Part 1 only — Tauri integration (IntelligenceKitProvider in Rust) is Part 2
