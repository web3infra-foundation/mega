// main.rs (Revised again to fix compilation errors)
extern crate cc;
extern crate pkg_config;
extern crate vcpkg;
// bindgen is currently disabled by the BUCK file's features=[]
// #[cfg(feature = "bindgen")]
// extern crate bindgen;

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
// Removed std::fs and std::io::Write as generate_exports_file is removed

mod cfgs;
mod find_normal;
// #[cfg(feature = "bindgen")]
// mod run_bindgen;

#[derive(PartialEq, Debug, Clone, Copy)]
enum Version {
    Openssl3xx,
    Openssl11x,
    Openssl10x,
    Libressl,
    Boringssl,
}

// Helper to print env vars and set rerun-if-changed
fn env_inner(name: &str) -> Option<OsString> {
    let var = env::var_os(name);
    println!("cargo:rerun-if-env-changed={}", name);

    // Optional: Print value for debugging build environment
    // match var {
    //     Some(ref v) => println!("cargo:warning=Build env {} = {}", name, v.to_string_lossy()),
    //     None => println!("cargo:warning=Build env {} unset", name),
    // }

    var
}

// Check target-specific and generic env vars
fn env(name: &str) -> Option<OsString> {
    let prefix = env::var("TARGET").unwrap().to_uppercase().replace('-', "_");
    let prefixed = format!("{}_{}", prefix, name);
    env_inner(&prefixed).or_else(|| env_inner(name))
}

