use core::panic;
use std::{
    env::current_dir,
    fs,
    hash::Hasher,
    io::Read,
    path::{Path, PathBuf},
};

use cmake::Config;

/// copy `mega-app/` to `libs/ztm/agent/apps/mega`
fn copy_mega_apps() {
    let src = Path::new("mega");
    let dst = Path::new("libs/ztm/agent/apps/mega");
    assert!(src.exists(), "neptune/mega not exists");
    copy_dir_all(src, dst).unwrap_or_else(|e| {
        fs::remove_dir(dst).expect("failed to remove agent/apps/mega");
        panic!("failed to copy mega to agent/apps/mega: {}", e);
    });
}

/// copy `hub/main.js` to `libs/ztm/hub/main.js`
fn copy_mega_ztm_hub() {
    let path = Path::new("hub");
    if fs::metadata(path).is_err() {
        println!("neptune/hub does not exist, skip to copy");
        return;
    }
    let dst = Path::new("libs/ztm/hub");
    copy_dir_all(path, dst).unwrap_or_else(|e| {
        fs::remove_dir(dst).expect("failed to remove ztm/hub");
        panic!("failed to copy neptune/hub to neptune/libs/ztm/hub: {}", e);
    });
}

/// use npm to build agent ui in `libs/ztm/agent/gui`
fn npm_build_agent_ui() {
    let ui = Path::new("libs/ztm/agent/gui");
    if cfg!(feature = "agent-ui") {
        if !ui.exists() {
            // run npm run build in the libs/ztm/gui
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            let _ = std::process::Command::new("npm")
                .current_dir("libs/ztm/gui")
                .arg("run")
                .arg("build")
                .output()
                .expect("failed to run npm run build in ztm/gui");
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd.exe")
                .current_dir("libs/ztm/gui")
                .arg("/C")
                .arg("npm run build")
                .output()
                .expect("failed to run npm run build in ztm/gui");
        }
    } else if ui.exists() {
        std::fs::remove_dir_all(ui).expect("failed to remove agent/gui");
    }
}

fn parse_link_args_to_rustc(dst: &Path) {
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
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    println!("cargo:rustc-link-search={}/build", dst.display());
    #[cfg(target_os = "windows")] // windows's path is diffirent
    {
        let mut build_type = "Release";
        if cfg!(debug_assertions) {
            build_type = "Debug";
            // copy pipyd.lib to pipy.lib
            let src = dst.join("build/Debug/pipyd.lib");
            let dst = dst.join("build/Debug/pipy.lib");
            if src.exists() {
                fs::copy(src, dst).expect("failed to copy pipyd.lib to pipy.lib");
            }
        }
        println!(
            "cargo:rustc-link-search={}/build/{}",
            dst.display(),
            build_type
        );
    }

    println!("cargo:rustc-link-lib=pipy");
}

/// copy finally lib to target/debug or target/release
/// for example, in macos and debug mode, it copy target/debug/build/neptune-xxxx/out/libpipy.dylib to target/debug/libpipy.dylib
/// !only works when develop mega, didn't work if use `mega` as crate of other project
fn copy_lib_to_target(dst: &Path) {
    let source = {
        let _source: PathBuf = dst.join("build");
        if cfg!(target_os = "macos") {
            _source.join("libpipy.dylib")
        } else if cfg!(target_os = "linux") {
            _source.join("libpipy.so")
        } else if cfg!(target_os = "windows") {
            if cfg!(debug_assertions) {
                _source.join("Debug").join("pipyd.dll")
            } else {
                _source.join("Release").join("pipy.dll")
            }
        } else {
            println!("cargo:warning=unexpected target os");
            return;
        }
    };

    // !hack, only work directory is `$workspace/neptune`
    let target = {
        let mut _target = current_dir().unwrap();
        _target.pop(); // path: neptune/../
        if cfg!(debug_assertions) {
            _target.join("target").join("debug")
        } else {
            _target.join("target").join("release")
        }
    };
    if target.exists() {
        let target = target.join(source.file_name().unwrap());
        if target.exists() {
            fs::remove_file(&target).expect("failed to remove origin lib");
        }
        fs::copy(&source, &target).expect("failed to copy lib to target");
    } else {
        println!("neptune/../target/debug and neptune/../target/release not exists");
    }
}

