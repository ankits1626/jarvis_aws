// VenvManager - manages an isolated Python virtual environment for the MLX sidecar
//
// Creates and maintains a venv at ~/.jarvis/venv/mlx/ with MLX dependencies installed.
// Uses a marker file (.jarvis-setup-complete) with a requirements hash to detect
// when dependencies need updating.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

/// Venv status reported to frontend
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VenvStatus {
    NotCreated,
    Ready,
    NeedsUpdate,
}

/// Progress event emitted during venv setup
#[derive(Debug, Clone, Serialize)]
pub struct VenvSetupProgress {
    pub phase: String,
    pub message: String,
}

const MARKER_FILE: &str = ".jarvis-setup-complete";

pub struct VenvManager {
    venv_dir: PathBuf,
    requirements_path: PathBuf,
}

impl VenvManager {
    /// Create a new VenvManager, resolving the requirements.txt path.
    pub fn new() -> Result<Self, String> {
        let home = dirs::home_dir()
            .ok_or("Cannot determine home directory")?;
        let venv_dir = home.join(".jarvis/venv/mlx");

        let requirements_path = Self::resolve_requirements_path()?;

        Ok(Self {
            venv_dir,
            requirements_path,
        })
    }

    /// Resolve requirements.txt path (production bundle or dev mode)
    fn resolve_requirements_path() -> Result<PathBuf, String> {
        // Production: Contents/Resources/sidecars/mlx-server/requirements.txt
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                if let Some(contents_dir) = exe_dir.parent() {
                    let prod_path = contents_dir
                        .join("Resources/sidecars/mlx-server/requirements.txt");
                    if prod_path.exists() {
                        return Ok(prod_path);
                    }
                }
            }
        }

        // Dev mode: CARGO_MANIFEST_DIR/sidecars/mlx-server/requirements.txt
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("sidecars/mlx-server/requirements.txt");
        if dev_path.exists() {
            return Ok(dev_path);
        }

        Err(format!(
            "requirements.txt not found. Checked:\n  - Contents/Resources/sidecars/mlx-server/requirements.txt\n  - {:?}",
            dev_path
        ))
    }

    /// Check venv status by examining marker file and requirements hash.
    pub fn status(&self) -> VenvStatus {
        let marker = self.venv_dir.join(MARKER_FILE);
        if !marker.exists() {
            return VenvStatus::NotCreated;
        }

        // Compare stored hash with current requirements hash
        match std::fs::read_to_string(&marker) {
            Ok(stored_hash) => {
                let current_hash = self.requirements_hash();
                if stored_hash.trim() == current_hash {
                    VenvStatus::Ready
                } else {
                    VenvStatus::NeedsUpdate
                }
            }
            Err(_) => VenvStatus::NotCreated,
        }
    }

    /// Get the venv Python path if the venv exists on disk.
    pub fn venv_python_path(&self) -> Option<PathBuf> {
        let python = self.venv_dir.join("bin/python3");
        if python.exists() {
            Some(python)
        } else {
            None
        }
    }

    /// Resolve which Python to use for sidecar spawning.
    /// Returns venv Python if ready, otherwise the base python_path from settings.
    pub fn resolve_python_path(&self, base_python: &str) -> String {
        if self.status() == VenvStatus::Ready {
            if let Some(venv_python) = self.venv_python_path() {
                return venv_python.to_string_lossy().to_string();
            }
        }
        base_python.to_string()
    }

    /// Create the venv and install dependencies.
    /// Emits progress events to the frontend.
    pub async fn setup(&self, base_python: &str, app_handle: &AppHandle) -> Result<(), String> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

        // Phase 1: Create venv
        self.emit_progress(app_handle, "creating_venv", "Creating Python virtual environment...");

        // Ensure parent directory exists
        if let Some(parent) = self.venv_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create venv parent directory: {}", e))?;
        }

        let output = Command::new(base_python)
            .args(["-m", "venv", &self.venv_dir.to_string_lossy()])
            .current_dir(&home)
            .output()
            .await
            .map_err(|e| format!("Failed to create venv: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err = format!("Venv creation failed: {}", stderr);
            self.emit_error(app_handle, &err);
            return Err(err);
        }

        eprintln!("VenvManager: Venv created at {:?}", self.venv_dir);

        // Phase 2: Install dependencies
        self.emit_progress(app_handle, "installing_deps", "Installing MLX dependencies (this may take a minute)...");

        let pip_path = self.venv_dir.join("bin/pip3");
        let output = Command::new(&pip_path)
            .args(["install", "-r", &self.requirements_path.to_string_lossy()])
            .current_dir(&home)
            .output()
            .await
            .map_err(|e| format!("Failed to run pip install: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err = format!("pip install failed: {}", stderr);
            self.emit_error(app_handle, &err);
            return Err(err);
        }

        eprintln!("VenvManager: Dependencies installed successfully");

        // Phase 3: Validate
        self.emit_progress(app_handle, "validating", "Validating MLX installation...");

        let venv_python = self.venv_dir.join("bin/python3");
        let output = Command::new(&venv_python)
            .args(["-c", "import mlx; import mlx_lm; import huggingface_hub; print('ok')"])
            .current_dir(&home)
            .output()
            .await
            .map_err(|e| format!("Validation command failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err = format!("MLX validation failed: {}", stderr);
            self.emit_error(app_handle, &err);
            return Err(err);
        }

        // Write marker file with requirements hash
        let hash = self.requirements_hash();
        let marker = self.venv_dir.join(MARKER_FILE);
        std::fs::write(&marker, &hash)
            .map_err(|e| format!("Failed to write marker file: {}", e))?;

        eprintln!("VenvManager: Setup complete (hash: {})", &hash[..16]);

        let _ = app_handle.emit("mlx-venv-setup-complete", ());
        Ok(())
    }

    /// Delete the entire venv directory.
    pub fn reset(&self) -> Result<(), String> {
        if self.venv_dir.exists() {
            std::fs::remove_dir_all(&self.venv_dir)
                .map_err(|e| format!("Failed to remove venv: {}", e))?;
            eprintln!("VenvManager: Venv removed at {:?}", self.venv_dir);
        }
        Ok(())
    }

    /// Compute SHA-256 hash of the requirements.txt file.
    fn requirements_hash(&self) -> String {
        match std::fs::read(&self.requirements_path) {
            Ok(contents) => {
                let mut hasher = Sha256::new();
                hasher.update(&contents);
                format!("{:x}", hasher.finalize())
            }
            Err(_) => "unknown".to_string(),
        }
    }

    fn emit_progress(&self, app_handle: &AppHandle, phase: &str, message: &str) {
        let _ = app_handle.emit("mlx-venv-setup-progress", VenvSetupProgress {
            phase: phase.to_string(),
            message: message.to_string(),
        });
    }

    fn emit_error(&self, app_handle: &AppHandle, error: &str) {
        let _ = app_handle.emit("mlx-venv-setup-error", serde_json::json!({ "error": error }));
    }
}
