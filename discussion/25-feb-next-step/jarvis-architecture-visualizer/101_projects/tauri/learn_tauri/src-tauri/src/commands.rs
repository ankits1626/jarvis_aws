// src-tauri/src/commands.rs

// This tag turns a regular Rust function into
// something the frontend can call
use std::sync::Mutex;

pub struct AppState {
    pub visit_count: Mutex<i32>,
}

#[tauri::command]
pub fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to Jarvis.", name)
}

// Want to add another command? Same pattern:
#[tauri::command]
pub fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

#[tauri::command]
pub fn system_info() -> String {
    "Hello from system_info command!".to_string()
}

#[tauri::command]
pub fn increment_counter(state: tauri::State<'_, AppState>) -> i32 {
    let mut count = state.visit_count.lock().unwrap();
    *count += 1;
    *count
}
