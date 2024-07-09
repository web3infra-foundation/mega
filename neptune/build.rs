use std::{fs, path::Path};

use cmake::Config;
fn build_agent_ui() {
    let ui = Path::new("libs/ztm/agent/gui");
    if cfg!(feature = "agent-ui") {
        if !ui.exists() {
            // run npm run build in the libs/ztm/gui
            let _ = std::process::Command::new("npm")
                .current_dir("libs/ztm/gui")
                .arg("run")
                .arg("build")
                .output()
                .expect("failed to run npm run build in ztm/gui");
        }
    } else if ui.exists() {
        std::fs::remove_dir_all(ui).expect("failed to remove agent/gui");
    }
}

fn parse_args_to_rustc(dst: &Path) {
    // ** `cargo:rustc-*` format is used to pass information to the cargo build system

    // parse to `rustc` to look for dynamic library, used in running
    let origin_path = if cfg!(target_os = "macos") {
        "@executable_path"
    } else if cfg!(target_os = "linux") {
        "$ORIGIN"
    } else {
        "" // windows auto search excutable path
    };
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}/build,-rpath,{}",
        dst.display(),
        origin_path
    );

    // add the path to the library to the linker search path, used in build
    println!("cargo:rustc-link-search={}/build", dst.display());

    println!("cargo:rustc-link-lib=pipy");
}

/// copy `mega-app/` to `libs/ztm/agent/apps/mega`
fn copy_mega_apps() {
    let src = Path::new("mega_app");
    let dst = Path::new("libs/ztm/agent/apps/mega");
    if src.exists() {
        if dst.exists() {
            fs::remove_dir_all(dst).expect("failed to remove origin agent/apps");
        }
        // std::fs::copy(src, dst).expect("failed to copy mega to agent/apps");
        copy_dir_all(src, dst).expect("failed to copy mega to agent/apps");
    }
}

fn main() {
    copy_mega_apps();

    build_agent_ui();

    /* compile ztm & pipy */
    // run npm install in the libs/ztm
    let _ = std::process::Command::new("npm")
        .current_dir("libs/ztm/pipy")
        .arg("install")
        .output()
        .expect("failed to run npm install in ztm/pipy");

    let mut config = Config::new("libs/ztm/pipy");

    // set to use clang/clang++ to compile
    config.define("CMAKE_C_COMPILER", "clang");
    config.define("CMAKE_CXX_COMPILER", "clang++");

    // compile ztm in pipy
    config.define("PIPY_SHARED", "ON");
    config.define("PIPY_GUI", "OFF");
    config.define("PIPY_CODEBASES", "ON");
    config.define(
        "PIPY_CUSTOM_CODEBASES",
        "ztm/agent:../agent,ztm/hub:../hub,ztm/ca:../ca",
    );

    std::env::set_var("CMAKE_BUILD_PARALLEL_LEVEL", "4");

    // add target to build
    config.no_build_target(true);

    // build
    let dst = config.build();

    parse_args_to_rustc(&dst);
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
