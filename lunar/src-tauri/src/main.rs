// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::str::FromStr;
use std::sync::Arc;

use serde::Deserialize;
use tauri::api::process::{Command, CommandChild, CommandEvent};
use tauri::State;
use tokio::sync::Mutex;

#[derive(Default)]
struct ServiceState {
    child: Option<CommandChild>,
}
#[derive(Debug, Deserialize, Clone)]
struct MegaStartParams {
    pub bootstrap_node: String,
}

impl Default for MegaStartParams {
    fn default() -> Self {
        Self {
            bootstrap_node: String::from_str("http://34.84.172.121/relay").unwrap(),
        }
    }
}

#[tauri::command]
async fn start_mega_service(
    handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<ServiceState>>>,
    params: MegaStartParams,
) -> Result<String, String> {
    let resource_path = handle
        .path_resolver()
        .resource_dir()
        .expect("failed to resolve resource");
    let libs_dir = resource_path.join("libs");

    #[cfg(target_os = "macos")]
    std::env::set_var("DYLD_LIBRARY_PATH", libs_dir.to_str().unwrap());

    #[cfg(target_os = "linux")]
    std::env::set_var("LD_LIBRARY_PATH", libs_dir.to_str().unwrap());

    #[cfg(target_os = "windows")]
    std::env::set_var("PATH", format!("{};{}", libs_dir.to_str().unwrap(), std::env::var("PATH").unwrap()));

    let mut service_state = state.lock().await;
    if service_state.child.is_some() {
        return Err("Service is already running".into());
    }

    let (mut rx, child) = Command::new_sidecar("mega")
        .expect("Failed to create `mega` binary command")
        .args([
            "service",
            "http",
            "--bootstrap-node",
            &params.bootstrap_node,
        ])
        .spawn()
        .expect("Failed to spawn `Mega service`");

    service_state.child = Some(child);
    let cloned_state = Arc::clone(&state);
    // Sidecar output
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    print!("{}", line);
                }
                CommandEvent::Stderr(line) => {
                    eprint!("Sidecar stderr: {}", line);
                }
                CommandEvent::Terminated(payload) => {
                    if let Some(code) = payload.code {
                        if code == 0 {
                            println!("Sidecar executed successfully.");
                        } else {
                            eprintln!("Sidecar failed with exit code: {}", code);
                        }
                    } else if let Some(signal) = payload.signal {
                        eprintln!("Sidecar terminated by signal: {}", signal);
                    }
                    // update ServiceState child
                    let mut service_state = cloned_state.lock().await;
                    service_state.child = None;
                    break;
                }
                _ => {}
            }
        }
    });
    Ok(resource_path.to_str().unwrap().to_string())
}

#[tauri::command]
async fn stop_mega_service(state: State<'_, Arc<Mutex<ServiceState>>>) -> Result<(), String> {
    let mut service_state = state.lock().await;
    if let Some(child) = service_state.child.take() {
        child.kill().map_err(|e| e.to_string())?;
    } else {
        println!("Mega Service is not running");
    }
    Ok(())
}

#[tauri::command]
async fn restart_mega_service(
    handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<ServiceState>>>,
    params: MegaStartParams,
) -> Result<String, String> {
    stop_mega_service(state.clone()).await?;
    start_mega_service(handle, state, params).await
}

#[tauri::command]
async fn mega_service_status(state: State<'_, Arc<Mutex<ServiceState>>>) -> Result<bool, String> {
    let service_state = state.lock().await;
    Ok(service_state.child.is_some())
}

fn main() {
    // let params = MegaStartParams::default();
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(ServiceState::default())))
        .invoke_handler(tauri::generate_handler![
            start_mega_service,
            stop_mega_service,
            restart_mega_service,
            mega_service_status
        ])
        .setup(|_| {
            // let app_handle = app.handle();
            // let state = app.state::<Arc<Mutex<ServiceState>>>().clone();
            // tauri::async_runtime::spawn(async move {
            //     if let Err(e) = start_mega_service(state, params).await {
            //         eprintln!("Failed to restart rust_service: {}", e);
            //     } else {
            //         println!("Rust service restarted successfully");
            //     }
            // });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