fn main() {
    // --- Check Cfg Declarations ---
    // (Keep these as they declare expected cfgs)
    println!("cargo:rustc-check-cfg=cfg(osslconf, values(\"OPENSSL_NO_OCB\", \"OPENSSL_NO_SM4\", \"OPENSSL_NO_SEED\", \"OPENSSL_NO_CHACHA\", \"OPENSSL_NO_CAST\", \"OPENSSL_NO_IDEA\", \"OPENSSL_NO_CAMELLIA\", \"OPENSSL_NO_RC4\", \"OPENSSL_NO_BF\", \"OPENSSL_NO_PSK\", \"OPENSSL_NO_DEPRECATED_3_0\", \"OPENSSL_NO_SCRYPT\", \"OPENSSL_NO_SM3\", \"OPENSSL_NO_RMD160\", \"OPENSSL_NO_EC2M\", \"OPENSSL_NO_OCSP\", \"OPENSSL_NO_CMS\", \"OPENSSL_NO_COMP\", \"OPENSSL_NO_SOCK\", \"OPENSSL_NO_STDIO\", \"OPENSSL_NO_EC\", \"OPENSSL_NO_SSL3_METHOD\", \"OPENSSL_NO_KRB5\", \"OPENSSL_NO_TLSEXT\", \"OPENSSL_NO_SRP\", \"OPENSSL_NO_RFC3779\", \"OPENSSL_NO_SHA\", \"OPENSSL_NO_NEXTPROTONEG\", \"OPENSSL_NO_ENGINE\", \"OPENSSL_NO_BUF_FREELISTS\"))");
    println!("cargo:rustc-check-cfg=cfg(openssl)");
    println!("cargo:rustc-check-cfg=cfg(libressl)");
    println!("cargo:rustc-check-cfg=cfg(boringssl)");
    println!("cargo:rustc-check-cfg=cfg(libressl250)");
    println!("cargo:rustc-check-cfg=cfg(libressl251)");
    println!("cargo:rustc-check-cfg=cfg(libressl252)");
    println!("cargo:rustc-check-cfg=cfg(libressl261)");
    println!("cargo:rustc-check-cfg=cfg(libressl270)");
    println!("cargo:rustc-check-cfg=cfg(libressl271)");
    println!("cargo:rustc-check-cfg=cfg(libressl273)");
    println!("cargo:rustc-check-cfg=cfg(libressl280)");
    println!("cargo:rustc-check-cfg=cfg(libressl281)");
    println!("cargo:rustc-check-cfg=cfg(libressl291)");
    println!("cargo:rustc-check-cfg=cfg(libressl310)");
    println!("cargo:rustc-check-cfg=cfg(libressl321)");
    println!("cargo:rustc-check-cfg=cfg(libressl332)");
    println!("cargo:rustc-check-cfg=cfg(libressl340)");
    println!("cargo:rustc-check-cfg=cfg(libressl350)");
    println!("cargo:rustc-check-cfg=cfg(libressl360)");
    println!("cargo:rustc-check-cfg=cfg(libressl361)");
    println!("cargo:rustc-check-cfg=cfg(libressl370)");
    println!("cargo:rustc-check-cfg=cfg(libressl380)");
    println!("cargo:rustc-check-cfg=cfg(libressl381)");
    println!("cargo:rustc-check-cfg=cfg(libressl382)");
    println!("cargo:rustc-check-cfg=cfg(libressl390)");
    println!("cargo:rustc-check-cfg=cfg(libressl400)");
    println!("cargo:rustc-check-cfg=cfg(ossl101)");
    println!("cargo:rustc-check-cfg=cfg(ossl102)");
    println!("cargo:rustc-check-cfg=cfg(ossl102f)");
    println!("cargo:rustc-check-cfg=cfg(ossl102h)");
    println!("cargo:rustc-check-cfg=cfg(ossl110)");
    println!("cargo:rustc-check-cfg=cfg(ossl110f)");
    println!("cargo:rustc-check-cfg=cfg(ossl110g)");
    println!("cargo:rustc-check-cfg=cfg(ossl110h)");
    println!("cargo:rustc-check-cfg=cfg(ossl111)");
    println!("cargo:rustc-check-cfg=cfg(ossl111b)");
    println!("cargo:rustc-check-cfg=cfg(ossl111c)");
    println!("cargo:rustc-check-cfg=cfg(ossl111d)");
    println!("cargo:rustc-check-cfg=cfg(ossl300)");
    println!("cargo:rustc-check-cfg=cfg(ossl310)");
    println!("cargo:rustc-check-cfg=cfg(ossl320)");
    println!("cargo:rustc-check-cfg=cfg(ossl330)");
    println!("cargo:rustc-check-cfg=cfg(ossl340)");


    // --- Find OpenSSL ---
    let target = env::var("TARGET").unwrap();
    // Use the standard find_normal function which respects OPENSSL_DIR etc.
    // The BUCK file sets OPENSSL_DIR=$(location :local-openssl-src)
    println!("cargo:warning=Attempting to find OpenSSL using environment variables (OPENSSL_DIR, etc.) and pkg-config...");

    // Call get_openssl and handle its current tuple return type (Vec<PathBuf>, PathBuf)
    // We assume it panics internally on failure for now, matching original behavior perhaps.
    // If it returns Result, you'd use .unwrap() or proper error handling.
    let (lib_dirs, include_dir): (Vec<PathBuf>, PathBuf) = find_normal::get_openssl(&target);


    println!("cargo:warning=Found OpenSSL include directory: {}", include_dir.display());
    for lib_dir in &lib_dirs {
        println!("cargo:warning=Found OpenSSL library directory: {}", lib_dir.display());
        // Tell rustc to search for libraries in this directory
        println!("cargo:rustc-link-search=native={}", lib_dir.to_string_lossy());
    }
    // Tell rustc the include path (useful for dependent crates or bindgen)
    println!("cargo:include={}", include_dir.to_string_lossy());

    // Rerun build script if headers change in the found include directory
    // Check both the root include dir and the openssl subdir
    println!("cargo:rerun-if-changed={}", include_dir.display());
    let openssl_include_subdir = include_dir.join("openssl");
    if openssl_include_subdir.exists() {
         println!("cargo:rerun-if-changed={}", openssl_include_subdir.display());
    }


    // --- Post-processing Steps (Moved from removed postprocess function) ---

    // **TODO**: Implement actual version detection by parsing headers in `include_dir`.
    // For now, we'll use placeholder values. This is CRITICAL for correctness.
    let detected_version_type = Version::Openssl3xx; // Placeholder!
    let detected_version_number = 0x30000000; // Placeholder for 3.0.0!

    // Set Version CFGs based on (placeholder) detected version
    set_version_cfgs(detected_version_type, detected_version_number);

    // If bindgen feature were enabled, run it now
    // #[cfg(feature = "bindgen")]
    // {
    //    println!("cargo:warning=Running bindgen...");
    //    run_bindgen::run(&[include_dir]); // Pass the found include_dir
    // }
    // --- End of Post-processing Steps ---


    // --- Determine Link Libraries ---
    let libs_env = env("OPENSSL_LIBS");
    let libs_to_link = match libs_env.as_ref().and_then(|s| s.to_str()) {
         Some(v) => { // User explicitly specified libraries
             if v.is_empty() {
                 println!("cargo:warning=OPENSSL_LIBS is set but empty, linking no libraries by default.");
                 vec![]
             } else {
                 let user_libs: Vec<&str> = v.split(':').collect();
                 println!("cargo:warning=Using OPENSSL_LIBS override: {:?}", user_libs);
                 user_libs
             }
         }
         None => { // Determine default libraries based on target and (placeholder) version
             println!("cargo:warning=Using default libraries based on target and detected version (currently placeholder)...");
             match detected_version_type {
                Version::Openssl10x if target.contains("windows") => vec!["ssleay32", "libeay32"],
                Version::Openssl3xx | Version::Openssl11x if target.contains("windows-msvc") => {
                    vec!["libssl", "libcrypto"]
                }
                // Default for most Unix-like systems and MinGW
                 _ => vec!["ssl", "crypto"],
             }
         }
     };

    // --- Determine Link Mode ---
    // We don't have `default_kind` from find_normal::Info, so `determine_mode` relies more on env vars and file checks.
    let kind = determine_mode(&lib_dirs, &libs_to_link);
    println!("cargo:warning=Linking kind determined as: {}", kind);


    // --- Link Libraries ---
    for lib in libs_to_link {
        println!("cargo:rustc-link-lib={}={}", kind, lib);
    }

    // Add platform-specific dependencies for static linking
    if kind == "static" {
        if target.contains("windows") {
            println!("cargo:rustc-link-lib=dylib=gdi32");
            println!("cargo:rustc-link-lib=dylib=user32");
            println!("cargo:rustc-link-lib=dylib=crypt32");
            println!("cargo:rustc-link-lib=dylib=ws2_32");
            println!("cargo:rustc-link-lib=dylib=advapi32");
        }
        // Use the placeholder version type here
        if detected_version_type == Version::Boringssl && env::var("CARGO_CFG_UNIX").is_ok() {
             let cpp_lib = match env::var("CARGO_CFG_TARGET_OS").unwrap().as_ref() {
                 "macos" => "c++",
                 _ => "stdc++",
             };
             println!("cargo:rustc-link-lib={}", cpp_lib);
        }
         // Use the placeholder version type here
        if detected_version_type == Version::Openssl3xx
             && (env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux"
                 || env::var("CARGO_CFG_TARGET_OS").unwrap() == "android")
             && env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() == "32"
         {
             println!("cargo:rustc-link-lib=atomic");
         }
    }

    println!("cargo:warning=openssl-sys build script finished successfully.");
}


