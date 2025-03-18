
use std::{ffi::OsStr, sync::Arc};
use libfuse_fs::passthrough::newlogfs::LoggingFileSystem;
use tokio::signal;

use scorpio::daemon::daemon_main;
use scorpio::fuse::MegaFuse;
use scorpio::manager::{fetch::CheckHash, ScorpioManager};
use scorpio::server::mount_filesystem;
use scorpio::util::scorpio_config;

#[tokio::main]
async fn main() {
   
    println!("Hello, world!");
    let config_path = "config.toml";
    let mut manager = ScorpioManager::from_toml(config_path).unwrap();
    manager.check().await;
    let fuse_interface = MegaFuse::new_from_manager(&manager).await;
    let workspace = scorpio_config::get_config().get_value("workspace")
        .expect("Error: 'workspace' key is missing in the configuration.");
    let mountpoint =OsStr::new(workspace) ;
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
    }
}
