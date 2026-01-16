use std::{ffi::OsStr, net::SocketAddr, sync::Arc};

use clap::Parser;
use libfuse_fs::passthrough::newlogfs::LoggingFileSystem;
use scorpio::{
    daemon::daemon_main,
    fuse::MegaFuse,
    manager::{fetch::CheckHash, ScorpioManager},
    server::mount_filesystem,
    util::config,
};
#[cfg(not(unix))]
use tokio::signal;
use tokio::sync::oneshot;

/// Command line arguments for the application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "scorpio.toml")]
    config_path: String,

    /// HTTP bind address for the daemon (Antares API lives under /antares/*)
    #[arg(long, default_value = "0.0.0.0:2725")]
    http_addr: SocketAddr,
}

#[tokio::main]
async fn main() {
    println!(
        r#"
        ____   ___   __   ____  ____   __    __  
        / ___) / __) /  \ (  _ \(  _ \ (  )  /  \ 
        \___ \( (__ (  O ) )   / ) __/  )(  (  O )
        (____/ \___) \__/ (__\_)(__)   (__)  \__/ 
"#
    );
    let args = Args::parse();

    if let Err(e) = config::init_config(&args.config_path) {
        eprintln!("Failed to load config: {e}");
        std::process::exit(1);
    }

    let mut manager = ScorpioManager::from_toml(config::config_file()).unwrap();
    manager.check().await;
    //init scorpio configuration

    let fuse_interface = MegaFuse::new_from_manager(&manager).await;
    let workspace = config::workspace();
    let mountpoint = OsStr::new(workspace);
    let lgfs = LoggingFileSystem::new(fuse_interface.clone());
    let mut mount_handle = mount_filesystem(lgfs, mountpoint).await;

    print!("server running...");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let mut daemon_task = tokio::spawn(daemon_main(
        Arc::new(fuse_interface),
        manager,
        shutdown_rx,
        args.http_addr,
    ));

    let mut mount_finished = false;
    tokio::select! {
        res = &mut mount_handle => {
            mount_finished = true;
            if let Err(e) = res {
                eprintln!("FUSE session ended with error: {e:?}");
            }
        }
        _ = shutdown_signal() => {
            // fallthrough to shutdown sequence below
        }
    }

    // Stop HTTP server first (this triggers Antares shutdown cleanup), then unmount the main workspace FS.
    let _ = shutdown_tx.send(());
    match tokio::time::timeout(std::time::Duration::from_secs(20), &mut daemon_task).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => eprintln!("HTTP daemon task join failed: {e}"),
        Err(_) => {
            eprintln!("HTTP daemon shutdown timed out; aborting task");
            daemon_task.abort();
        }
    }

    if !mount_finished {
        println!("unmount....");
        let _ = mount_handle.unmount().await;
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = signal::ctrl_c().await;
    }
}
