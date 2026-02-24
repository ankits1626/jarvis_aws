# Key Facts

Project constants, configuration, and frequently-needed non-sensitive information.

> **NEVER store passwords, API keys, or sensitive credentials in this file.**

## Project Overview

- **Project Name**: Jarvis AWS
- **Description**: Desktop assistant app built with Tauri (Rust + web frontend)
- **Repository**: jarvis_aws

## Project Structure

- **Tauri App**: `jarvis-app/src-tauri/` (Rust backend)
- **Frontend**: `jarvis-app/` (web frontend)
- **Specs**: `.kiro/specs/` (feature specifications)

## Key Technologies

- **Desktop Framework**: Tauri
- **Backend Language**: Rust
- **Package Manager**: Cargo (Rust)

## Features

- YouTube browsing observation
- Content extractors (Medium, Email, ChatGPT)
- Gems system (`.kiro/specs/jarvis-gems/`)

## Important Paths

- Tauri commands: `jarvis-app/src-tauri/src/commands.rs`
- App entry: `jarvis-app/src-tauri/src/lib.rs`
- Gems module: `jarvis-app/src-tauri/src/gems/`
