# Implementation Plan: JARVIS Gems

## Overview

This plan implements a persistent knowledge base for browser extractions. The implementation follows a trait-based architecture with SQLite + FTS5 as the default storage backend. Tasks are organized to build incrementally: core storage abstraction → SQLite implementation → Tauri commands → frontend integration.

## Tasks

- [x] 1. Set up Gems module structure and dependencies
  - Create `src/gems/` directory with `mod.rs`, `store.rs`, `sqlite_store.rs`
  - Add dependencies to `Cargo.toml`: `async-trait`, `uuid`, `rusqlite` (with bundled feature), `dirs`, `chrono`
  - Export gems module in `src/lib.rs`
  - _Requirements: 1.1, 1.6, 2.1_

- [x] 2. Implement GemStore trait and core data types
  - [x] 2.1 Define `Gem` and `GemPreview` structs in `store.rs`
    - Implement all fields per design: id, source_type, source_url, domain, title, author, description, content, source_meta, captured_at
    - Add Serialize/Deserialize derives
    - _Requirements: 1.6_
  
  - [x] 2.2 Define `GemStore` trait in `store.rs`
    - Add async_trait annotation
    - Define methods: save, get, list, search, delete with proper signatures
    - Ensure trait is Send + Sync for Tauri state management
    - _Requirements: 1.1, 1.2_

