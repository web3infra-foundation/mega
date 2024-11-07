
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

use scorpio::{deamon::deamon_main, fuse::MegaFuse, manager::{fetch::CheckHash, ScorpioManager}, server::mount_filesystem};
use tokio::signal;



#[tokio::main]
async fn main() {
   
    println!("Hello, world!");
    let config_path = "config.toml";
    let mut manager = ScorpioManager::from_toml(config_path).unwrap();
    manager.check().await;
    let fuse_interface = MegaFuse::new_from_manager(&manager).await;

    let mountpoint =OsStr::new("/home/luxian/dic") ;
    let mut mount_handle =  mount_filesystem(fuse_interface.clone(), mountpoint).await;
    let handle = &mut mount_handle;

    // 在tokio运行时中执行deamon_main函数
   
    deamon_main(Arc::new(fuse_interface),manager).await;
   
    tokio::select! {
        res = handle => res.unwrap(),
        _ = signal::ctrl_c() => {
            mount_handle.unmount().await.unwrap()
        }
    };
    // init_runtime( Handle::current() ); 
    // println!("Hello, world!");
    // let config_path = "config.toml";
    // let mut manager = ScorpioManager::from_toml(config_path).unwrap();
    // manager.check().await;
    // let fuse_interface = MegaFuse::new_from_manager(&manager);

    // //run(fuse_interface.clone(), &manager.mount_path)
    // let mut se = FuseSession::new(Path::new(&manager.mount_path), "dic", "", false).unwrap();
    // se.mount().unwrap();
    // let ch = se.new_channel().unwrap();
    // let server = Arc::new(Server::new(fuse_interface.clone()));
    // let mut fuse_server = FuseServer { server, ch };
    // // Spawn server thread
    // let handle = tokio::task::spawn_blocking( move || {
    //     fuse_server.svc_loop()
    // });
    
    // // 在tokio运行时中执行deamon_main函数
   
    // deamon_main(fuse_interface,manager).await;
   
    // // Wait for termination signal
    // let mut signals = Signals::new(TERM_SIGNALS).unwrap();
    // println!("Signals start");
    // if let Some(_sig) = signals.forever().next() {
    //     //pass
    // }
    // //  Unmount and wake up
    // se.umount().unwrap();
    // se.wake().unwrap();
    // // Join server thread
    // let _ = handle.await;
}
