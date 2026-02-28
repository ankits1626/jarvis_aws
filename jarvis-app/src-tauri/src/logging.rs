// File-based logging — captures all eprintln! output to a timestamped log file.
//
// Creates a new log file on every app launch:
//   ~/Library/Application Support/com.jarvis.app/logs/jarvis-2026-03-01_14-30-00.log
//
// Keeps last 5 log files, deletes older ones.

use std::fs;
use std::io::{self, Write};
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize file logging. Call once at app startup, before any eprintln! calls.
///
/// Sets up stderr to write to both the terminal AND a log file via an OS-level
/// pipe + tee thread. All existing eprintln! calls throughout the codebase
/// automatically go to the log file.
pub fn init(logs_dir: &std::path::Path) {
    INIT.call_once(|| {
        if let Err(e) = setup_logging(logs_dir) {
            eprintln!("Warning: Failed to initialize file logging: {}", e);
        }
    });
}

fn setup_logging(logs_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create logs directory
    fs::create_dir_all(logs_dir)?;

    // Rotate old logs (keep last 5)
    rotate_logs(logs_dir, 5)?;

    // Create new log file with timestamp
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let log_file_path = logs_dir.join(format!("jarvis-{}.log", timestamp));

    // Open log file
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;

    eprintln!("Logging: Writing to {}", log_file_path.display());

    // Create a pipe: redirect stderr fd to write end,
    // then tee from read end to both original stderr and log file.
    let mut pipe_fds = [0i32; 2];
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
        return Err("Failed to create pipe".into());
    }
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    // Save original stderr fd
    let original_stderr_fd = unsafe { libc::dup(2) };
    if original_stderr_fd < 0 {
        return Err("Failed to dup stderr".into());
    }

    // Redirect stderr to the write end of our pipe
    if unsafe { libc::dup2(write_fd, 2) } < 0 {
        return Err("Failed to redirect stderr".into());
    }
    unsafe { libc::close(write_fd) };

    // Wrap fds into File objects for the tee thread
    let read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut original_stderr = unsafe { std::fs::File::from_raw_fd(original_stderr_fd) };
    let mut log_writer = log_file;

    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(read_file);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    // Write to original stderr (terminal)
                    let _ = writeln!(original_stderr, "{}", line);
                    // Write to log file with timestamp
                    let ts = chrono::Local::now().format("%H:%M:%S%.3f");
                    let _ = writeln!(log_writer, "[{}] {}", ts, line);
                    let _ = log_writer.flush();
                }
                Err(_) => break,
            }
        }
    });

    Ok(())
}

/// Delete old log files, keeping the most recent `keep` files.
fn rotate_logs(logs_dir: &std::path::Path, keep: usize) -> Result<(), io::Error> {
    let mut log_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

    for entry in fs::read_dir(logs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("log")
            && path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("jarvis-"))
                .unwrap_or(false)
        {
            if let Ok(metadata) = entry.metadata() {
                let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
                log_files.push((path, modified));
            }
        }
    }

    // Sort newest first
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Delete everything beyond `keep`
    for (path, _) in log_files.iter().skip(keep) {
        eprintln!("Logging: Removing old log {}", path.display());
        let _ = fs::remove_file(path);
    }

    Ok(())
}

/// Get the logs directory path.
pub fn logs_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("com.jarvis.app").join("logs"))
}
