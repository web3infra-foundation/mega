use adw::gio;
use adw::gio::ResourceLookupFlags;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

pub mod delegate;
pub mod mega_core;
pub mod servers;

// For running mega core, we should set up tokio runtime.
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
