//! Using shadow_rs to build-time information stored in Mega.
//!
//!
//!

fn main() -> shadow_rs::SdResult<()> {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
    shadow_rs::new()
}
