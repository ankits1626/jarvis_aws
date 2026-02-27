# Root Cause: VenvManager Panic on Installed DMG

**Date:** 2026-02-27
**Symptom:** App panics on launch from installed DMG with "requirements.txt not found"

## The Error

```
Failed to initialize VenvManager: requirements.txt not found. Checked:
  - Contents/Resources/sidecars/mlx-server/requirements.txt
  - "/Users/ankit/code/learn/jarvis_aws/jarvis_aws/jarvis-app/src-tauri/sidecars/mlx-server/requirements.txt"
```

## Root Cause

**The `sidecars/` directory is not being bundled into the DMG.** The Tauri bundle config in `tauri.conf.json` has no `"resources"` field, so the entire `sidecars/mlx-server/` folder (containing `requirements.txt`, `server.py`, etc.) is never copied into the app bundle.

## Why It Works in Dev but Fails in Production

### Dev mode (`cargo tauri dev`)

VenvManager falls back to `CARGO_MANIFEST_DIR`, which points to the actual source tree:

```
src-tauri/sidecars/mlx-server/requirements.txt  → EXISTS ✓
```

### Production (installed DMG)

VenvManager resolves the path relative to the running executable:

```
/Applications/JarvisApp.app/Contents/MacOS/jarvis-app
  → parent: Contents/MacOS
  → parent: Contents
  → join: Contents/Resources/sidecars/mlx-server/requirements.txt  → DOES NOT EXIST ✗
```

The file doesn't exist because Tauri never copied it during bundling.

## Path Resolution Walkthrough

From `venv_manager.rs` (lines 52-77):

```rust
// Step 1: Try production path
if let Ok(exe) = std::env::current_exe() {
    if let Some(exe_dir) = exe.parent() {           // .../Contents/MacOS
        if let Some(contents_dir) = exe_dir.parent() { // .../Contents
            let prod_path = contents_dir
                .join("Resources/sidecars/mlx-server/requirements.txt");
            // This file was never bundled → not found
        }
    }
}

// Step 2: Try dev path using CARGO_MANIFEST_DIR
// Only set during `cargo build` / `cargo tauri dev` → not set in production
```

Both paths fail → panic.

## The Missing Config

Current `tauri.conf.json` bundle section:

```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [...],
  "externalBin": [...],
  "macOS": {
    "infoPlist": "Info.plist",
    "frameworks": ["libs/libvosk.dylib"]
  }
}
```

Notice: **no `"resources"` field**. Tauri's `resources` config is what tells the bundler to copy non-executable files into `Contents/Resources/` in the macOS app bundle.

## Other Code That Would Also Break

The same missing-resource problem affects two more files that look for `sidecars/mlx-server/server.py` using the same path pattern:

| File | Line | What it looks for |
|------|------|-------------------|
| `venv_manager.rs` | ~55 | `Resources/sidecars/mlx-server/requirements.txt` |
| `mlx_provider.rs` | ~98 | `Resources/sidecars/mlx-server/server.py` |
| `llm_model_manager.rs` | ~609 | `Resources/sidecars/mlx-server/server.py` |

All three would fail in production for the same reason — the files were never bundled.

## The Fix (for reference)

Add `"resources"` to the bundle config in `tauri.conf.json`:

```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [...],
  "externalBin": [...],
  "resources": ["sidecars/"],
  "macOS": {
    "infoPlist": "Info.plist",
    "frameworks": ["libs/libvosk.dylib"]
  }
}
```

This tells Tauri to copy the entire `sidecars/` directory into `Contents/Resources/sidecars/` during bundling, making all three path lookups resolve correctly in production.
