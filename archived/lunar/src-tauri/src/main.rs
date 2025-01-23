// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{env, fs, thread, time};

use serde::Deserialize;
use tauri::Manager;
use tauri::State;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

#[derive(Default)]
struct ServiceState {
    child: Option<CommandChild>,
    with_relay: bool,
}

impl Drop for ServiceState {
    fn drop(&mut self) {
        if let Some(child_process) = self.child.take() {
            child_process
                .kill()
                .expect("Failed to kill sidecar process");
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
struct MegaStartParams {
    pub bootstrap_node: Option<String>,
}

fn set_up_lib(handle: tauri::AppHandle) {
    let resource_path = handle.path().resource_dir().expect("home dir not found");
    let libs_dir = resource_path.join("libs");

    #[cfg(target_os = "macos")]
    std::env::set_var("DYLD_LIBRARY_PATH", libs_dir.to_str().unwrap());

    #[cfg(target_os = "linux")]
    std::env::set_var("LD_LIBRARY_PATH", libs_dir.to_str().unwrap());

    #[cfg(target_os = "windows")]
    std::env::set_var(
        "PATH",
        format!(
            "{};{}",
            libs_dir.to_str().unwrap(),
            std::env::var("PATH").unwrap()
        ),
    );
}

#[tauri::command]
fn start_mega_service(
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<ServiceState>>>,
    params: MegaStartParams,
) -> Result<(), String> {
    let mut service_state = state.lock().unwrap();
    if service_state.child.is_some() {
        return Err("Service is already running".into());
    }

    let args = if let Some(ref addr) = params.bootstrap_node {
        service_state.with_relay = true;
        vec!["service", "http", "--bootstrap-node", addr]
    } else {
        service_state.with_relay = false;
        vec!["service", "http"]
    };

    let sidecar_command = app
        .shell()
        .sidecar("mega")
        .expect("Failed to create `mega` binary command");

    let (mut rx, child) = sidecar_command
        .args(args)
        .spawn()
        .expect("Failed to spawn `Mega service`");

    service_state.child = Some(child);
    let cloned_state = Arc::clone(&state);
    // Sidecar output
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    // line to string
                    // print!("{}", line);
                    print!("Sidecar stdout: {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    eprint!("Sidecar stderr: {}", String::from_utf8_lossy(&line));
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
                    let mut service_state = cloned_state.lock().unwrap();
                    service_state.child = None;
                    service_state.with_relay = false;
                    break;
                }
                _ => {}
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn stop_mega_service(state: State<'_, Arc<Mutex<ServiceState>>>) -> Result<(), String> {
    let mut service_state = state.lock().unwrap();
    if let Some(child) = service_state.child.take() {
        child.kill().map_err(|e| e.to_string())?;
    } else {
        println!("Mega Service is not running");
    }
    Ok(())
}

#[tauri::command]
fn restart_mega_service(
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<ServiceState>>>,
    params: MegaStartParams,
) -> Result<(), String> {
    stop_mega_service(state.clone())?;
    // wait for process exit
    thread::sleep(time::Duration::from_millis(1000));
    start_mega_service(app, state, params)?;
    Ok(())
}

#[tauri::command]
fn mega_service_status(state: State<'_, Arc<Mutex<ServiceState>>>) -> Result<(bool, bool), String> {
    let service_state = state.lock().unwrap();
    Ok((service_state.child.is_some(), service_state.with_relay))
}

#[tauri::command]
fn clone_repository(app: tauri::AppHandle, repo_url: String, name: String) -> Result<(), String> {
    let home = match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => path,
        _ => {
            println!("Unable to get your home dir!");
            PathBuf::new()
        }
    };
    let target_dir = home.join(".mega").join(name.clone());

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).unwrap();
    }
    let app_clone = app.clone();
    let output = tauri::async_runtime::block_on(async {
        app_clone
            .shell()
            .sidecar("libra")
            .expect("Failed to create `libra` binary command")
            .args(["clone", &repo_url, target_dir.to_str().unwrap()])
            .output()
            .await
    })
    .map_err(|e| format!("Failed to execute process: {}", e))?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    change_remote_url(app.to_owned(), target_dir.clone(), name)?;
    push_to_new_remote(app, target_dir)?;
    Ok(())
}

fn change_remote_url(
    app: tauri::AppHandle,
    repo_path: PathBuf,
    name: String,
) -> Result<(), String> {
    tauri::async_runtime::block_on(async {
        app.shell()
            .sidecar("libra")
            .expect("Failed to create `libra` binary command")
            .args(["remote", "remove", "origin"])
            .current_dir(repo_path.clone())
            .output()
            .await
    })
    .map_err(|e| format!("Failed to execute process: {}", e))?;

    let output = tauri::async_runtime::block_on(async {
        app.shell()
            .sidecar("libra")
            .expect("Failed to create `libra` binary command")
            .args([
                "remote",
                "add",
                "origin",
                &format!("http://localhost:8000/third-part/{}", name),
            ])
            .current_dir(repo_path.clone())
            .output()
            .await
    })
    .map_err(|e| format!("Failed to execute process: {}", e))?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn push_to_new_remote(app: tauri::AppHandle, repo_path: PathBuf) -> Result<(), String> {
    let output = tauri::async_runtime::block_on(async {
        app.shell()
            .sidecar("libra")
            .expect("Failed to create `libra` binary command")
            .args(["push", "origin", "master"])
            .current_dir(repo_path)
            .output()
            .await
    })
    .map_err(|e| format!("Failed to execute process: {}", e))?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn main() {
    // let params = MegaStartParams::default();
    let params = MegaStartParams {
        bootstrap_node: Some("http://gitmono.org/relay".to_string()),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Arc::new(Mutex::new(ServiceState::default())))
        .invoke_handler(tauri::generate_handler![
            start_mega_service,
            stop_mega_service,
            restart_mega_service,
            mega_service_status,
            clone_repository
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            set_up_lib(app_handle.to_owned());
            let state = app.state::<Arc<Mutex<ServiceState>>>().clone();
            if let Err(e) = start_mega_service(app_handle, state, params) {
                eprintln!("Failed to restart rust_service: {}", e);
            } else {
                println!("Rust service restarted successfully");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
