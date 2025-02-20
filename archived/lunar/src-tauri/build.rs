use std::process::Command;

fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");

    // only for githut action check, according to [[feat] Prevent error when distDir doesn't exist](https://github.com/tauri-apps/tauri/issues/3142)
    // ../out is the default distDir
    std::fs::create_dir_all("../out").expect("failed to create out directory");
    std::fs::create_dir_all("./binaries").expect("failed to create binaries directory");
    std::fs::create_dir_all("./libs").expect("failed to create libs directory");

    // Determine the platform-specific extension
    let extension = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    // Execute the `rustc -vV` command to get Rust compiler information
    let output = Command::new("rustc").arg("-vV").output().unwrap().stdout;
    let rust_info = String::from_utf8(output).unwrap();

    // Extract the target triple
    let target_triple = rust_info
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or("Failed to determine platform target triple")
        .unwrap();

    let sidecar_path = format!("./binaries/mega-{}{}", target_triple, extension);
    let libra_path = format!("./binaries/libra-{}{}", target_triple, extension);

    let debug_path = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    std::fs::copy(format!("../../target/{}/mega{}", debug_path, extension), sidecar_path)
        .expect("Run cargo build to build mega bin for Lunar first");

    std::fs::copy(format!("../../target/{}/libra{}", debug_path, extension), libra_path)
        .expect("Run cargo build to build libra bin for Lunar first");

    // Copy libpipy due to target os
    #[cfg(target_os = "macos")]
    std::fs::copy(
        format!("../../target/{}/libpipy.dylib", debug_path),
        "./libs/libpipy.dylib",
    )
    .expect("copy libpipy failed");

    #[cfg(target_os = "linux")]
    std::fs::copy(
        format!("../../target/{}/libpipy.so", debug_path),
        "./libs/libpipy.so",
    )
    .expect("copy libpipy failed");

    #[cfg(target_os = "windows")]
    {
        if cfg!(debug_assertions) {
            std::fs::copy("../../target/debug/pipyd.dll", "./libs/pipyd.dll")
                .expect("copy libpipy failed");
        } else {
            std::fs::copy("../../target/release/pipy.dll", "./libs/pipy.dll")
                .expect("copy libpipy failed");
        }
    }
    tauri_build::build()
}
