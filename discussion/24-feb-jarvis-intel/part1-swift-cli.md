# Part 1: IntelligenceKit — Persistent Server over stdin/stdout

## Goal

Build a persistent Swift server that wraps Apple Foundation Models, communicating
over stdin/stdout using newline-delimited JSON. Manages multiple concurrent sessions,
each backed by a `LanguageModelSession` with multi-turn context.

Must work independently from the terminal before any Tauri integration.

## Naming

**IntelligenceKit** — named after Apple Intelligence, the on-device AI branding.
Follows the same convention as WhisperKit (Apple-specific transcription tool).
The generic role (`IntelProvider` trait) stays provider-agnostic; IntelligenceKit
is specifically the Apple Foundation Models implementation.

## Architecture: Client-Server over stdin/stdout

```
Tauri (Rust Client)                     IntelligenceKit (Swift Server)
    │                                            │
    │── spawn once at app startup ──────────────>│ starts read loop
    │                                            │
    │── {"command":"check-availability"}\n ─────>│
    │<── {"ok":true,"available":true}\n ────────│
    │                                            │
    │── {"command":"open-session"}\n ───────────>│ creates LanguageModelSession
    │<── {"ok":true,"session_id":"a1b2"}\n ─────│
    │                                            │
    │── {"command":"message","session_id":"a1b2",│
    │    "task":"generate-tags",                  │
    │    "content":"<chunk 1>"}\n ──────────────>│ uses session's LM context
    │<── {"ok":true,"tags":[...]}\n ────────────│
    │                                            │
    │── {"command":"message","session_id":"a1b2",│
    │    "task":"generate-tags",                  │
    │    "content":"<chunk 2>"}\n ──────────────>│ same session, has context
    │<── {"ok":true,"tags":[...]}\n ────────────│
    │                                            │
    │── {"command":"close-session",               │
    │    "session_id":"a1b2"}\n ───────────────>│ frees session
    │<── {"ok":true}\n ─────────────────────────│
    │                                            │
    │── {"command":"shutdown"}\n ───────────────>│
    │                                            │ exits
```

**Key differences from original short-lived design:**
- Server stays alive — no per-request process overhead
- Multiple concurrent sessions via `session_id`
- Multi-turn context preserved within a session (chunked tagging, future chat)
- Idle timeout auto-closes inactive sessions (120s default)
- Invalid messages return errors — server never crashes

## Deliverable

A working server you can test interactively:

```bash
cd intelligence-kit && swift build
.build/debug/IntelligenceKit

# Type these lines one at a time (server responds after each):
{"command":"check-availability"}
{"command":"open-session"}
{"command":"message","session_id":"<id>","task":"generate-tags","title":"AI Agents","content":"AI agents are autonomous systems...","source_type":"Article"}
{"command":"close-session","session_id":"<id>"}
{"command":"shutdown"}
```

## Package Structure

```
intelligence-kit/                              # sibling to jarvis-listen/
├── Package.swift                              # Swift 6.2, macOS 26+
├── Sources/IntelligenceKit/
│   ├── main.swift                             # Server: read loop, routing, logging, signals
│   ├── Models.swift                           # Request/Response Codable types
│   ├── SessionManager.swift                   # Session lifecycle + idle timeout monitor
│   ├── Session.swift                          # Individual session + task methods (generateTags)
│   └── Availability.swift                     # SystemLanguageModel availability check
└── Tests/IntelligenceKitTests/
    ├── ModelsTests.swift                      # JSON encode/decode round-trip tests
    └── SessionManagerTests.swift              # Session lifecycle tests
```

## Wire Protocol

### Commands

| Command | Fields | Response |
|---------|--------|----------|
| `open-session` | `instructions` (optional) | `{"ok":true,"session_id":"..."}` |
| `message` | `session_id`, `task`, + task-specific fields | task-specific response |
| `close-session` | `session_id` | `{"ok":true}` |
| `check-availability` | (none) | `{"ok":true,"available":true/false}` |
| `shutdown` | (none) | `{"ok":true}` then exit |

### Tasks (for `message` command)

| Task | Fields | Response |
|------|--------|----------|
| `generate-tags` | `title` (opt), `content` (req), `source_type` (opt) | `{"ok":true,"tags":[...]}` |
| `summarize` (future) | `content` | `{"ok":true,"summary":"..."}` |
| `redact-pii` (future) | `content` | `{"ok":true,"redacted":"..."}` |
| `chat` (future) | `message` | `{"ok":true,"reply":"..."}` |

### Error Responses

All errors: `{"ok":false,"error":"<code>"}`

Codes: `invalid_json`, `unknown_command`, `session_id_required`, `session_not_found`, `task_required`, `unknown_task`, `content_required`, `model_unavailable`, `guardrail_blocked`

**Server never exits on error** — returns JSON error and continues reading.

## Success Criteria

Part 1 is done when:
1. `swift build` compiles without errors
2. Server starts and logs "Server ready" to stderr
3. `check-availability` returns `available: true` on the dev machine
4. Can open a session, send generate-tags, receive 3-5 sensible tags, close session
5. Chunked tag generation works (multiple messages on same session, model has context)
6. Invalid messages return clean error JSON — server stays alive
7. `shutdown` command and SIGTERM both exit cleanly
8. Idle sessions auto-close after 120s
9. All logs go to stderr, all responses go to stdout

## What Part 1 Does NOT Include

- No Tauri integration (that's Part 2)
- No Rust code changes
- No binary copying to `src-tauri/binaries/`
- No summarize/redact-pii/chat tasks (future, but the session architecture supports them)
- No settings UI changes
