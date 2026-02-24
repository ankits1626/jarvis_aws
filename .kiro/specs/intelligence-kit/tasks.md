# Implementation Tasks: IntelligenceKit

## Phase 1: Project Setup and Core Infrastructure

- [x] 1. Create Swift package structure
  - [x] 1.1 Create `intelligence-kit/` directory as sibling to `jarvis-listen/`
  - [x] 1.2 Create `Package.swift` with Swift 6.2, macOS 26+ platform, StrictConcurrency enabled
  - [x] 1.3 Create `Sources/IntelligenceKit/` directory
  - [x] 1.4 Create `Tests/IntelligenceKitTests/` directory
  - [x] 1.5 Verify package builds with `swift build`

- [x] 2. Implement logging utilities in `Logging.swift`
  - [x] 2.1 Create `Logging.swift` file
  - [x] 2.2 Create logging functions: `logToStderr()`, `logError()`, `logWarning()`
  - [x] 2.3 Auto-prefix all log output with `[IntelligenceKit]` inside the logging functions
  - [x] 2.4 Ensure all logs go to `FileHandle.standardError`

## Phase 2: Data Models and Protocol Types

- [x] 3. Implement request/response models in `Models.swift`
  - [x] 3.1 Define `Command` enum with all command types
  - [x] 3.2 Define `Request` struct with all fields and CodingKeys for snake_case mapping
  - [x] 3.3 Define `Response` enum with custom Codable implementation
  - [x] 3.4 Define `SuccessResponse` struct with CodingKeys
  - [x] 3.5 Define `ErrorResponse` struct
  - [x] 3.6 Define `AvailabilityResponse` struct
  - [x] 3.7 Define `ResultValue` enum with custom Codable for flat JSON
  - [x] 3.8 Define `@Generable` output format types: `StringListOutput`, `TextOutput`

- [x] 4. Write unit tests for Models
  - [x] 4.1 Test Request JSON decoding with snake_case fields
  - [x] 4.2 Test Response JSON encoding (success, error, availability)
  - [x] 4.3 Test ResultValue encoding (string_list, text)
  - [x] 4.4 Test CodingKeys mapping (sessionId ↔ session_id, outputFormat ↔ output_format)

## Phase 3: Session Management

- [x] 5. Implement SessionManager actor in `SessionManager.swift`
  - [x] 5.1 Define `SessionState` struct with session and lastActivity
  - [x] 5.2 Implement `openSession()` with UUID generation and LanguageModelSession initialization
  - [ ] 5.2b Accept optional `idleTimeout` parameter in SessionManager init (default 120.0)
  - [x] 5.3 Implement `getSession()` with lastActivity update
  - [x] 5.4 Implement `closeSession()` with resource cleanup
  - [x] 5.5 Implement `closeAllSessions()` for shutdown
  - [x] 5.6 Implement idle monitoring background task (30-second interval)
  - [x] 5.7 Implement `checkIdleSessions()` with collect-then-close pattern

- [x] 6. Write unit tests for SessionManager
  - [x] 6.1 Test session creation returns unique IDs
  - [x] 6.2 Test concurrent session independence
  - [x] 6.3 Test session retrieval and lastActivity update
  - [x] 6.4 Test session closure
  - [x] 6.5 Test idle timeout (with configurable timeout for testing)
  - [x] 6.6 Test closeAllSessions

## Phase 4: Message Execution

- [x] 7. Implement MessageExecutor in `MessageExecutor.swift`
  - [x] 7.1 Define `ExecutionError` enum
  - [x] 7.2 Implement content truncation (10,000 characters)
  - [x] 7.3 Implement prompt construction (prompt + content)
  - [x] 7.4 Implement guided generation for string_list output format
  - [x] 7.5 Implement guided generation for text output format
  - [x] 7.6 Implement error handling (guardrails, model unavailable, unknown format)

- [x] 8. Write unit tests for MessageExecutor
  - [x] 8.1 Test content truncation at 10,000 characters
  - [x] 8.2 Test prompt construction includes both prompt and content
  - [x] 8.3 Test string_list output format produces array
  - [x] 8.4 Test text output format produces string
  - [x] 8.5 Test unknown output format throws error
  - [x] 8.6 Test guardrail error handling
  - [x] 8.7 Test model unavailable error handling

## Phase 5: Availability Checking

