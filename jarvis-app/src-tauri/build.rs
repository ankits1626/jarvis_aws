fn main() {
    // Add ~/.jarvis/lib to the native library search path for libvosk
    if let Some(home) = dirs::home_dir() {
        let lib_path = home.join(".jarvis/lib");
        if lib_path.exists() {
            println!("cargo:rustc-link-search=native={}", lib_path.display());
            // Set rpath so the dylib is found at runtime during development
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());

            // Copy libvosk.dylib to src-tauri/libs/ so Tauri can bundle it into the .app
            let manifest_dir = std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").unwrap(),
            );
            let libs_dir = manifest_dir.join("libs");
            std::fs::create_dir_all(&libs_dir).ok();
            let src = lib_path.join("libvosk.dylib");
            let dst = libs_dir.join("libvosk.dylib");
            if src.exists() {
                std::fs::copy(&src, &dst).ok();
            }
        }
    }

    // Set rpath so the bundled .app finds libvosk.dylib in Contents/Frameworks/
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");

    tauri_build::build()
}
