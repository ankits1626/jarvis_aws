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