- [x] 3. Implement SqliteGemStore
  - [x] 3.1 Create SqliteGemStore struct with Connection wrapper
    - Use `Arc<Mutex<Connection>>` for thread-safe access
    - Implement `new()` method that initializes at `~/.jarvis/gems.db`
    - Implement `new_in_memory()` method for testing (use #[cfg(test)] for unit tests in same crate)
    - Note: Integration tests should use unit test helpers or create temporary file-based stores
    - _Requirements: 2.1, 2.2_
  
  - [x] 3.2 Implement schema initialization
    - Create `gems` table with all columns and UNIQUE constraint on source_url
    - Create `gems_fts` FTS5 virtual table indexed on title, description, content
    - Create triggers for FTS5 sync (insert, update, delete)
    - _Requirements: 2.3, 2.4_
  
  - [x]* 3.3 Write unit tests for schema initialization
    - Test that gems table exists with correct columns
    - Test that gems_fts table exists
    - Test that triggers are created
    - _Requirements: 2.3, 2.4_
  
  - [x] 3.4 Implement GemStore::save method
    - Use INSERT ... ON CONFLICT(source_url) DO UPDATE for upsert
    - Query back the saved gem to return correct ID (handles conflict case)
    - Handle JSON serialization of source_meta
    - _Requirements: 3.1, 3.5_
  
  - [x] 3.5 Write property test for save-retrieve round trip
    - **Property 1: Save-Retrieve Round Trip**
    - **Validates: Requirements 3.1, 3.6**
  
  - [ ]* 3.6 Write property test for UUID generation
    - **Property 2: UUID Generation Validity**
    - **Validates: Requirements 3.2**
  
  - [ ]* 3.7 Write property test for timestamp validity
    - **Property 3: ISO 8601 Timestamp Validity**
    - **Validates: Requirements 3.3**
  
  - [ ]* 3.8 Write property test for field mapping
    - **Property 4: PageGist to Gem Field Mapping**
    - Verify all fields map correctly: title, author, description, content_excerpt→content, extra→source_meta, url→source_url, domain, and source_type enum→string
    - **Validates: Requirements 3.4**
  
  - [x] 3.9 Write unit test for upsert behavior
    - Save same source_url twice, verify only one gem exists
    - Verify second save updates the existing record
    - Verify FTS5 sync after upsert: search for updated title/content returns the gem
    - Test that old content is no longer searchable after upsert
    - _Requirements: 3.5_
  
  - [x] 3.10 Implement GemStore::get method
    - Query by ID with optional() to return Option<Gem>
    - Handle row-to-Gem conversion with proper error handling
    - _Requirements: 3.6_
  
  - [x] 3.11 Implement GemStore::list method
    - Query with ORDER BY captured_at DESC
    - Support limit and offset parameters
    - Convert Gem to GemPreview with content truncation
    - _Requirements: 4.1, 4.2, 4.3, 4.5_
  
  - [ ]* 3.12 Write property test for list ordering
    - **Property 6: List Ordering**
    - **Validates: Requirements 4.1**
  
  - [ ]* 3.13 Write property test for pagination limit
    - **Property 7: Pagination Limit**
    - **Validates: Requirements 4.2**
  
  - [ ]* 3.14 Write property test for pagination offset
    - **Property 8: Pagination Offset**
    - **Validates: Requirements 4.3**
  
  - [ ]* 3.15 Write unit tests for content truncation
    - Test truncation at exactly 200 characters
    - Test with multi-byte UTF-8 characters (emoji, Chinese)
    - Test content shorter than 200 characters (no truncation)
    - _Requirements: 4.5_
  
  - [x] 3.16 Implement GemStore::search method
    - Use FTS5 MATCH query with rank ordering
    - Handle empty query by delegating to list()
    - Support limit parameter
    - Convert results to GemPreview with truncation
    - Catch FTS5 syntax errors (unmatched quotes, invalid syntax) and return user-friendly error message
    - _Requirements: 5.1, 5.2, 5.3, 5.5, 5.6_
  
  - [ ]* 3.17 Write property test for search field coverage
    - **Property 10: Search Field Coverage**
    - **Validates: Requirements 5.2, 5.6**
  
  - [ ]* 3.18 Write unit tests for search edge cases
    - Test empty query returns same as list
    - Test search with special characters (quotes, ampersands)
    - Test search with no results
    - _Requirements: 5.5_
  
  - [x] 3.19 Implement GemStore::delete method
    - Delete by ID and check rows_affected
    - Return error if gem not found
    - _Requirements: 6.1, 6.2, 6.3_
  
  - [ ]* 3.20 Write property test for delete removes gem
    - **Property 11: Delete Removes Gem**
    - **Validates: Requirements 6.2, 6.4**
  
  - [ ]* 3.21 Write property test for delete non-existent errors
    - **Property 12: Delete Non-Existent Errors**
    - **Validates: Requirements 6.3**

- [x] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Implement Tauri commands
  - [x] 5.1 Add gems commands to src/commands.rs
    - Import GemStore trait and types
    - Add helper function to map PageGist to Gem
    - _Requirements: 1.3_
  
  - [x] 5.2 Implement save_gem command
    - Accept PageGist parameter
    - Generate UUID v4 for gem ID
    - Capture current timestamp in ISO 8601 format
    - Map PageGist fields to Gem fields
    - Merge published_date and image_url into source_meta alongside extra field
    - Call gem_store.save() via State
    - Return saved Gem to frontend
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.6, 3.7_
  
  - [x] 5.3 Implement list_gems command
    - Accept optional limit (default 50) and offset (default 0)
    - Call gem_store.list() via State
    - Return Vec<GemPreview>
    - _Requirements: 4.1, 4.2, 4.3_
  
  - [x] 5.4 Implement search_gems command
    - Accept query string and optional limit (default 50)
    - Call gem_store.search() via State
    - Return Vec<GemPreview>
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  
  - [x] 5.5 Implement delete_gem command
    - Accept gem ID string
    - Call gem_store.delete() via State
    - Return success or error
    - _Requirements: 6.1, 6.2, 6.3, 6.4_
  
  - [x] 5.6 Implement get_gem command
    - Accept gem ID string
    - Call gem_store.get() via State
    - Return Option<Gem>
    - _Requirements: 3.6_

- [x] 6. Integrate GemStore into Tauri application
  - [x] 6.1 Initialize SqliteGemStore in lib.rs setup() function
    - Create SqliteGemStore instance with error handling
    - Wrap in Arc<dyn GemStore> for trait object
    - Register with app.manage()
    - _Requirements: 1.4, 2.5_
  
  - [x] 6.2 Register gems commands in invoke_handler
    - Add save_gem, list_gems, search_gems, delete_gem, get_gem to handler
    - _Requirements: 1.3_
  
  - [ ]* 6.3 Write integration test for command invocation
    - Test that commands can be invoked via Tauri
    - Test error propagation from store to frontend
    - Note: Use temporary file-based store or mock GemStore for integration tests (new_in_memory is #[cfg(test)] only)
    - _Requirements: 1.3_

- [x] 7. Checkpoint - Ensure backend tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Implement frontend TypeScript types
  - [x] 8.1 Add Gem and GemPreview interfaces to src/state/types.ts
    - Match Rust struct fields exactly
    - Use proper TypeScript types (string | null for Option<String>)
    - _Requirements: 7.1, 8.1_

- [x] 9. Enhance GistCard component with Save Gem button
  - [x] 9.1 Add save functionality to GistCard component
    - Remove the existing "Export" button from GistCard UI (replaced by Save Gem)
    - Add "Save Gem" button alongside remaining actions (Copy button)
    - Implement handleSave that calls save_gem command via invoke
    - Add saved/saving state management
    - Add error state display
    - Change button to "Saved" (disabled) after successful save
    - Note: export_gist Tauri command remains available for programmatic use (Req 9.2)
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 9.2_
  
  - [ ]* 9.2 Write unit tests for GistCard save functionality
    - Test button click triggers save_gem command
    - Test button state changes after save
    - Test error display on save failure
    - _Requirements: 7.2, 7.3, 7.4_

- [x] 10. Implement GemsPanel component
  - [x] 10.1 Create GemsPanel component
    - Add header with title and close button
    - Add search input with debounced onChange (300ms)
    - Add scrollable gems list container
    - Add empty state message
    - Load gems on mount via list_gems command
    - _Requirements: 8.1, 8.2, 8.3, 8.7, 8.8_
  
  - [x] 10.2 Implement search functionality
    - Debounce search input at 300ms
    - Call search_gems command when query changes
    - Call list_gems when query is empty
    - Update gems list with results
    - _Requirements: 8.6_
  
  - [x] 10.3 Create GemCard sub-component
    - Display source type badge
    - Display title, domain, author (if present)
    - Display description (if present)
    - Display content preview (truncated)
    - Display captured date (formatted)
    - Add "Open" button that uses @tauri-apps/plugin-shell open() to launch source_url in default browser
    - Add "Delete" button with confirmation
    - _Requirements: 8.4_
  
  - [x] 10.4 Implement delete functionality
    - Show confirmation dialog before delete
    - Call delete_gem command
    - Remove gem from local state on success
    - Display error on failure
    - _Requirements: 8.5_
  
  - [ ]* 10.5 Write unit tests for GemsPanel
    - Test gems load on mount
    - Test search debouncing
    - Test delete confirmation and execution
    - Test empty state display
    - _Requirements: 8.1, 8.6, 8.7, 8.8_

- [x] 11. Integrate GemsPanel into main application
  - [x] 11.1 Add "Gems" navigation button to main UI
    - Add button to hamburger menu or toolbar
    - Toggle GemsPanel visibility on click
    - _Requirements: 8.1_
  
  - [x] 11.2 Add GemsPanel to App component
    - Import and render GemsPanel conditionally
    - Pass onClose handler to toggle visibility
    - _Requirements: 8.1_

- [x] 12. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Property tests use proptest with minimum 100 iterations per test
- Each property test references its design document property number
- Unit tests focus on specific examples and edge cases
- Content truncation uses character count (not byte offset) for safe UTF-8 handling
- SqliteGemStore uses bundled rusqlite feature for cross-platform consistency
- GemStore trait allows future implementations (ApiGemStore, CompositeGemStore) without changing commands
- All existing functionality (prepare_tab_gist, export_gist, browser tabs) remains unchanged
