
// use fuse_backend_rs::api::server::Server;
// use fuse_backend_rs::transport::FuseSession;
// use scorpio::init_runtime;
// use scorpio::manager::ScorpioManager;
// use scorpio::fuse::MegaFuse;
// use scorpio::server::FuseServer;
// use scorpio::manager::fetch::CheckHash;
// use signal_hook::consts::TERM_SIGNALS;
// use signal_hook::iterator::Signals; 
// use scorpio::deamon::deamon_main;
// use tokio::runtime::Handle;


use std::{ffi::OsStr, sync::Arc};

use scorpio::daemon::daemon_main;
use scorpio::fuse::MegaFuse;
use scorpio::manager::{fetch::CheckHash, ScorpioManager};
use scorpio::server::mount_filesystem;
use tokio::signal;
use scorpio::passthrough::newlogfs::LoggingFileSystem;

#[tokio::main]
async fn main() {
   
    println!("Hello, world!");
    let config_path = "config.toml";
    let mut manager = ScorpioManager::from_toml(config_path).unwrap();
    manager.check().await;
    let fuse_interface = MegaFuse::new_from_manager(&manager).await;
    let mountpoint =OsStr::new(&manager.workspace) ;
    let lgfs = LoggingFileSystem::new(fuse_interface.clone());
    let mut mount_handle =  mount_filesystem(lgfs, mountpoint).await;
    let handle = &mut mount_handle;


    // spawn the server running function. 
    tokio::spawn(daemon_main(Arc::new(fuse_interface),manager));

    print!("server running...");
    tokio::select! {
        res = handle => res.unwrap(),
        _ = signal::ctrl_c() => {
            
            println!("unmount....");
            mount_handle.unmount().await.unwrap();
            
        }
    };
}
