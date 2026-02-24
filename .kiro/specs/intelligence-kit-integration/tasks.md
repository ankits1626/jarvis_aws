# Implementation Plan: IntelligenceKit Integration

## Overview

This plan integrates IntelligenceKit — a macOS Swift server wrapping Apple's on-device Foundation Models — into the Jarvis Tauri app as a sidecar process. The implementation follows a trait-based architecture with IntelligenceKitProvider as the default backend. Tasks are organized to build incrementally: core provider abstraction → sidecar lifecycle → NDJSON client → enrichment pipeline → database schema → frontend integration.

## Tasks

- [x] 1. Set up Intelligence module structure and dependencies
  - Create `src-tauri/src/intelligence/` directory with `mod.rs`, `provider.rs`, `intelligencekit_provider.rs`
  - Add dependencies to `Cargo.toml`: `async-trait`, `tokio` (with process, io-util features), `serde`, `serde_json`, `chrono`
  - Export intelligence module in `src-tauri/src/lib.rs`
  - _Requirements: 1.1, 1.5_

- [ ] 2. Implement IntelProvider trait and core types
  - [x] 2.1 Define `AvailabilityResult` struct in `provider.rs`
    - Add fields: available (bool), reason (Option<String>)
    - Add Serialize/Deserialize derives
    - _Requirements: 1.2_
  
  - [x] 2.2 Define `IntelProvider` trait in `provider.rs`
    - Add async_trait annotation
    - Define methods: check_availability, generate_tags, summarize with proper signatures
    - Ensure trait is Send + Sync for Tauri state management
    - Document tag count constraint (1-10 accepted, trimmed to 5)
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [ ] 3. Implement IntelligenceKitProvider sidecar lifecycle
  - [x] 3.1 Define NDJSON protocol structs in `intelligencekit_provider.rs`
    - Create `NdjsonCommand` struct with command, session_id, instructions, prompt, content, output_format fields
    - Create `NdjsonResponse` struct with ok, session_id, result, error, available, reason fields
    - Add Serialize/Deserialize derives with proper #[serde] attributes
    - _Requirements: 3.1, 3.2_
  
  - [x] 3.2 Create IntelligenceKitProvider struct with ProviderState
    - Define `ProviderState` with child, session_id, availability, stdin, stdout fields
    - Wrap state in Arc<Mutex<ProviderState>>
    - _Requirements: 2.2, 2.3, 3.4_
  
  - [x] 3.3 Implement IntelligenceKitProvider::new() method
    - Resolve binary path using Tauri v2 PathResolver: `app_handle.path().resolve("binaries/IntelligenceKit", BaseDirectory::Resource)`
    - Spawn using tokio::process::Command with piped stdin/stdout/stderr
    - Take ownership of stdio handles and wrap in BufWriter/BufReader
    - Spawn stderr monitoring task that prefixes output with `[IntelligenceKit]`
    - Call check_availability_internal() and cache result
    - Open initial session if available
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.7_
  
  - [x] 3.4 Implement IntelligenceKitProvider::shutdown() method
    - Send shutdown command via send_command()
    - Wait up to 3 seconds for graceful exit
    - Send SIGTERM if timeout
    - _Requirements: 2.6_
  
  - [ ]* 3.5 Write unit test for binary not found graceful degradation
    - Test that new() returns error when binary missing
    - Test that app can continue without IntelligenceKit
    - _Requirements: 2.8_

