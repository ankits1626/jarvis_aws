# Architectural Decisions

This file logs architectural decisions (ADRs) with context and trade-offs.

## Format

### ADR-XXX: Decision Title (YYYY-MM-DD)

**Context:**
- Why the decision was needed

**Decision:**
- What was chosen

**Alternatives Considered:**
- Option -> Why rejected

**Consequences:**
- Benefits and trade-offs

## Entries

### ADR-002: IntelligenceKit Sidecar Spawn Approach (2026-02-24)

**Context:**
- IntelligenceKit needs to communicate via synchronous NDJSON request-response (write command → read one response line)
- JarvisListen uses `tauri_plugin_shell` with event-based stdout delivery
- Need to decide whether to follow JarvisListen's pattern or use a different approach

**Decision:**
- Use `tokio::process::Command` directly for IntelligenceKit (not `tauri_plugin_shell`)
- Still register IntelligenceKit in `externalBin` for bundling
- Still use Tauri's `PathResolver` to locate the binary

**Alternatives Considered:**
- Use `tauri_plugin_shell` like JarvisListen → Rejected: Event-based stdout delivery (`Receiver<CommandEvent>`) makes synchronous request-response awkward. Would require background task routing events into per-request channels, adding complexity and latency.
- Use `std::process::Command` → Rejected: Need async I/O for non-blocking operations in Tauri commands

**Consequences:**
- Direct process control enables clean synchronous NDJSON protocol
- Simpler implementation (no event routing layer)
- Lower latency (no event queue)
- Diverges from JarvisListen pattern (but for good reason - different communication requirements)
- Still benefits from Tauri's binary bundling and path resolution

### ADR-001: Use Tauri for Desktop App (2026-02-24)

**Context:**
- Need a desktop application (Jarvis) with native capabilities
- App needs to interact with browser data and local system

**Decision:**
- Use Tauri (Rust backend + web frontend) for the desktop application

**Alternatives Considered:**
- Electron -> Rejected: heavier resource usage, larger bundle size
- Native app -> Rejected: platform-specific code, slower development

**Consequences:**
- Smaller bundle size and lower memory usage than Electron
- Rust backend for performance-critical operations
- Web frontend for rapid UI development
- Requires Rust knowledge for backend commands


### ADR-003: Model Catalog Duplication in MlxProvider (2026-02-27)

**Context:**
- MlxProvider needs to look up model capabilities (audio, text) when loading models
- The authoritative catalog lives in LlmModelManager
- Creating a dependency from MlxProvider → LlmModelManager would create circular dependencies (LlmModelManager uses MlxProvider for downloads)

**Decision:**
- Duplicate the catalog mapping in MlxProvider's `lookup_capabilities()` function
- Use exact directory name matching (equality checks) as primary strategy
- Fall back to `contains()` matching for flexibility with future model name variations
- Accept that the catalog must be updated in two places when adding new models

**Alternatives Considered:**
- Extract catalog to shared module → Rejected: Adds complexity for a small, rarely-changing catalog (6 models)
- Pass capabilities from Rust to Python at load time → Accepted: This is already done, but lookup is still needed for validation
- Use only `contains()` matching → Rejected: Prone to false positives (e.g., "qwen3-14b-llama-3.2-3b-hybrid" would match multiple patterns)

**Consequences:**
- Benefits:
  - No circular dependencies
  - Exact matching prevents false positives
  - Fallback matching provides flexibility
  - Simple implementation for small catalog
- Trade-offs:
  - Catalog must be updated in two places (LlmModelManager and MlxProvider)
  - Risk of desynchronization if one is updated without the other
- Mitigation:
  - Document the duplication clearly in code comments
  - Keep catalog small and stable
  - Consider extracting to shared module if catalog grows significantly (>20 models)
