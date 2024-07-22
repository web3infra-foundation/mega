// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn hello_string(name: &str) -> String {
    format!("Hello from Rust, {}!", name)
}

fn start_mega() {
    let args_str = "service http".to_string();
    let args = args_str.split(' ').collect();
    mega::cli::parse(Some(args)).expect("failed to start mega");
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![hello_string])
        .setup(|_| {
            std::thread::spawn(move || {
                start_mega();
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
