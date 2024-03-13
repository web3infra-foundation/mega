use std::{
    fs::File,
    io::{BufRead, BufReader, ErrorKind},
};

use clap::Parser;
use fuse_demo::common::{DEFAULT_DATA_DIR_PREFIX, DEFAULT_DIRECT_IO, DEFAULT_LOG_DIR_PREFIX};
use fuser::MountOption;
use simple_log::LogConfigBuilder;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filesystem name
    #[arg(short, long)]
    name: String,
    /// The local cache directory for remote files
    #[arg(long,default_value = None)]
    data_dir: Option<String>,
    /// Log file location
    #[arg(long,default_value = None)]
    log_path: Option<String>,
    /// File system mount point
    #[arg(short, long)]
    mount_point: String,
    #[arg(long)]
    direct_io: Option<bool>,
    /// Api interface address
    #[arg(short, long)]
    server_url: String,
    /// Remote file root directory
    #[arg(short, long)]
    remote_root: String,
}

fn fuse_allow_other_enabled() -> bool {
    let file = match File::open("/etc/fuse.conf") {
        Ok(f) => f,
        Err(e) => {
            println!("{}", e);
            panic!("Unable to read /etc/fuse.conf");
        }
    };
    for line in BufReader::new(file).lines() {
        if line.unwrap().trim_start().starts_with("user_allow_other") {
            return true;
        }
    }
    false
}

fn main() {
    let args = Args::parse();
    let fs_name = args.name;
    // init logger
    let config = args
        .log_path
        .map_or(
            LogConfigBuilder::builder().path(DEFAULT_LOG_DIR_PREFIX.to_string() + "/" + &fs_name),
            |path| LogConfigBuilder::builder().path(path),
        )
        .size(10000)
        .roll_count(10)
        .time_format("%Y-%m-%d %H:%M:%S.%f")
        .level("debug")
        .output_file()
        .build();
    if let Err(e) = simple_log::new(config) {
        panic!("{}", e);
    }

    let mut options = vec![MountOption::FSName(fs_name.clone()), MountOption::AllowRoot];
    if fuse_allow_other_enabled() {
        options.push(MountOption::AllowOther);
    } else {
        panic!("Fuse not allow other! Please add user_allow_other in /etc/fuse.conf.")
    }

    let data_dir = args
        .data_dir
        .map_or(DEFAULT_DATA_DIR_PREFIX.to_owned() + "/" + &fs_name, |dir| {
            dir
        });

    let mount_point = args.mount_point;
    let direct_io = args.direct_io.map_or(DEFAULT_DIRECT_IO, |di| di);

    let server_url = args.server_url;
    let remote_root = args.remote_root;
    println!("{mount_point},{server_url},{remote_root},{fs_name}");
    let fs =
        fuse_demo::fs::RLFileSystem::new(server_url, fs_name, direct_io, remote_root, data_dir);
    if let Err(e) = fuser::mount2(fs, mount_point, &options) {
        if e.kind() == ErrorKind::PermissionDenied {
            panic!("{}", e);
        }
    }
}
