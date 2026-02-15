fn main() {
    // Add ~/.jarvis/lib to the native library search path for libvosk
    if let Some(home) = dirs::home_dir() {
        let lib_path = home.join(".jarvis/lib");
        if lib_path.exists() {
            println!("cargo:rustc-link-search=native={}", lib_path.display());
            // Set rpath so the dylib is found at runtime too
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());
        }
    }
    tauri_build::build()
}