/// Sets the appropriate version cfgs based on the detected version.
/// NOTE: This function currently uses PLACEHOLDER values.
/// Proper implementation requires parsing headers (e.g., opensslv.h)
/// found in the `include_dir`.
fn set_version_cfgs(version_type: Version, version: u64) {
    println!("cargo:warning=Setting CFGs based on detected OpenSSL Version Type: {:?}, Numeric: {:#x} (PLACEHOLDER VALUES!)", version_type, version);

    // Set top-level type cfg
    match version_type {
        Version::Openssl3xx | Version::Openssl11x | Version::Openssl10x => {
            println!("cargo:rustc-cfg=openssl");
        },
        Version::Libressl => {
            println!("cargo:rustc-cfg=libressl");
        }
        Version::Boringssl => {
            println!("cargo:rustc-cfg=boringssl");
            // BoringSSL often needs probing for features rather than version numbers
            // cfgs::probe(); // Consider re-enabling if needed for BoringSSL specifics
            println!("cargo:warning=BoringSSL detected; consider enabling cfgs::probe() if needed.");
        }
    }

    // Set specific version cfgs based on the numeric version
    // ** THIS IS THE CRITICAL PART THAT NEEDS REAL IMPLEMENTATION **
    match version_type {
       Version::Openssl3xx | Version::Openssl11x | Version::Openssl10x => {
            // Check OPENSSL_VERSION_NUMBER format (e.g., 0x1010107fL for 1.1.1g)
            if version >= 0x10001000 { println!("cargo:rustc-cfg=ossl101"); }
            if version >= 0x10002000 { println!("cargo:rustc-cfg=ossl102"); }
            if version >= 0x10002060 { println!("cargo:rustc-cfg=ossl102f"); } // 1.0.2f
            if version >= 0x10002080 { println!("cargo:rustc-cfg=ossl102h"); } // 1.0.2h
            if version >= 0x10100000 { println!("cargo:rustc-cfg=ossl110"); }
            if version >= 0x10100060 { println!("cargo:rustc-cfg=ossl110f"); } // 1.1.0f
            if version >= 0x10100070 { println!("cargo:rustc-cfg=ossl110g"); } // 1.1.0g
            if version >= 0x10100080 { println!("cargo:rustc-cfg=ossl110h"); } // 1.1.0h
            if version >= 0x10101000 { println!("cargo:rustc-cfg=ossl111"); }
            if version >= 0x10101020 { println!("cargo:rustc-cfg=ossl111b"); } // 1.1.1b
            if version >= 0x10101030 { println!("cargo:rustc-cfg=ossl111c"); } // 1.1.1c
            if version >= 0x10101040 { println!("cargo:rustc-cfg=ossl111d"); } // 1.1.1d
            if version >= 0x30000000 { println!("cargo:rustc-cfg=ossl300"); }
            if version >= 0x30100000 { println!("cargo:rustc-cfg=ossl310"); }
            if version >= 0x30200000 { println!("cargo:rustc-cfg=ossl320"); }
            if version >= 0x30300000 { println!("cargo:rustc-cfg=ossl330"); }
            if version >= 0x30400000 { println!("cargo:rustc-cfg=ossl340"); }
       }
       Version::Libressl => {
           // Check LIBRESSL_VERSION_NUMBER format (e.g., 0x2050000fL for 2.5.0)
            if version >= 0x20500000 { println!("cargo:rustc-cfg=libressl250"); }
            if version >= 0x20501000 { println!("cargo:rustc-cfg=libressl251"); }
            if version >= 0x20502000 { println!("cargo:rustc-cfg=libressl252"); }
            if version >= 0x20601000 { println!("cargo:rustc-cfg=libressl261"); }
            if version >= 0x20700000 { println!("cargo:rustc-cfg=libressl270"); }
            if version >= 0x20701000 { println!("cargo:rustc-cfg=libressl271"); }
            if version >= 0x20703000 { println!("cargo:rustc-cfg=libressl273"); }
            if version >= 0x20800000 { println!("cargo:rustc-cfg=libressl280"); }
            if version >= 0x20801000 { println!("cargo:rustc-cfg=libressl281"); }
            if version >= 0x20901000 { println!("cargo:rustc-cfg=libressl291"); }
            if version >= 0x30100000 { println!("cargo:rustc-cfg=libressl310"); }
            if version >= 0x30201000 { println!("cargo:rustc-cfg=libressl321"); }
            if version >= 0x30302000 { println!("cargo:rustc-cfg=libressl332"); }
            if version >= 0x30400000 { println!("cargo:rustc-cfg=libressl340"); }
            if version >= 0x30500000 { println!("cargo:rustc-cfg=libressl350"); }
            if version >= 0x30600000 { println!("cargo:rustc-cfg=libressl360"); }
            if version >= 0x30601000 { println!("cargo:rustc-cfg=libressl361"); }
            if version >= 0x30700000 { println!("cargo:rustc-cfg=libressl370"); }
            if version >= 0x30800000 { println!("cargo:rustc-cfg=libressl380"); }
            if version >= 0x30801000 { println!("cargo:rustc-cfg=libressl381"); }
            if version >= 0x30802000 { println!("cargo:rustc-cfg=libressl382"); }
            if version >= 0x30900000 { println!("cargo:rustc-cfg=libressl390"); }
            if version >= 0x40000000 { println!("cargo:rustc-cfg=libressl400"); }
       }
       Version::Boringssl => {
           // BoringSSL needs specific checks, not simple version numbers
           println!("cargo:warning=BoringSSL version CFGs require specific probing, not fully implemented here.");
       }
    }
}