// run npm install in the libs/ztm/pipy
fn npm_install_pipy() {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let _ = std::process::Command::new("npm")
        .current_dir("libs/ztm/pipy")
        .arg("install")
        .output()
        .expect("failed to run npm install in ztm/pipy");

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd.exe")
        .current_dir("libs/ztm/pipy")
        // .arg("install")
        .arg("/C")
        .arg("npm install")
        .output()
        .expect("failed to run npm install in ztm/pipy");
}

fn build() -> PathBuf {
    npm_build_agent_ui();

    /* compile ztm & pipy */
    npm_install_pipy();

    let mut config = Config::new("libs/ztm/pipy");

    // set to use clang/clang++ to compile if not in windows
    #[cfg(not(target_os = "windows"))]
    {
        config.define("CMAKE_C_COMPILER", "clang");
        config.define("CMAKE_CXX_COMPILER", "clang++");
    }
    #[cfg(target_os = "windows")]
    {
        config.generator("Visual Studio 17 2022");
        config.define("CMAKE_CXX_FLAGS", "/DWIN32 /D_WINDOWS /W3 /GR /EHsc"); // XXX enumerate possible parameters found
    }
    // compile ztm in pipy
    config.define("PIPY_SHARED", "ON");
    config.define("PIPY_GUI", "OFF");
    config.define("PIPY_CODEBASES", "ON");
    config.define(
        "PIPY_CUSTOM_CODEBASES",
        "ztm/agent:../agent,ztm/hub:../hub,ztm/ca:../ca",
    );

    // build, with half of the cpu
    let cups = num_cpus::get() - num_cpus::get() / 2;
    std::env::set_var("CMAKE_BUILD_PARALLEL_LEVEL", cups.to_string());
    config.build()
}

/// file/path list to detect change
/// list files because cargo didn't dupport exclude path
/// we didn't want to detect changes such as node_modules
fn return_if_change() {
    /* set return if changed to reduce build times, ref: `https://doc.rust-lang.org/cargo/reference/build-scripts.html#change-detection`` */
    println!("cargo:rerun-if-changed=mega");
    println!("cargo:rerun-if-changed=src");

    println!("cargo:rerun-if-changed=libs/ztm/agent");
    println!("cargo:rerun-if-changed=libs/ztm/hub");
    println!("cargo:rerun-if-changed=libs/ztm/ca");

    println!("cargo:rerun-if-changed=libs/ztm/pipy/src");
    println!("cargo:rerun-if-changed=libs/ztm/pipy/CMakeLists.txt");
    println!("cargo:rerun-if-changed=libs/ztm/pipy/include");
    println!("cargo:rerun-if-changed=libs/ztm/pipy/deps");
}

fn main() {
    // check submodule exists
    let check_file = Path::new("libs/ztm/pipy/CMakeLists.txt");
    if !check_file.exists() {
        panic!("Please run `git submodule update --init --recursive` to get the submodule");
    }
    return_if_change();

    copy_mega_apps();
    copy_mega_ztm_hub();
    let dst = build();
    parse_link_args_to_rustc(&dst);
    copy_lib_to_target(&dst); // optional, didn't work in all cases
}

/// recursively copy directory, didn't change file's timestamp if not changed
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fn hash_file(file: PathBuf) -> std::io::Result<u64> {
        let mut file = fs::File::open(file)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&buf, &mut hasher);
        Ok(hasher.finish())
    }

    fs::create_dir_all(&dst)?;
    // copy files if not exists or changed
    for entry in fs::read_dir(&src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            // check content hash, skip if same, so didn't change timestamp of file, in case cargo rebuild
            let dts_file = dst.as_ref().join(entry.file_name());
            if dts_file.exists() {
                let src_hash = hash_file(entry.path())?;
                let dst_hash = hash_file(dts_file.clone())?;
                if src_hash == dst_hash {
                    continue;
                }
            }
            fs::copy(entry.path(), &dts_file)?;
            println!("copy {:?} to {:?}", entry.path(), &dts_file);
        }
    }

    // remove files that not exists in src
    for entry in fs::read_dir(dst)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_entry = src.as_ref().join(entry.file_name());
        if !src_entry.exists() {
            if ty.is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}