- [ ] 4. Implement NDJSON client communication
  - [x] 4.1 Implement send_command() method
    - Acquire mutex lock on state
    - Serialize command to JSON + newline
    - Write to stdin with 30s timeout
    - Read one line from stdout with 30s timeout
    - Deserialize response
    - Return Result with descriptive errors
    - _Requirements: 3.1, 3.2, 3.3, 3.5, 3.6_
  
  - [x] 4.2 Implement check_availability_internal() method
    - Send check-availability command
    - Parse response and return AvailabilityResult
    - Handle error responses gracefully
    - _Requirements: 2.7_
  
  - [ ]* 4.3 Write property test for NDJSON serialization round trip
    - **Property 2: NDJSON Serialization Round Trip**
    - **Validates: Requirements 3.1, 3.2**
  
  - [ ]* 4.4 Write unit tests for response parsing
    - Test parsing {"ok":true,"available":true}
    - Test parsing {"ok":false,"error":"..."}
    - Test parsing session_id extraction
    - Test parsing result field (string_list and text formats)
    - _Requirements: 3.3_
  
  - [ ]* 4.5 Write unit test for command timeout
    - Test that commands timing out after 30s return error
    - _Requirements: 3.5_
  
  - [ ]* 4.6 Write property test for error handling never panics
    - **Property 4: Error Handling Never Panics**
    - **Validates: Requirements 3.6**

- [ ] 5. Implement session management
  - [x] 5.1 Implement open_session() method
    - Send open-session command with generic instructions
    - Extract session_id from response
    - Store session_id in state
    - Return session_id or error
    - _Requirements: 4.1, 4.2_
  
  - [x] 5.2 Implement ensure_session() method
    - Check if session_id exists in state
    - If not, call open_session()
    - Return session_id
    - _Requirements: 4.1_
  
  - [ ]* 5.3 Write unit test for session recovery after timeout
    - Test that after session timeout, next request re-opens session
    - _Requirements: 4.3_

- [ ] 6. Implement IntelProvider trait for IntelligenceKitProvider
  - [x] 6.1 Implement check_availability() method
    - Return cached availability from state
    - _Requirements: 1.2_
  
  - [x] 6.2 Implement generate_tags_internal() helper method (in separate impl block)
    - Call ensure_session()
    - Send message command with tag generation prompt
    - Parse response as Vec<String>
    - Validate non-empty array (return error if empty)
    - Trim to max 5 tags if more returned
    - Return Result<Vec<String>, String>
    - _Requirements: 1.3, 4.1_
  
  - [x] 6.3 Implement generate_tags() trait method
    - Call generate_tags_internal()
    - Detect "session_not_found" error
    - Retry once after re-opening session
    - _Requirements: 1.3, 4.4_
  
  - [x] 6.4 Implement summarize_internal() helper method (in separate impl block)
    - Call ensure_session()
    - Send message command with summarization prompt
    - Parse response as String
    - Validate non-empty string (return error if empty)
    - Return Result<String, String>
    - _Requirements: 1.4, 4.1_
  
  - [x] 6.5 Implement summarize() trait method
    - Call summarize_internal()
    - Detect "session_not_found" error
    - Retry once after re-opening session
    - _Requirements: 1.4, 4.4_
  
  - [ ]* 6.6 Write property test for tag count constraint
    - **Property 1: Tag Count Constraint**
    - Test that generate_tags returns 1-5 tags (accepts 1-10, trims to 5)
    - **Validates: Requirements 1.3**
  
  - [ ]* 6.7 Write property test for concurrent request serialization
    - **Property 3: Concurrent Request Serialization**
    - **Validates: Requirements 3.4**
  
  - [ ]* 6.8 Write unit test for session_not_found retry
    - Test that session_not_found error triggers re-open and retry
    - _Requirements: 4.4_

- [ ] 7. Checkpoint - Ensure provider tests pass
  - Run all intelligence module tests
  - Verify sidecar spawn, NDJSON communication, session management work correctly
  - Ask user if questions arise

- [x] 8. Extend Gem schema with ai_enrichment
  - [x] 8.1 Add ai_enrichment field to Gem struct in `src-tauri/src/gems/store.rs`
    - Add field: `pub ai_enrichment: Option<serde_json::Value>`
    - Add documentation comment explaining JSON structure
    - _Requirements: 6.1, 6.2_
  
  - [x] 8.2 Add tags and summary fields to GemPreview struct
    - Add fields: `pub tags: Option<Vec<String>>`, `pub summary: Option<String>`
    - Add documentation comments
    - _Requirements: 6.3_
  
  - [x] 8.3 Add filter_by_tag method to GemStore trait
    - Define signature: `async fn filter_by_tag(&self, tag: &str, limit: usize, offset: usize) -> Result<Vec<GemPreview>, String>`
    - _Requirements: 6.8_

