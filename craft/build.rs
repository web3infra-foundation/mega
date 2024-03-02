use std::{env,fs,self};
use std::path::{PathBuf,Path};
use std::fmt::{Display, Formatter};
use std::io::Error as IoError;
use std::process::Command;
use std::sync::Arc;

trait ErrorTransform<T> {
    fn transform(self) -> T;
}
#[derive(Debug)]
enum BuildError {
    MsgFmtError(String,String,u32,Arc<dyn std::error::Error>),
    IoError(String,String,u32,Arc<dyn std::error::Error>),
    PoFileError(String,String,u32,Arc<dyn std::error::Error>),
    CreatMoFileError(String,String,u32,Arc<dyn std::error::Error>),
    MsgFmtProcessError(String,String,u32,Arc<dyn std::error::Error>)
}
impl Display for BuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::MsgFmtError(function,file,line,e) =>
                write!(f,"MsgFmtError in {} at {}:{} - {}", function, file, line, e),
            BuildError::IoError(function,file,line,e) =>
                write!(f,"IO error in {} at {}:{} - {}", function, file, line, e),
            BuildError::PoFileError(function,file,line,e) =>
                write!(f,".Po file error in {} at {}:{} - {}", function, file, line, e),
            BuildError::CreatMoFileError(function,file,line,e) =>
                write!(f,".Creat .Mo File directory error in {} at {}:{} - {}", function, file, line, e),
            BuildError::MsgFmtProcessError(function,file,line,e) =>
                write!(f,"MsgFmt process error in {} at {}:{} - {}", function, file, line, e)
        }
    }
}
#[macro_export]
macro_rules! create_error {
    ($kind:ident, $msg:expr) => {
        BuildError::$kind(
            format!("{}", std::any::type_name::<$kind>()),
            file!().to_string(),
            line!(),
            Arc::new($msg)
        )
    };
    ($kind:ident, $msg:expr, $func:expr) => {
        BuildError::$kind(
            $func.to_string(),
            file!().to_string(),
            line!(),
            Arc::new($msg)
        )
    };
    ($kind:ident) => {
        BuildError::$kind(
            "".to_string(),
            file!().to_string(),
            line!(),
            Arc::new(std::fmt::Error::default())
        )
    };
}
impl ErrorTransform<BuildError> for IoError {
    fn transform(self) -> BuildError {
        create_error!(IoError,self)
    }
}
impl From<IoError> for BuildError {
    fn from(error: IoError) -> Self {
        error.transform()
    }
}
#[cfg(target_os = "windows")]
fn vcpkg_init() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_triplet = match target_arch.as_str() {
        "x86_64" => "x64-windows-static",
        "x86" => "x86-windows-static",
        _ => panic!("Unsupported target architecture: {}", target_arch),
    };
    let library = vcpkg::Config::new()
        .emit_includes(true)
        .target_triplet(target_triplet)
        .find_package("libiconv");
    match library {
        Ok(lib) => {
            for path in lib.include_paths {
                println!("cargo:include={}", path.display());
            }
            println!("cargo:rustc-link-lib=static=iconv");
            for path in lib.link_paths {
                println!("cargo:rustc-link-search=native={}", path.display());
            }
        }
        Err(e) => {
            println!("cargo:warning=Could not find libiconv from vcpkg: {}", e);
        }
    }
}
fn main() -> Result<(),BuildError>{
    let target = env::var("TARGET").unwrap_or_else(|_|{
        if cfg!(target_os = "windows") {
            "windows".to_string()
        } else if cfg!(target_os = "linux"){
            "linux".to_string()
        } else if cfg!(target_os = "macos") {
            "macos".to_string()
        } else {
            panic!("Unsupported target OS");
        }
    });
    let target_override = env::var("BUILD_TARGET").ok();
    let effective_target = target_override.as_ref().unwrap_or(&target);

    println!("Configuring build for target: {}",effective_target);

    if effective_target.contains("windows" ){
        println!("cargo:rustc-cfg=target_os=\"windows\"");
        #[cfg(target_os = "windows")]
        vcpkg_init();
    } else if effective_target.contains("linux") {
        println!("cargo:rustc-cfg=target_os=\"linux\"");
    } else if effective_target.contains("darwin") {
        println!("cargo:rustc-cfg=target_os=\"macos\"");
        cc::Build::new()
            .file("src/lfs/commands/c_utils/memory_info.c")
            .compile("memory_info")
    }

    let translations_dir = Path::new("./src/lfs/errors/translations");
    let po_files = find_po_files(translations_dir)?;
    for po_file in po_files {
        let file_name = po_file.file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| create_error!(PoFileError))?;
        let pattern = regex::Regex::new(r"^[a-zA-Z]+_[a-zA-Z]+$").unwrap();
        if !pattern.is_match(file_name) {
            continue;
        }
        let parts: Vec<&str> = file_name.split('_').collect();
        if parts.len() != 2 {
            return Err(create_error!(PoFileError));
        }
        let domain = parts[0];
        let language_code = parts[1];

        let output_dir = Path::new("target")
            .join("translations")
            .join(language_code)
            .join("LC_MESSAGES");
        fs::create_dir_all(&output_dir).map_err(|e| create_error!(CreatMoFileError, e, "build_main"))?;
        let mo_file_path = output_dir.join(format!("{}.mo", domain));
        let status = Command::new("msgfmt")
            .arg(&po_file)
            .arg("-o")
            .arg(&mo_file_path)
            .status()
            .map_err(|e| create_error!(MsgFmtProcessError, e, "build_main"))?;
        if !status.success() {
            return Err(create_error!(MsgFmtError));
        }
    }

    Ok(())

}

fn find_po_files(dir:&Path) -> Result<Vec<PathBuf>,BuildError> {
    let mut po_files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| create_error!(IoError,e,"find_po_files"))? {
            let entry = entry.map_err(|e| create_error!(IoError,e,"find_po_files"))?;
            let path = entry.path();
            if path.is_dir() {
                po_files.extend(find_po_files(&path)?);
            } else if path.extension().and_then(|s| s.to_str()) == Some("po") {
                po_files.push(path);
            }
        }
    }
    Ok(po_files)
}