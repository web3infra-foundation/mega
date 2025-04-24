// find_normal.rs (Fixed E0252 env import conflict)
use std::env as std_env; // Use std_env to avoid conflict with super::env
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

// Import the custom environment variable reading function from main.rs/build.rs
use super::env;

pub fn get_openssl(target: &str) -> (Vec<PathBuf>, PathBuf) {
    // Use the custom `env` function (super::env) to read OPENSSL_* variables
    let lib_dir = env("OPENSSL_LIB_DIR").map(PathBuf::from);
    let include_dir = env("OPENSSL_INCLUDE_DIR").map(PathBuf::from);

    match (lib_dir, include_dir) {
        // If both LIB_DIR and INCLUDE_DIR are set, use them directly
        (Some(lib_dir), Some(include_dir)) => (vec![lib_dir], include_dir),
        // Otherwise, try to use OPENSSL_DIR or find it
        (lib_dir, include_dir) => {
            // Use the custom `env` function (super::env) here too
            let openssl_dir = env("OPENSSL_DIR").unwrap_or_else(|| find_openssl_dir(target));
            let openssl_dir = Path::new(&openssl_dir);
            let lib_dir = lib_dir.map(|d| vec![d]).unwrap_or_else(|| {
                // If LIB_DIR wasn't set, derive it from OPENSSL_DIR
                let mut lib_dirs = vec![];
                // OpenSSL 3.0 now puts it's libraries in lib64/ by default,
                // check for both it and lib/.
                if openssl_dir.join("lib64").exists() {
                    lib_dirs.push(openssl_dir.join("lib64"));
                }
                if openssl_dir.join("lib").exists() {
                    lib_dirs.push(openssl_dir.join("lib"));
                }
                if lib_dirs.is_empty() {
                    // Fallback if neither lib nor lib64 exist (e.g., vcpkg layout)
                     println!("cargo:warning=Could not find lib/ or lib64/ in OPENSSL_DIR, using OPENSSL_DIR itself as library path.");
                     lib_dirs.push(openssl_dir.to_path_buf());
                }
                lib_dirs
            });
            let include_dir = include_dir.unwrap_or_else(|| {
                // If INCLUDE_DIR wasn't set, derive it from OPENSSL_DIR
                let inc = openssl_dir.join("include");
                if !inc.exists() {
                     println!("cargo:warning=Could not find include/ in OPENSSL_DIR, using OPENSSL_DIR itself as include path.");
                     openssl_dir.to_path_buf()
                } else {
                     inc
                }
            });
            (lib_dir, include_dir)
        }
    }
}

