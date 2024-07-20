fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");

    // only for githut action check, according to [[feat] Prevent error when distDir doesn't exist](https://github.com/tauri-apps/tauri/issues/3142)
    // ../out is the default distDir
    std::fs::create_dir_all("../out").expect("failed to create out directory");
    tauri_build::build()
}
