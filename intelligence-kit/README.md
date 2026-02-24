# IntelligenceKit

A macOS Swift server that provides a generic gateway to Apple's Foundation Models framework (on-device 3B parameter LLM). It runs as a persistent local server over stdin/stdout using newline-delimited JSON (NDJSON).

## Overview

IntelligenceKit enables the Jarvis Tauri app to leverage on-device AI capabilities without API keys or network requests. The system is **task-agnostic** — it does not contain any domain-specific logic. The client sends a `prompt` (what to do), `content` (the input), and `output_format` (the shape of the result), and the server executes it through Foundation Models with guided generation.

## Requirements

- **macOS**: 26.0 or later
- **Hardware**: Apple Silicon (arm64)
- **Swift**: 6.2+
- **Apple Intelligence**: Must be enabled in System Settings

## Building

```bash
cd intelligence-kit
swift build
```

The executable will be at `.build/debug/IntelligenceKit`.

## Architecture

### Components

- **SessionManager**: Manages LanguageModelSession lifecycle with automatic idle timeout (120s)
- **MessageExecutor**: Executes prompts with guided generation (no task-specific logic)
- **CommandRouter**: Routes NDJSON commands to appropriate handlers
- **AvailabilityChecker**: Checks if Foundation Models is available
- **Signal Handler**: Handles SIGINT/SIGTERM for graceful shutdown

### NDJSON Protocol

Communication uses newline-delimited JSON over stdin/stdout:
- **stdin**: Read commands (one JSON object per line)
- **stdout**: Write responses (one JSON object per line)
- **stderr**: All logging (prefixed with `[IntelligenceKit]`)

## Usage

### Starting the Server

```bash
.build/debug/IntelligenceKit
```

The server logs `[IntelligenceKit] IntelligenceKit server ready` to stderr and waits for commands on stdin.

### Commands

#### 1. Check Availability

Check if Foundation Models is available before creating sessions.

**Request:**
```json
{"command":"check-availability"}
```

**Response (available):**
```json
{"ok":true,"available":true}
```

**Response (unavailable):**
```json
{"ok":true,"available":false,"reason":"Apple Intelligence not enabled by user"}
```

#### 2. Open Session

Create a new LanguageModelSession with optional instructions.

**Request:**
```json
{"command":"open-session","instructions":"You are a helpful assistant."}
```

**Response:**
```json
{"ok":true,"session_id":"550e8400-e29b-41d4-a716-446655440000"}
```

#### 3. Send Message

Execute a prompt against an existing session.

**Request (string_list format):**
```json
{
  "command":"message",
  "session_id":"550e8400-e29b-41d4-a716-446655440000",
  "prompt":"Generate 3-5 topic tags for this content",
  "content":"Article about machine learning and neural networks...",
  "output_format":"string_list"
}
```

**Response:**
```json
{"ok":true,"result":["machine learning","neural networks","AI","deep learning"]}
```

**Request (text format):**
```json
{
  "command":"message",
  "session_id":"550e8400-e29b-41d4-a716-446655440000",
  "prompt":"Summarize this content in one sentence",
  "content":"Long article text...",
  "output_format":"text"
}
```

**Response:**
```json
{"ok":true,"result":"This article discusses the fundamentals of machine learning."}
```

#### 4. Close Session

Explicitly close a session (optional - sessions auto-close after 120s idle).

**Request:**
```json
{"command":"close-session","session_id":"550e8400-e29b-41d4-a716-446655440000"}
```

**Response:**
```json
{"ok":true}
```

#### 5. Shutdown

Gracefully shut down the server.

**Request:**
```json
{"command":"shutdown"}
```

**Response:**
```json
{"ok":true}
```

The server closes all sessions and exits after sending the response.

### Error Responses

All errors follow the format:
```json
{"ok":false,"error":"<error_code>"}
```

Common error codes:
- `invalid_json`: Request could not be parsed
- `unknown_command`: Command not recognized
- `session_not_found`: Session ID doesn't exist
- `session_id_required`: Missing session_id field
- `prompt_required`: Missing or empty prompt
- `content_required`: Missing or empty content
- `output_format_required`: Missing output_format field
- `unknown_output_format`: Format not "string_list" or "text"
- `guardrail_blocked`: Content safety filter blocked the request
- `model_unavailable`: Foundation Models became unavailable

## Example Session

### Connected Session (Multiple Commands)

To send multiple commands to the same server instance, use a single pipe:

```bash
# Send multiple commands to one server instance
printf '{"command":"check-availability"}\n{"command":"open-session"}\n{"command":"message","session_id":"SESSION_ID_FROM_RESPONSE","prompt":"List 3 colors","content":"test","output_format":"string_list"}\n{"command":"shutdown"}\n' | .build/debug/IntelligenceKit
```

Or use a heredoc for better readability:

