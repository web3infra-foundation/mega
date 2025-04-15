
use std::{ffi::OsStr, sync::Arc};
use libfuse_fs::passthrough::newlogfs::LoggingFileSystem;
use tokio::signal;
use clap::Parser;
use scorpio::daemon::daemon_main;
use scorpio::fuse::MegaFuse;
use scorpio::manager::{fetch::CheckHash, ScorpioManager};
use scorpio::server::mount_filesystem;
use scorpio::util::config;

/// Command line arguments for the application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "scorpio.toml")]
    config_path: String,
}

#[tokio::main]
async fn main() {
   
    println!(r#"
        ____   ___   __   ____  ____   __    __  
        / ___) / __) /  \ (  _ \(  _ \ (  )  /  \ 
        \___ \( (__ (  O ) )   / ) __/  )(  (  O )
        (____/ \___) \__/ (__\_)(__)   (__)  \__/ 
"#
);
    let args = Args::parse();

    if let Err(e) = config::init_config(&args.config_path) {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    }

    let mut manager = ScorpioManager::from_toml(config::config_file()).unwrap();
    manager.check().await;
    //init scorpio configuration

    let fuse_interface = MegaFuse::new_from_manager(&manager).await;
    let workspace = config::workspace();
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
