# Requirements Document

## Introduction

IntelligenceKit is a macOS Swift server that provides a generic gateway to Apple's Foundation Models framework (on-device 3B parameter LLM). Named after "Apple Intelligence" (like WhisperKit for transcription), it serves as the Apple-specific implementation behind the generic `IntelProvider` trait in the Jarvis Tauri app.

IntelligenceKit is **task-agnostic** — it does not contain any tagging, summarization, or redaction logic. The Rust client sends a `prompt` (what to do), `content` (the input), and `output_format` (the shape of the result). The server just runs the prompt through Foundation Models and returns the structured result. All intelligence about what to do lives in the Rust client; the Swift server is a generic execution gateway.

It runs as a persistent local server over stdin/stdout using newline-delimited JSON. The Rust side (Tauri) is the client — it opens sessions, sends prompt+content messages, and receives structured results. The server manages multiple concurrent sessions, each backed by a `LanguageModelSession`. Sessions are auto-closed after idle timeout.

Runs entirely on-device using the Apple Neural Engine — no API keys, no network requests, fully private.

## Glossary

- **Foundation_Models**: Apple's on-device language model framework, shipping with macOS 26+, providing a 3B parameter LLM that runs on Apple Silicon's Neural Engine
- **Apple_Intelligence**: Apple's branding for on-device AI capabilities; IntelligenceKit is named after this
- **SystemLanguageModel**: The singleton entry point to Foundation Models — `SystemLanguageModel.default` provides the on-device model
- **LanguageModelSession**: A stateful conversation session with the on-device model, supporting multi-turn dialogue within a 4096-token context window
- **Generable**: A Swift macro (`@Generable`) that enables guided/constrained generation — the model is forced to produce output matching the annotated struct's shape via token masking
- **Guide**: A Swift macro (`@Guide`) that provides natural-language descriptions to steer the model's output for specific struct fields
- **Guided_Generation**: A technique where the model's token selection is constrained at decoding time to match a declared output schema — guarantees valid structured output (no parsing failures)
- **Output_Format**: One of the pre-compiled `@Generable` types that constrains the model's response shape — `string_list` (returns `[String]`) or `text` (returns `String`)
- **Neural_Engine**: Apple's dedicated ML accelerator on Apple Silicon chips (ANE), used by Foundation Models for inference
- **Sidecar**: An external binary bundled with the Tauri app, spawned as a child process; IntelligenceKit runs as a persistent sidecar (spawned once at app startup)
- **Guardrails**: Built-in content safety filters in Foundation Models that cannot be disabled; may occasionally block benign content
- **IntelProvider**: The generic Rust trait that abstracts intelligence backends; IntelligenceKit is one implementation (others: KeywordProvider, future ClaudeProvider)
- **Session**: A server-side `LanguageModelSession` instance identified by a `session_id`; maintains multi-turn context across messages
- **NDJSON**: Newline-Delimited JSON — each message is a single JSON object on one line, separated by `\n`; the wire protocol between client and server

## Requirements

### Requirement 1: Server Lifecycle

**User Story:** As the Tauri app, I want IntelligenceKit to run as a persistent server process, so that I can send multiple requests without the overhead of spawning a new process each time.

#### Acceptance Criteria

1. WHEN IntelligenceKit starts, THE Server SHALL enter a read loop, continuously reading newline-delimited JSON messages from stdin
2. THE Server SHALL process each message and write exactly one newline-delimited JSON response to stdout
3. THE Server SHALL stay alive indefinitely until it receives a `shutdown` command or SIGTERM/SIGINT
4. THE Server SHALL handle messages sequentially (read one, process, respond, read next)
5. THE Server SHALL log a startup message to stderr when ready to accept messages
6. WHEN the `shutdown` command is received, THE Server SHALL close all active sessions, log to stderr, and exit with code 0

### Requirement 2: NDJSON Wire Protocol