- [x] 9. Implement database schema migration
  - [x] 9.1 Add ai_enrichment column to gems table
    - Execute `ALTER TABLE gems ADD COLUMN ai_enrichment TEXT`
    - _Requirements: 6.4_
  
  - [x] 9.2 Update FTS5 insert trigger to include summary
    - Drop and recreate gems_ai trigger
    - Concatenate content and summary: `COALESCE(new.content, '') || ' ' || COALESCE(json_extract(new.ai_enrichment, '$.summary'), '')`
    - _Requirements: 6.6_
  
  - [x] 9.3 Update FTS5 update trigger to include summary
    - Drop and recreate gems_au trigger
    - Use same concatenation formula for both delete and insert operations
    - _Requirements: 6.6_
  
  - [x] 9.4 Update FTS5 delete trigger to include summary
    - Drop and recreate gems_ad trigger
    - Use same concatenation formula: `COALESCE(old.content, '') || ' ' || COALESCE(json_extract(old.ai_enrichment, '$.summary'), '')`
    - _Requirements: 6.6_
  
  - [ ]* 9.5 Write unit test for schema migration
    - Test that ai_enrichment column exists after migration
    - Test that existing gems have NULL ai_enrichment
    - _Requirements: 6.4, 6.5_
  
  - [x] 9.6 Update SqliteGemStore save() and row_to_gem() for ai_enrichment
    - Update save() INSERT/REPLACE SQL to include ai_enrichment column
    - Add ai_enrichment parameter binding (serialize Option<Value> to Option<String>)
    - Update row_to_gem() to read ai_enrichment from database row
    - Deserialize ai_enrichment from TEXT to Option<serde_json::Value>
    - Handle NULL values gracefully (return None)
    - _Requirements: 6.1, 6.4_

