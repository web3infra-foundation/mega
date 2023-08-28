use fuser::MountOption;

fn main() {
    let options = vec![
        MountOption::FSName("demo_fuse".to_string()),
        MountOption::AutoUnmount,
        MountOption::AllowRoot,
    ];

    let mountpoint = "/home/qzl/Temp/fuse_dir";
    fuser::mount2(
        fuse_demo::fs::RLFileSystem::new("http://localhost:8080/list","git fs",true,""),
        mountpoint,
        &options,
    ).unwrap();
}
