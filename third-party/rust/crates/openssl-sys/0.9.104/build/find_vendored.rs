use openssl_src;
use std::path::PathBuf;
use std::{env};

use super::env as env_util;

pub fn get_openssl(target: &str) -> (Vec<PathBuf>, PathBuf) {
    let openssl_config_dir = env_util("OPENSSL_CONFIG_DIR");

    println!("cargo:warning=Using vendored OpenSSL");
    
    // Print target directory for debugging
    if let Ok(out_dir) = env::var("OUT_DIR") {
        println!("cargo:warning=OUT_DIR: {}", out_dir);
    }
    
    // Print current directory
    if let Ok(current_dir) = env::current_dir() {
        println!("cargo:warning=Current directory: {}", current_dir.display());
    }

    // Create a new build object with custom paths
    let mut openssl_src_build = openssl_src::Build::new();
    
    // Try to find OpenSSL source relative to the current project
    let mut project_openssl_path = None;
    
    // First try using the explicitly provided directory
    if let Some(value) = openssl_config_dir {
        println!("cargo:warning=Using OPENSSL_CONFIG_DIR: {}", value.to_string_lossy());
        openssl_src_build.openssl_dir(PathBuf::from(&value));
        project_openssl_path = Some(PathBuf::from(value));
    } else {
        // Try to find OpenSSL source in the current directory
        if let Ok(current_dir) = env::current_dir() {
            // Try different possible locations
            let possible_locations = [
                "openssl",                 // Direct subdirectory
                "third-party/openssl",     // In third-party dir
                "../openssl",              // Parent directory
                "deps/openssl",            // deps directory
            ];
            
            for location in possible_locations {
                let path = current_dir.join(location);
                if path.exists() && path.join("INSTALL").exists() {
                    println!("cargo:warning=Found OpenSSL source at: {}", path.display());
                    openssl_src_build.openssl_dir(&path);
                    project_openssl_path = Some(path);
                    break;
                }
            }
        }
    }
    
    // If we found a path, proceed with the build
    if let Some(path) = project_openssl_path {
        println!("cargo:warning=Using OpenSSL source at: {}", path.display());
        
        // Attempt to build OpenSSL from the found source
        match std::panic::catch_unwind(|| {
            let mut clone_build = openssl_src::Build::new();
            clone_build.openssl_dir(&path);
            clone_build.build()
        }) {
            Ok(artifacts) => {
                println!("cargo:vendored=1");
                println!(
                    "cargo:root={}",
                    artifacts.lib_dir().parent().unwrap().display()
                );
                
                // Print paths for debugging
                println!("cargo:warning=OpenSSL lib directory: {}", artifacts.lib_dir().display());
                println!("cargo:warning=OpenSSL include directory: {}", artifacts.include_dir().display());

                return (
                    vec![artifacts.lib_dir().to_path_buf()],
                    artifacts.include_dir().to_path_buf(),
                );
            },
            Err(e) => {
                // Log error and fall back to normal OpenSSL
                println!("cargo:warning=Failed to build vendored OpenSSL from custom path: {:?}", e);
                println!("cargo:warning=Falling back to system OpenSSL");
            }
        }
    } else {
        println!("cargo:warning=Could not find OpenSSL source in project directory");
        println!("cargo:warning=Falling back to system OpenSSL");
    }
    
    // If we get here, we need to fall back to system OpenSSL
    super::find_normal::get_openssl(target)
}