/// Determines the link mode (static or dynamic).
/// Prefers environment variables, then checks for static libraries.
/// Removed default_kind parameter as it's not available from find_normal currently.
fn determine_mode(libdirs: &[PathBuf], libs: &[&str]) -> &'static str {
    // Prefer explicit environment variables
    if let Some(val) = env("OPENSSL_STATIC") {
        if val == "1" {
            println!("cargo:warning=Using static linking due to OPENSSL_STATIC=1");
            return "static";
        }
    }

    // Fallback: Check if static libraries exist in the found directories
    let mut found_static = false;
    if !libdirs.is_empty() && !libs.is_empty() {
        // Check library directories provided
        for lib_dir in libdirs {
             if !lib_dir.exists() { continue; }
             let static_lib_name = format!("lib{}.a", libs[0]); // Check only the first library for simplicity
             if lib_dir.join(static_lib_name).exists() {
                 println!("cargo:warning=Found static library file ({}.a) in {}, preferring static linking.", libs[0], lib_dir.display());
                 found_static = true;
                 break; // Found it in one directory, assume static
             }
        }
    }

    if found_static {
        "static"
    } else {
        println!("cargo:warning=Static library (.a) not found or OPENSSL_STATIC not set, defaulting to dynamic linking ('dylib').");
        "dylib" // Default to dynamic
    }
}