- [x] 9. Implement AvailabilityChecker in `Availability.swift`
  - [x] 9.1 Implement hardware check (#if !arch(arm64))
  - [x] 9.2 Implement macOS version check (ProcessInfo.operatingSystemVersion)
  - [x] 9.3 Implement model availability check (SystemLanguageModel.default.availability)
  - [x] 9.4 Return structured availability status with reason

- [x] 10. Write unit tests for AvailabilityChecker
  - [x] 10.1 Test availability check returns correct structure
  - [x] 10.2 Test unavailability reasons (mock different scenarios)
  - [x] 10.3 Test synchronous execution (no timeout needed)

## Phase 6: Command Routing

- [x] 11. Implement CommandRouter in `CommandRouter.swift`
  - [x] 11.1 Create `CommandRouter.swift` file
  - [x] 11.2 Implement `route()` function with command dispatch
  - [x] 11.3 Implement `handleOpenSession()` with field validation
  - [x] 11.4 Implement `handleMessage()` with all field validations and timing
  - [x] 11.5 Implement `handleCloseSession()` with field validation
  - [x] 11.6 Implement `handleCheckAvailability()` (synchronous, no await)
  - [x] 11.7 Implement `handleShutdown()` with session cleanup

- [x] 12. Write unit tests for CommandRouter
  - [x] 12.1 Test unknown command returns error
  - [x] 12.2 Test open-session success and failure cases
  - [x] 12.3 Test message with all field validations
  - [x] 12.4 Test close-session success and session_not_found
  - [x] 12.5 Test check-availability response structure
  - [x] 12.6 Test shutdown closes all sessions

## Phase 7: Signal Handling and Main Loop

- [x] 13. Implement signal handling in `main.swift`
  - [x] 13.1 Implement `SignalHandler` class with DispatchSource.makeSignalSource
  - [x] 13.2 Implement thread-safe shutdown flag with os_unfair_lock
  - [x] 13.3 Set up SIGINT, SIGTERM, and SIGPIPE handlers
  - [x] 13.4 Implement `setupSignalHandlers()` function
  - [x] 13.5 Add signal handler flag checking in read loop

- [x] 14. Implement main server loop in `main.swift`
  - [x] 14.1 Implement `writeResponse()` function for NDJSON output
  - [x] 14.2 Implement `runServer()` async function with read loop
  - [x] 14.3 Handle invalid JSON with error response
  - [x] 14.4 Handle empty lines (skip and continue)
  - [x] 14.5 Implement shutdown flag checking after each response
  - [x] 14.6 Implement graceful shutdown with session cleanup
  - [x] 14.7 Create top-level code entry point (no @main)
  - [x] 14.8 Set up async context with Task { } and RunLoop.main.run()

- [ ] 15. Write unit tests for signal handling and main loop
  - [ ] 15.1 Test signal handler sets shutdown flag
  - [ ] 15.2 Test invalid JSON returns error and continues
  - [ ] 15.3 Test empty lines are ignored
  - [ ] 15.4 Test shutdown command sets flag and exits cleanly
  - [ ] 15.5 Test EOF triggers graceful shutdown

## Phase 8: Property-Based Testing

- [ ] 16. Implement property test infrastructure
  - [ ] 16.1 Create test helpers for random request generation
  - [ ] 16.2 Create TestServer wrapper for spawning server process
  - [ ] 16.3 Implement NDJSON communication helpers

- [ ] 17. Write property tests (100 iterations each)
  - [ ] 17.1 Property 1: One response per request
  - [ ] 17.2 Property 2: Sequential message processing
  - [ ] 17.3 Property 3: NDJSON response format
  - [ ] 17.4 Property 4: Invalid JSON handling
  - [ ] 17.5 Property 5: Empty line handling
  - [ ] 17.6 Property 6: Required command field
  - [ ] 17.7 Property 7: Unique session IDs
  - [ ] 17.8 Property 8: Concurrent session independence
  - [ ] 17.9 Property 9: Session not found error
  - [ ] 17.10 Property 10: Session closure
  - [ ] 17.11 Property 11: Idle timeout (time-dependent)
  - [ ] 17.12 Property 12: Optional instructions parameter
  - [ ] 17.13 Property 13: Required message fields
  - [ ] 17.14 Property 14: Output format structure
  - [ ] 17.15 Property 15: Unknown output format error
  - [ ] 17.16 Property 16: Content truncation
  - [ ] 17.17 Property 17: Prompt construction
  - [ ] 17.18 Property 18: Error response structure
  - [ ] 17.19 Property 19: Session resilience after error
  - [ ] 17.20 Property 20: Error logging
  - [ ] 17.21 Property 21: Server continuity
  - [ ] 17.22 Property 22: Session lifecycle round-trip
  - [ ] 17.23 Property 23: Empty prompt rejection
  - [ ] 17.24 Property 24: Empty content rejection
  - [ ] 17.25 Property 25: Maximum content length boundary

## Phase 9: Integration Testing

- [ ] 18. Write end-to-end integration tests
  - [ ] 18.1 Test complete session lifecycle (open → message → close)
  - [ ] 18.2 Test multiple concurrent sessions
  - [ ] 18.3 Test availability check before session creation
  - [ ] 18.4 Test graceful shutdown with active sessions
  - [ ] 18.5 Test error recovery (send error-triggering message, then valid message)
  - [ ] 18.6 Test idle timeout with real timing
  - [ ] 18.7 Test all command types in sequence

## Phase 10: Documentation and Polish

- [x] 19. Add code documentation
  - [x] 19.1 Add doc comments to all public types and functions
  - [x] 19.2 Document trade-offs (signal handling, readLine blocking)
  - [x] 19.3 Add usage examples in README

- [-] 20. Final verification
  - [x] 20.1 Run all unit tests and verify >80% line coverage
  - [ ] 20.2 Run all property tests (100 iterations)
  - [ ] 20.3 Run integration tests on macOS 26+ with Apple Intelligence enabled
  - [x] 20.4 Verify binary builds and runs as expected
  - [ ] 20.5 Test with Tauri sidecar integration (manual testing)
