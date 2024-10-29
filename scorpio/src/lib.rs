

#[macro_use]
extern crate log;
mod passthrough;
mod overlayfs;
//mod store;
// pub mod fuse;
// mod dicfuse;
mod util;
// pub mod manager;
// pub mod server;
// pub mod deamon;
use once_cell::sync::OnceCell;
use tokio::runtime::Handle;

// 定义一个全局的 OnceCell 来存储 Tokio 运行时的句柄
static RUNTIME_HANDLE: OnceCell<Handle> = OnceCell::new();

// 初始化运行时并存储句柄
pub fn init_runtime(rt:Handle) {
    
    RUNTIME_HANDLE
        .set(rt)
        .expect("Failed to set runtime handle");
}

// 获取全局的运行时句柄
pub fn get_handle() -> &'static Handle {
    RUNTIME_HANDLE.get().expect("Runtime not initialized")
}