```bash
.build/debug/IntelligenceKit <<EOF
{"command":"check-availability"}
{"command":"open-session"}
{"command":"shutdown"}
EOF
```

### Individual Command Examples

These examples show individual commands (each starts a new server process):

```bash
# Check availability
echo '{"command":"check-availability"}' | .build/debug/IntelligenceKit
# Output: {"ok":true,"available":true}

# Open a session (note: session won't persist to next command)
echo '{"command":"open-session"}' | .build/debug/IntelligenceKit
# Output: {"ok":true,"session_id":"550e8400-e29b-41d4-a716-446655440000"}
```

**Note**: Each `echo | .build/debug/IntelligenceKit` invocation starts a new server process, so session IDs don't persist between commands. For real usage, keep the server running and send commands via a persistent stdin connection (as shown in the Tauri integration example).

## Features

### Guided Generation

IntelligenceKit uses Foundation Models' `@Generable` types to constrain output:
- **string_list**: Model produces an array of strings
- **text**: Model produces a single string

This ensures structured output without parsing or post-processing.

### Session Management

- **Unique IDs**: Each session gets a UUID
- **Concurrent Sessions**: Multiple sessions can run simultaneously
- **Idle Timeout**: Sessions auto-close after 120 seconds of inactivity
- **Activity Tracking**: Each message resets the idle timer

### Content Truncation

Content is automatically truncated to 10,000 characters to fit within the 4096-token context window.

### Error Resilience

- Invalid JSON returns an error and continues processing
- Empty lines are skipped silently
- Errors within a session don't invalidate the session
- Server never crashes - all errors are returned as JSON responses

## Signal Handling

The server handles SIGINT and SIGTERM gracefully:
- Sets a shutdown flag (checked after each request)
- Closes all active sessions
- Exits cleanly

**Trade-off**: Since `readLine()` blocks, the signal flag won't be checked until the next line arrives. In normal operation, Tauri sends a `shutdown` command before SIGTERM, allowing graceful cleanup.

## Logging

All logs go to stderr with automatic `[IntelligenceKit]` prefix:
- Session lifecycle events (open, close, timeout)
- Message execution timing and content length
- Errors and warnings
- Shutdown notifications

Example stderr output:
```
[IntelligenceKit] IntelligenceKit server ready
[IntelligenceKit] Session opened: 550e8400-e29b-41d4-a716-446655440000
[IntelligenceKit] Message processed: session=550e8400-e29b-41d4-a716-446655440000, format=string_list, content_length=1234, time=0.45s
[IntelligenceKit] Session closed: 550e8400-e29b-41d4-a716-446655440000
[IntelligenceKit] Shutdown complete
```

## Testing

Run unit tests:
```bash
swift test
```

Current test coverage:
- Models: JSON encoding/decoding, CodingKeys mapping
- SessionManager: Session lifecycle, idle timeout, concurrent sessions
- MessageExecutor: Content truncation, prompt construction, format validation
- AvailabilityChecker: Hardware/OS/model availability checks
- CommandRouter: Command routing, field validation, error handling

## Test UI

A browser-based utility for interactively testing IntelligenceKit without typing raw JSON in the terminal. Located in `test-ui/`.

### Prerequisites

- Node.js (v18+)
- IntelligenceKit binary built (`swift build`)

### Running

```bash
cd test-ui
node server.js
```

Open **http://localhost:3847** in your browser.

### Workflow

1. Click **Start** to launch the IntelligenceKit process
2. Click **Check Availability** to verify Foundation Models is ready
3. Click **Open Session** — the session ID appears automatically
4. Fill in **prompt**, **content**, and **output_format**, then click **Send Message** (or press `Cmd+Enter`)
5. View responses in the **Response Log** panel and server output in the **Server Logs** panel
6. Use the **Raw JSON** panel to send arbitrary NDJSON commands
7. Click **Stop** when done

### Architecture

The test UI consists of two files:

- **`server.js`** — Node.js bridge that spawns IntelligenceKit as a child process, pipes HTTP requests to its stdin, and returns stdout responses. Runs on port 3847.
- **`index.html`** — Single-page browser UI with quick-action buttons, a message form, raw JSON input, and color-coded response/stderr log panels.

## Integration with Tauri

IntelligenceKit is designed to run as a Tauri sidecar:

```rust
use tauri::api::process::{Command, CommandEvent};

// Start the server
let (mut rx, _child) = Command::new_sidecar("IntelligenceKit")?
    .spawn()?;

// Send commands via stdin
child.write(b"{\"command\":\"check-availability\"}\n")?;

// Read responses from stdout
while let Some(event) = rx.recv().await {
    match event {
        CommandEvent::Stdout(line) => {
            let response: Response = serde_json::from_str(&line)?;
            // Handle response
        }
        CommandEvent::Stderr(line) => {
            // Log to console
        }
        _ => {}
    }
}
```

## License

Part of the Jarvis AWS project for the AWS 10,000 AIdeas Competition.
