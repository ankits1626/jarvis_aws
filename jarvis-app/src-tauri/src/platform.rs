/// Platform detection and platform-specific operations
pub struct PlatformDetector;

impl PlatformDetector {
    /// Returns true if the current platform is supported for recording
    /// Currently only macOS is supported
    pub fn is_supported() -> bool {
        cfg!(target_os = "macos")
    }

    /// Returns the platform-specific sidecar binary name
    /// This matches the binary names in the binaries/ directory
    pub fn get_sidecar_name() -> &'static str {
        if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "JarvisListen-aarch64-apple-darwin"
            } else {
                "JarvisListen-x86_64-apple-darwin"
            }
        } else if cfg!(target_os = "windows") {
            "JarvisListen-x86_64-pc-windows-msvc"
        } else if cfg!(target_os = "linux") {
            "JarvisListen-x86_64-unknown-linux-gnu"
        } else {
            "JarvisListen-unknown"
        }
    }

    /// Opens the system settings for the current platform
    /// On macOS, opens the Screen Recording privacy settings
    /// Returns an error on non-macOS platforms
    #[cfg(target_os = "macos")]
    pub fn open_system_settings() -> Result<(), String> {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn()
            .map_err(|e| format!("Failed to open System Settings: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn open_system_settings() -> Result<(), String> {
        Err("System settings not available on this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported() {
        // Test that is_supported returns the correct value for the current platform
        #[cfg(target_os = "macos")]
        assert!(PlatformDetector::is_supported());

        #[cfg(not(target_os = "macos"))]
        assert!(!PlatformDetector::is_supported());
    }

    #[test]
    fn test_get_sidecar_name() {
        let name = PlatformDetector::get_sidecar_name();
        
        // Verify the name is not empty and contains expected patterns
        assert!(!name.is_empty());
        assert!(name.starts_with("JarvisListen-"));
        
        // Verify platform-specific naming
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert_eq!(name, "JarvisListen-aarch64-apple-darwin");
        
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        assert_eq!(name, "JarvisListen-x86_64-apple-darwin");
        
        #[cfg(target_os = "windows")]
        assert_eq!(name, "JarvisListen-x86_64-pc-windows-msvc");
        
        #[cfg(target_os = "linux")]
        assert_eq!(name, "JarvisListen-x86_64-unknown-linux-gnu");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_open_system_settings_macos() {
        // On macOS, this should succeed (though we can't verify the window actually opens in tests)
        // We just verify it doesn't panic or return an error
        let result = PlatformDetector::open_system_settings();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_open_system_settings_non_macos() {
        // On non-macOS platforms, this should return an error
        let result = PlatformDetector::open_system_settings();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not available on this platform"));
    }
}
