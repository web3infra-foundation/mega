// use std::{path::Path, sync::Arc, thread::JoinHandle};

// use fuse_backend_rs::{api::{filesystem::FileSystem, server::Server}, transport::{FuseChannel, FuseSession}};
// #[allow(unused)]
// pub struct FuseServer<T: FileSystem + Send + Sync> {
//     pub server: Arc<Server<T>>,
//     pub ch: FuseChannel,
// }
// pub fn run<T: FileSystem + Send + Sync+ 'static>(fuse:Arc<T>,path:&str )->JoinHandle<Result<(), std::io::Error>>{
//     let mut se = FuseSession::new(Path::new(path), "dic", "", false).unwrap();
//     se.mount().unwrap();
//     let ch: FuseChannel = se.new_channel().unwrap();
//     let server = Arc::new(Server::new(fuse));
//     let mut fuse_server = FuseServer { server, ch };
//     // Spawn server thread
//     std::thread::spawn( move || {
//         fuse_server.svc_loop()
//     })

// }
// #[allow(unused)]
// impl <FS:FileSystem+ Send + Sync>FuseServer<FS> {
//     pub fn svc_loop(&mut self) -> Result<(), std::io::Error> {
//         let _ebadf = std::io::Error::from_raw_os_error(libc::EBADF);
//         println!("entering server loop");
//         loop {
//             if let Some((reader, writer)) = self
//                 .ch
//                 .get_request()
//                 .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?
//             {
//                 if let Err(e) = self
//                     .server
//                     .handle_message(reader, writer.into(), None, None)
//                 {
//                     match e {
//                         fuse_backend_rs::Error::EncodeMessage(_ebadf) => {
//                             break;
//                         }
//                         _ => {
//                             print!("Handling fuse message failed");
//                             continue;
//                         }
//                     }
//                 }
//             } else {
//                 print!("fuse server exits");
//                 break;
//             }
//         }
//         Ok(())
//     }
// }

use std::ffi::{OsStr, OsString};

use rfuse3::{
    raw::{Filesystem, MountHandle, Session},
    MountOptions,
};

#[cfg(target_os = "linux")]
// TODO(perf): tune these values with production lookup metrics and make them
// configurable per mount role (interactive vs build-heavy workers).
const ANTARES_FUSE_CACHE_MOUNT_OPTIONS: &str =
    "kernel_cache,auto_cache,entry_timeout=60,attr_timeout=60,negative_timeout=10";

#[cfg(not(target_os = "linux"))]
const ANTARES_FUSE_CACHE_MOUNT_OPTIONS: &str = "";

fn apply_antares_cache_mount_options(options: &mut MountOptions) {
    if !ANTARES_FUSE_CACHE_MOUNT_OPTIONS.is_empty() {
        options.custom_options(ANTARES_FUSE_CACHE_MOUNT_OPTIONS);
    }
}

#[allow(unused)]
pub async fn mount_filesystem<F: Filesystem + std::marker::Sync + Send + 'static>(
    fs: F,
    mountpoint: &OsStr,
) -> MountHandle {
    mount_filesystem_with_antares_cache(fs, mountpoint, false).await
}

#[allow(unused)]
pub async fn mount_filesystem_with_antares_cache<
    F: Filesystem + std::marker::Sync + Send + 'static,
>(
    fs: F,
    mountpoint: &OsStr,
    enable_antares_cache: bool,
) -> MountHandle {
    if let Err(e) = env_logger::try_init() {
        if !e.to_string().contains("initialized") {
            eprintln!("Failed to initialize logger: {}", e);
        }
    }
    //let logfs = LoggingFileSystem::new(fs);

    let mount_path: OsString = OsString::from(mountpoint);
    let path = std::path::Path::new(&mount_path);
    if !path.exists() {
        if let Err(e) = std::fs::create_dir_all(path) {
            panic!("failed to create mountpoint: {}", e);
        }
    }
    if !path.exists() {
        panic!("mountpoint does not exist");
    }
    if !path.is_dir() {
        panic!("mountpoint is not a directory");
    }
    let has_entries = std::fs::read_dir(path)
        .map(|mut it| it.next().is_some())
        .unwrap_or(true);
    if has_entries {
        panic!("mountpoint is not empty or is inaccessible");
    }
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    let mut mount_options = MountOptions::default();
    // .allow_other(true)
    mount_options.force_readdir_plus(true).uid(uid).gid(gid);
    if enable_antares_cache {
        apply_antares_cache_mount_options(&mut mount_options);
    }

    let session = Session::<F>::new(mount_options);
    session.mount(fs, mount_path).await.unwrap()
}
