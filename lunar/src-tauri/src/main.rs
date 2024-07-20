// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn hello_string(name: &str) -> String {
    format!("Hello from Rust, {}!", name)
}

fn start_mega(config_path: &str) {
    let args_str = format!("-c \"{}\" service http", config_path);
    let args = args_str.split(' ').collect();
    mega::cli::parse(Some(args)).expect("failed to start mega");
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![hello_string])
        .setup(|app| {
            let resource_path = app
                .path_resolver()
                .resolve_resource("config.toml")
                .expect("failed to resolve config.toml resource");
            std::thread::spawn(move || {
                start_mega(resource_path.to_str().unwrap());
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