// Helper function to find openssl install directory using various methods
fn find_openssl_dir(target: &str) -> OsString {
    // Use std_env here for HOST variable
    let host = std_env::var("HOST").unwrap();

    // Check common locations for specific OSes first
    if host == target && target.ends_with("-apple-darwin") {
        let homebrew_dir = match target {
            "aarch64-apple-darwin" => "/opt/homebrew",
            _ => "/usr/local",
        };

        if let Some(dir) = resolve_with_wellknown_homebrew_location(homebrew_dir) {
            return dir.into();
        } else if let Some(dir) = resolve_with_wellknown_location("/opt/pkg") {
            // pkgsrc (e.g., NetBSD)
            return dir.into();
        } else if let Some(dir) = resolve_with_wellknown_location("/opt/local") {
            // MacPorts
            return dir.into();
        }
    }

    // Try standard package managers if specific locations fail or aren't applicable
    try_pkg_config(); // Exits on success
    try_vcpkg(); // Exits on success

    // Check system paths for BSDs where pkg-config might not be set up for base OpenSSL/LibreSSL
    if host == target && (target.contains("freebsd") || target.contains("openbsd")) {
        println!("cargo:warning=Falling back to /usr for BSD system OpenSSL/LibreSSL.");
        return OsString::from("/usr");
    }

    // Check /usr/local for DragonFlyBSD ports
    if host == target && target.contains("dragonfly") {
        println!("cargo:warning=Falling back to /usr/local for DragonFlyBSD ports OpenSSL/LibreSSL.");
        return OsString::from("/usr/local");
    }

    // If we reach here, we haven't found OpenSSL. Print detailed error and exit.
    let msg_header =
        "Could not find directory of OpenSSL installation, and this `-sys` crate cannot
proceed without this knowledge. If OpenSSL is installed and this crate had
trouble finding it,  you can set the `OPENSSL_DIR` environment variable for the
compilation process.";

    println!(
        "cargo:warning={} See stderr section below for further information.",
        msg_header.replace('\n', " ")
    );

    let mut msg = format!(
        "

{}

Make sure you also have the development packages of openssl installed.
For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

If you're in a situation where you think the directory *should* be found
automatically, please open a bug at https://github.com/sfackler/rust-openssl
and include information about your system as well as this message.

$HOST = {}
$TARGET = {}
openssl-sys = {}

",
        msg_header,
        host, // Use host variable read using std_env
        target,
        std_env!("CARGO_PKG_VERSION") // Use std_env macro here too
    );

    // Add OS-specific hints
    if host.contains("apple-darwin") && target.contains("apple-darwin") {
        let system = Path::new("/usr/lib/libssl.0.9.8.dylib"); // Check for very old system lib
        if system.exists() {
            msg.push_str(
                "
WARNING: Found system copy of libssl.0.9.8.dylib, which is deprecated and unsupported.

openssl-sys crate build failed: no supported version of OpenSSL found.

Common ways to fix this:
- Use the `vendored` feature of the `openssl` crate to build OpenSSL from source.
- Use Homebrew (`brew install openssl@1.1` or `brew install openssl@3`) and potentially set OPENSSL_DIR.
- Use MacPorts (`sudo port install openssl3`) and potentially set OPENSSL_DIR.

",
            );
        } else {
             msg.push_str(
                "
openssl-sys crate build failed: Failed to find a supported OpenSSL installation.

Common ways to fix this:
- Use the `vendored` feature of the `openssl` crate to build OpenSSL from source.
- Use Homebrew (`brew install openssl@1.1` or `brew install openssl@3`) and potentially set OPENSSL_DIR.
- Use MacPorts (`sudo port install openssl3`) and potentially set OPENSSL_DIR.

",
            );
        }
    }

    if host.contains("unknown-linux")
        && target.contains("unknown-linux-gnu")
        && Command::new("pkg-config").output().is_err()
    {
        msg.push_str(
            "
Hint: It looks like you're compiling on Linux and also targeting Linux. Currently this
requires the `pkg-config` utility to find OpenSSL but unfortunately `pkg-config`
could not be found. If you have OpenSSL installed you can likely fix this by
installing `pkg-config`.

Debian/Ubuntu: sudo apt-get install pkg-config libssl-dev
Fedora/CentOS/RHEL: sudo yum install pkgconf-pkg-config openssl-devel
Arch: sudo pacman -S pkgconf openssl

",
        );
    }

    if host.contains("windows") && target.contains("windows-gnu") {
        // MSYS2/MinGW
        msg.push_str(
            "
Hint: It looks like you're compiling for MinGW (windows-gnu target).
Ensure OpenSSL development files and pkg-config are installed in your MSYS2 environment.

pacman -S mingw-w64-x86_64-openssl mingw-w64-x86_64-pkg-config

Or for 32-bit MinGW:
pacman -S mingw-w64-i686-openssl mingw-w64-i686-pkg-config

Then try building this crate again within the MinGW terminal.

",
        );
    }

    if host.contains("windows") && target.contains("windows-msvc") {
        msg.push_str(
            "
Hint: It looks like you're compiling for MSVC (windows-msvc target).
We couldn't detect an OpenSSL installation. You might need to:
- Use the `vendored` feature of the `openssl` crate.
- Install OpenSSL using vcpkg:
    1. Clone vcpkg: git clone https://github.com/Microsoft/vcpkg.git
    2. Bootstrap: .\\vcpkg\\bootstrap-vcpkg.bat
    3. Integrate: .\\vcpkg\\vcpkg integrate install
    4. Install OpenSSL: .\\vcpkg\\vcpkg install openssl --triplet x64-windows (or x86-windows)
- Download precompiled binaries (less recommended for development) and set OPENSSL_DIR.
  See: https://github.com/sfackler/rust-openssl#windows

",
        );
    }

    eprintln!("{}", msg);
    std::process::exit(101); // Use a distinct exit code
}

// Helper to check for Homebrew OpenSSL installations (common on macOS)
fn resolve_with_wellknown_homebrew_location(dir: &str) -> Option<PathBuf> {
    // Check common Homebrew prefixes
    let versions = ["openssl@3", "openssl@3.0", "openssl@1.1"];

    // Check default aarch64/x86_64 Homebrew installation location first
    for version in &versions {
        let homebrew_opt = Path::new(dir).join("opt").join(version);
        if homebrew_opt.join("include/openssl/opensslv.h").exists() {
            println!("cargo:warning=Found Homebrew OpenSSL at {}", homebrew_opt.display());
            return Some(homebrew_opt);
        }
    }

    // Last resort: Call `brew --prefix` (can be slow)
    println!("cargo:warning=Trying 'brew --prefix' as a fallback to find OpenSSL...");
    for version in &versions {
        let output = execute_command_and_get_output("brew", &["--prefix", version]);
        if let Some(ref output_str) = output {
            let homebrew_prefix = PathBuf::from(output_str.trim());
            if homebrew_prefix.join("include/openssl/opensslv.h").exists() {
                 println!("cargo:warning=Found Homebrew OpenSSL via 'brew --prefix' at {}", homebrew_prefix.display());
                return Some(homebrew_prefix);
            }
        }
    }

    None
}

