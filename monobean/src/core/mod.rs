use adw::gio;
use adw::gio::ResourceLookupFlags;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

pub mod delegate;
pub mod mega_core;
pub mod servers;

#[derive(Debug, Clone)]
pub enum CoreConfigChanged {
    BaseDir(PathBuf),

    // Log Config
    LogPath(PathBuf),
    Level(String),
    PrintStd(bool),

    // Database Config
    DbType(String),
    DbPath(String),
    DbUrl(String),
    MaxConnection(u32),
    MinConnection(u32),
    SqlxLogging(bool),

    // Monorepo Config
    ImportDir(PathBuf),
    Admin(String),
    RootDirs(Vec<String>),

    // Auth Config
    EnableHttpAuth(bool),
    EnableTestUser(bool),
    TestUserName(String),
    TestUserToken(String),

    // Pack Config
    PackDecodeMemSize(String),
    PackDecodeDiskSize(String),
    PackDecodeCachePath(PathBuf),
    CleanCacheAfterDecode(bool),
    ChannelMessageSize(usize),

    // LFS Config
    LfsUrl(String),

    // OAuth Config
    GithubClientId(String),
    GithubClientSecret(String),
    UiDomain(String),
    CookieDomain(String),

    // P2P Options
    P2POption(String),
}

/// For running mega core, we have to set up tokio runtime.
pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Setting up tokio runtime must succeed.")
    })
}

pub fn load_mega_resource(path: &str) -> Vec<u8> {
    let bytes = gio::resources_lookup_data(path, ResourceLookupFlags::all()).unwrap();
    bytes.as_ref().into()
}