**User Story:** As a developer integrating IntelligenceKit as a Tauri sidecar, I want a simple line-based JSON protocol, so that I can communicate with it from Rust using standard stdin/stdout.

#### Acceptance Criteria

1. EACH message from client to server SHALL be a single JSON object terminated by `\n`
2. EACH response from server to client SHALL be a single JSON object terminated by `\n`
3. THE Server SHALL never write non-JSON content to stdout — all diagnostics and logs go to stderr only
4. WHEN a line contains invalid JSON, THE Server SHALL write `{"ok":false,"error":"invalid_json"}` to stdout and continue reading (not crash or exit)
5. WHEN a line is empty (blank line), THE Server SHALL ignore it and continue reading
6. ALL requests SHALL contain a `command` field (string)
7. THE Server SHALL support the following commands: `"open-session"`, `"message"`, `"close-session"`, `"check-availability"`, `"shutdown"`

### Requirement 3: Session Management

**User Story:** As the Tauri app, I want to open named sessions that persist across multiple messages, so that I can send chunked content and the model retains context from previous messages.

#### Acceptance Criteria

1. WHEN the client sends `open-session`, THE Server SHALL create a new `LanguageModelSession` and return a unique `session_id`
2. THE Server SHALL support multiple concurrent sessions
3. EACH `message` command SHALL include a `session_id` field to identify which session to use
4. IF a `message` references a `session_id` that does not exist, THE Server SHALL return `{"ok":false,"error":"session_not_found"}`
5. WHEN the client sends `close-session` with a `session_id`, THE Server SHALL destroy that session and free its resources
6. THE Server SHALL automatically close sessions that have been idle for more than 120 seconds
7. WHEN a session is auto-closed due to idle timeout, THE Server SHALL log a warning to stderr
8. THE `open-session` command SHALL accept an optional `instructions` field to set the session's system instructions
9. IF `instructions` is not provided, THE Server SHALL use minimal default instructions (the server is task-agnostic — specific instructions come from the client's prompt)

### Requirement 4: Generic Message Execution

**User Story:** As the Tauri app, I want to send any prompt with content to IntelligenceKit and get a structured result back, so that I can implement tagging, summarization, PII redaction, and future tasks without changing the Swift server.

#### Acceptance Criteria

1. EACH `message` command SHALL include: `session_id` (string), `prompt` (string), `content` (string), `output_format` (string)
2. THE `prompt` field SHALL contain the full instructions for what the model should do (e.g., "Generate 3-5 topic tags, each 1-3 words, lowercase")
3. THE `content` field SHALL contain the input text to process
4. THE Server SHALL support the following `output_format` values: `"string_list"` and `"text"`
5. WHEN `output_format` is `"string_list"`, THE Server SHALL use guided generation to return `{"ok":true,"result":["item1","item2",...]}` where result is an array of strings
6. WHEN `output_format` is `"text"`, THE Server SHALL use guided generation to return `{"ok":true,"result":"the text output"}` where result is a string
7. WHEN `output_format` is unrecognized, THE Server SHALL return `{"ok":false,"error":"unknown_output_format"}`
8. THE Server SHALL truncate `content` to 10,000 characters before sending to the model to fit within the 4096-token context window
9. THE Server SHALL construct the model prompt by combining the client's `prompt` field with the `content` field
10. WHEN `prompt` is missing or empty, THE Server SHALL return `{"ok":false,"error":"prompt_required"}`
11. WHEN `content` is missing or empty, THE Server SHALL return `{"ok":false,"error":"content_required"}`
12. THE Server SHALL NOT contain any task-specific logic (no tagging prompts, no summarization logic) — all task intelligence comes from the client's `prompt`

### Requirement 5: Availability Checking

**User Story:** As the Tauri app, I want to check if Apple Intelligence is available on the user's machine before attempting to open sessions, so that I can fall back to an alternative provider.

#### Acceptance Criteria

1. WHEN the `command` is `"check-availability"`, THE Server SHALL check `SystemLanguageModel.default` for availability
2. IF the model is available and ready, THE Server SHALL return `{"ok":true,"available":true}`
3. IF the model is not available, THE Server SHALL return `{"ok":true,"available":false,"reason":"<descriptive reason>"}`
4. THE Server SHALL distinguish between at least these unavailability reasons: Apple Intelligence not enabled by user, macOS version too old, hardware not supported (not Apple Silicon)
5. THE `check-availability` command SHALL NOT require a session — it works outside of any session
6. THE availability check SHALL complete within 2 seconds

### Requirement 6: Error Handling

**User Story:** As a developer, I want IntelligenceKit to return structured error responses without crashing, so that the Tauri app can handle failures gracefully and the server stays alive.

#### Acceptance Criteria

1. WHEN Foundation Models throws a guardrail error (content safety), THE Server SHALL return `{"ok":false,"error":"guardrail_blocked"}`
2. WHEN the on-device model is unavailable during execution, THE Server SHALL return `{"ok":false,"error":"model_unavailable"}`
3. WHEN any unexpected error occurs, THE Server SHALL return `{"ok":false,"error":"<description>"}` with a descriptive message
4. THE Server SHALL never crash from a single bad message — errors are returned as JSON and the server continues reading
5. ALL error responses SHALL include `"ok":false` and an `"error"` string field
6. THE Server SHALL log detailed error information to stderr for debugging, while returning a concise error message in the JSON response
7. WHEN an error occurs within a session, THE session SHALL remain valid — the client can send another message on the same session

### Requirement 7: Signal Handling and Graceful Shutdown

**User Story:** As the Tauri sidecar manager, I want IntelligenceKit to shut down cleanly when terminated, so that no resources are leaked.

#### Acceptance Criteria

1. WHEN SIGTERM is received, THE Server SHALL close all active sessions and exit with code 0
2. WHEN SIGINT is received (Ctrl+C), THE Server SHALL perform the same graceful shutdown as SIGTERM
3. THE Server SHALL release all Foundation Models resources on shutdown
4. WHEN stdin reaches EOF (parent process died), THE Server SHALL perform graceful shutdown and exit with code 0

### Requirement 8: Platform and Build Requirements

**User Story:** As a developer, I want IntelligenceKit to build with Swift 6.2 and target macOS 26+, so that it can use the Foundation Models framework.

#### Acceptance Criteria

1. THE System SHALL require macOS 26.0 or later (Foundation Models minimum requirement)
2. THE System SHALL be built with Swift 6.2 or later
3. THE System SHALL use only Apple-provided frameworks: Foundation, FoundationModels
4. THE System SHALL NOT depend on any external packages or libraries
5. THE System SHALL be compiled for Apple Silicon (arm64) architecture
6. THE System SHALL enable StrictConcurrency (matching JarvisListen convention)
7. THE Swift package SHALL define an executable target named `IntelligenceKit`
8. THE package directory SHALL be named `intelligence-kit` and be a sibling to `jarvis-listen` in the project root

### Requirement 9: Logging and Diagnostics

**User Story:** As a developer debugging IntelligenceKit, I want diagnostic logs written to stderr, so that I can troubleshoot issues without corrupting the JSON stdout protocol.

#### Acceptance Criteria

1. WHEN IntelligenceKit starts, THE Server SHALL log to stderr: "IntelligenceKit server ready"
2. WHEN a session is opened, THE Server SHALL log to stderr: the session_id
3. WHEN a message is processed, THE Server SHALL log to stderr: the session_id, output_format, content length, and processing time
4. WHEN a session is closed (manually or by timeout), THE Server SHALL log to stderr: the session_id and reason
5. WHEN an error occurs, THE Server SHALL log to stderr: the full error details
6. ALL log messages SHALL be prefixed with `[IntelligenceKit]` for easy filtering
7. ALL log messages SHALL go to stderr — stdout is reserved exclusively for JSON responses
8. THE Server SHALL follow the same `logToStderr()`, `logError()`, `logWarning()` pattern as JarvisListen