// Helper to check if a directory looks like an OpenSSL installation root
fn resolve_with_wellknown_location(dir: &str) -> Option<PathBuf> {
    let root_dir = Path::new(dir);
    let include_openssl = root_dir.join("include/openssl/opensslv.h");
    if include_openssl.exists() {
         println!("cargo:warning=Found potential OpenSSL installation at {}", root_dir.display());
        Some(root_dir.to_path_buf())
    } else {
        None
    }
}


/// Attempt to find OpenSSL through pkg-config. Exits successfully if found.
fn try_pkg_config() {
    // Use std_env here for TARGET and HOST
    let target = std_env::var("TARGET").unwrap();
    let host = std_env::var("HOST").unwrap();

    // Allow cross-compile detection for MinGW
    if target.contains("windows-gnu") && host.contains("windows") {
        println!("cargo:warning=Setting PKG_CONFIG_ALLOW_CROSS=1 for MinGW build.");
        // Use std_env here
        std_env::set_var("PKG_CONFIG_ALLOW_CROSS", "1");
    } else if target.contains("windows-msvc") {
        // Don't use pkg-config for MSVC, prefer vcpkg
        return;
    }

    println!("cargo:warning=Attempting to find OpenSSL via pkg-config...");
    let lib = match pkg_config::Config::new()
        .print_system_libs(false) // Don't print -l flags, we'll do that later
        .probe("openssl")
    {
        Ok(lib) => lib,
        Err(e) => {
            // It's expected that pkg-config might fail, so just print a note.
            println!("cargo:warning=Could not find OpenSSL via pkg-config: {}", e);
            return; // Don't exit, let other methods try
        }
    };

    println!("cargo:warning=Found OpenSSL via pkg-config.");
    // If pkg-config succeeds, we have all the info needed.
    // Output the necessary cargo directives and exit.
    for include in lib.include_paths.iter() {
        println!("cargo:include={}", include.display());
         println!("cargo:rerun-if-changed={}", include.display());
         let openssl_subdir = include.join("openssl");
         if openssl_subdir.exists() {
             println!("cargo:rerun-if-changed={}", openssl_subdir.display());
         }
    }
    for lib_path in lib.link_paths.iter() {
        println!("cargo:rustc-link-search=native={}", lib_path.display());
    }
    for library in lib.libs.iter() {
        // pkg-config gives library names without "lib" prefix or ".a"/".so" suffix
        println!("cargo:rustc-link-lib={}", library);
    }

    // We found it via pkg-config, no need to continue searching.
    process::exit(0);
}

/// Attempt to find OpenSSL through vcpkg. Exits successfully if found.
fn try_vcpkg() {
    // Use std_env here for TARGET
    let target = std_env::var("TARGET").unwrap();
    // vcpkg is primarily for Windows MSVC targets
    if !target.contains("windows-msvc") {
        return;
    }

    println!("cargo:warning=Attempting to find OpenSSL via vcpkg...");
    // vcpkg will automatically emit cargo metadata if it finds the package
    match vcpkg::Config::new().emit_includes(true).find_package("openssl") {
        Ok(_lib) => {
            // vcpkg::find_package emits the cargo metadata automatically
            println!("cargo:warning=Found OpenSSL via vcpkg.");

            // Vcpkg handles static/dynamic linking implicitly based on triplet.
            // We still need to add Windows system libs often needed when linking OpenSSL statically.
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=crypt32");
            println!("cargo:rustc-link-lib=advapi32"); // Added
            println!("cargo:rustc-link-lib=ws2_32");   // Added

            // We found it via vcpkg, no need to continue searching.
            process::exit(0);
        }
        Err(e) => {
            println!("cargo:warning=Could not find OpenSSL via vcpkg: {}", e);
            return; // Don't exit, let other methods try
        }
    };
}

// Helper to run a command and capture its stdout
fn execute_command_and_get_output(cmd: &str, args: &[&str]) -> Option<String> {
    match Command::new(cmd).args(args).output() {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
        },
        Ok(output) => {
             println!("cargo:warning=Command '{} {}' failed with status: {} stderr: {}",
                 cmd,
                 args.join(" "),
                 output.status,
                 String::from_utf8_lossy(&output.stderr));
             None
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute command '{} {}': {}", cmd, args.join(" "), e);
            None
        }
    }
}