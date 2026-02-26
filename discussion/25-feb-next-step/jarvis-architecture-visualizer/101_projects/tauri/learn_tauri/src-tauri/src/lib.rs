// src-tauri/src/lib.rs

mod commands;  // "I have a commands.rs file"

use commands::{greet, add_numbers, system_info, increment_counter, AppState};  // bring them into scope

pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            visit_count: std::sync::Mutex::new(0),
        })
        .invoke_handler(tauri::generate_handler![
            greet,        // ✅ frontend can call this
            add_numbers,  // ✅ and this
            system_info,
            increment_counter,
            // Any function NOT listed here = frontend can't access it
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}