- [x] 10. Implement SqliteGemStore enrichment methods
  - [x] 10.1 Implement filter_by_tag() method
    - Use SQLite json_each to expand tags array
    - Filter by exact tag match: `WHERE json_each.value = ?1`
    - Return DISTINCT gems ordered by captured_at DESC
    - Support pagination with LIMIT and OFFSET
    - _Requirements: 6.8_
  
  - [x] 10.2 Update gem_to_preview() helper to extract ai_enrichment fields
    - Extract tags from ai_enrichment.tags using serde_json::from_value
    - Extract summary from ai_enrichment.summary using .as_str()
    - Handle NULL ai_enrichment gracefully (return None for both)
    - Handle JSON parsing errors gracefully (return None)
    - Merge with existing gem_to_preview() logic (don't overwrite)
    - _Requirements: 6.7_
  
  - [ ]* 10.3 Write property test for GemPreview extraction
    - **Property 7: GemPreview Extraction**
    - **Validates: Requirements 6.7**
  
  - [ ]* 10.4 Write property test for tag filtering accuracy
    - **Property 8: Tag Filtering Accuracy**
    - **Validates: Requirements 6.8**
  
  - [ ]* 10.5 Write property test for summary searchability
    - **Property 6: Summary Searchability**
    - **Validates: Requirements 6.6**
  
  - [ ]* 10.6 Write property test for list command backwards compatibility
    - **Property 9: List Command Backwards Compatibility**
    - **Validates: Requirements 11.2**
    - Test that list_gems works with mixed enriched/unenriched gems
  
  - [ ]* 10.7 Write property test for search command backwards compatibility
    - **Property 10: Search Command Backwards Compatibility**
    - **Validates: Requirements 11.2**
    - Test that search_gems works with mixed enriched/unenriched gems
  
  - [ ]* 10.8 Write property test for delete command backwards compatibility
    - **Property 11: Delete Command Backwards Compatibility**
    - **Validates: Requirements 11.4**
    - Test that delete_gem works on both enriched and unenriched gems

- [ ] 11. Implement enrichment Tauri commands
  - [x] 11.1 Add enrich_content() helper function in `src-tauri/src/commands.rs`
    - Accept IntelProvider and content string
    - Call generate_tags() and summarize()
    - Build ai_enrichment JSON with tags, summary, provider, enriched_at
    - Return Result<serde_json::Value, String>
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.7_
  
  - [x] 11.2 Update save_gem() command to add enrichment
    - Add intel_provider parameter: `tauri::State<'_, Arc<dyn IntelProvider>>`
    - After page_gist_to_gem conversion, check availability
    - If available and content non-empty, call enrich_content()
    - Set gem.ai_enrichment on success, log error and continue on failure
    - Save gem (with or without enrichment)
    - _Requirements: 5.1, 5.2, 5.5, 5.6, 5.8_
  
  - [x] 11.3 Implement enrich_gem() command
    - Accept gem id parameter
    - Check availability first, return error if unavailable
    - Fetch gem by ID, return error if not found
    - Get content for enrichment (content or description)
    - Call enrich_content()
    - Set gem.ai_enrichment and save
    - Return updated gem
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_
  
  - [x] 11.4 Implement check_intel_availability() command
    - Accept intel_provider parameter
    - Call check_availability() and return result
    - _Requirements: 8.1, 8.2, 8.3, 8.4_
  
  - [x] 11.5 Implement filter_gems_by_tag() command
    - Accept tag, limit (optional), offset (optional) parameters
    - Call gem_store.filter_by_tag()
    - Return Vec<GemPreview>
    - _Requirements: 6.8_
  
  - [ ]* 11.6 Write unit test for save_gem enrichment flow
    - Test that save_gem calls generate_tags and summarize
    - Test that enrichment uses description when content is empty
    - Test that enrichment structure is correct
    - _Requirements: 5.1, 5.2, 5.7_
  
  - [ ]* 11.7 Write property test for save succeeds without enrichment
    - **Property 5: Save Succeeds Without Enrichment**
    - **Validates: Requirements 5.5, 11.1**
  
  - [ ]* 11.8 Write unit test for enrich_gem error when unavailable
    - Test that enrich_gem returns error when provider unavailable
    - _Requirements: 7.4_

- [x] 12. Integrate IntelProvider into Tauri application
  - [x] 12.1 Initialize IntelligenceKitProvider in lib.rs setup() function
    - Create IntelligenceKitProvider instance with error handling
    - Wrap in Arc<dyn IntelProvider> for trait object
    - Register with app.manage()
    - Log availability status on startup
    - Continue app startup even if provider unavailable
    - _Requirements: 1.6, 2.1, 2.8_
  
  - [x] 12.2 Register enrichment commands in invoke_handler
    - Add enrich_gem, check_intel_availability, filter_gems_by_tag to handler
    - _Requirements: 1.6_
  
  - [x] 12.3 Register IntelligenceKit binary in tauri.conf.json
    - Add "binaries/IntelligenceKit" to externalBin array
    - _Requirements: 2.1_
  
  - [ ]* 12.4 Add shutdown hook for IntelligenceKitProvider
    - Call provider.shutdown() in app cleanup
    - Note: Tauri doesn't provide app-level cleanup hooks; process cleanup happens automatically on exit
    - _Requirements: 2.6_

- [ ] 13. Checkpoint - Ensure backend integration tests pass
  - Run all tests including enrichment pipeline
  - Verify graceful degradation when provider unavailable
  - Test backwards compatibility with existing commands
  - Run existing feature tests to verify Property 12 (Existing Features Unaffected)
  - Ask user if questions arise

- [x] 14. Update frontend TypeScript types
  - [x] 14.1 Add ai_enrichment field to Gem interface in `src/state/types.ts`
    - Add field: `ai_enrichment: { tags: string[], summary: string, provider: string, enriched_at: string } | null`
    - _Requirements: 6.1_
  
  - [x] 14.2 Add tags and summary fields to GemPreview interface
    - Add fields: `tags: string[] | null`, `summary: string | null`
    - _Requirements: 6.3_
  
  - [x] 14.3 Add AvailabilityResult interface
    - Add fields: `available: boolean`, `reason?: string`
    - _Requirements: 8.2_

- [x] 15. Enhance GemsPanel with AI enrichment display
  - [x] 15.1 Add AI status indicator to GemsPanel header
    - Call check_intel_availability on mount and cache result
    - Display "AI" badge (green when available, gray when unavailable)
    - Add tooltip explaining status on hover
    - _Requirements: 10.1, 10.2, 10.3_
  
  - [x] 15.2 Add tags display to gem cards
    - Display tags as clickable badges below title
    - Only show tags section if tags are present
    - Implement filterByTag handler that calls filter_gems_by_tag command
    - _Requirements: 9.1, 9.3, 9.6_
  
  - [x] 15.3 Add summary display to gem cards
    - Display summary below tags (if present)
    - Style differently from content preview
    - Fall back to content_preview if summary not present
    - _Requirements: 9.2, 9.4_
  
  - [x] 15.4 Add Enrich button to gem cards
    - Show "Enrich" button (sparkle icon) when AI available and gem has no ai_enrichment
    - Show "Re-enrich" button (refresh icon) when AI available and gem has ai_enrichment
    - Implement enrichGem handler that calls enrich_gem command
    - Show loading spinner during enrichment
    - Update card in-place with new tags and summary on success
    - Show error toast on failure
    - _Requirements: 9.7, 9.8, 9.9, 9.10, 9.11_

- [x] 16. Enhance GistCard with AI availability indicator
  - [x] 16.1 Add AI enrichment indicator to GistCard
    - Call check_intel_availability on mount
    - Show "AI enrichment will be added on save" indicator when available
    - Use sparkle icon for visual consistency
    - _Requirements: 9.5_

- [x] 17. Final checkpoint - Ensure all tests pass
  - Run all backend tests (unit, property, integration)
  - Run all frontend tests
  - Verify backwards compatibility with existing features
  - Test graceful degradation scenarios
  - Ask user if questions arise

- [ ] 18. Manual testing and validation
  - [ ] 18.1 Test with IntelligenceKit binary present
    - Start app, verify availability=true
    - Save gem, verify tags and summary appear
    - Click Enrich button, verify enrichment works
    - Search for words from summary, verify gem appears
    - Filter by tag, verify only matching gems appear
    - _Requirements: All_
  
  - [ ] 18.2 Test with IntelligenceKit binary missing
    - Start app, verify availability=false and app starts successfully
    - Save gem, verify gem saves without enrichment
    - Verify UI shows "AI unavailable" indicators
    - _Requirements: 2.8, 5.5, 11.8_
  
  - [ ] 18.3 Test backwards compatibility
    - Verify list_gems works with mixed enriched/unenriched gems
    - Verify search_gems works with mixed enriched/unenriched gems
    - Verify delete_gem works on both enriched and unenriched gems
    - Verify JarvisListen and IntelligenceKit can run simultaneously
    - _Requirements: 11.1, 11.2, 11.4, 11.5_

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Property tests use proptest with minimum 100 iterations per test
- Each property test references its design document property number
- Unit tests focus on specific examples and edge cases
- IntelligenceKit uses `tokio::process::Command` directly (not `tauri_plugin_shell`) for synchronous request-response
- The `*_internal` helper methods must be in a separate `impl IntelligenceKitProvider` block (inherent methods)
- Trait methods (`check_availability`, `generate_tags`, `summarize`) go in `impl IntelProvider for IntelligenceKitProvider` block
- Tag count: requests 3-5 in prompt, accepts 1-10, trims to max 5, errors if 0
- All existing functionality (recording, transcription, browser observation, gems) remains unchanged
- IntelProvider trait allows future implementations (ClaudeProvider, KeywordProvider) without changing commands
- System degrades gracefully when Apple Intelligence unavailable (older Mac, Intel hardware, user disabled)

