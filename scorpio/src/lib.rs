

#[macro_use]
extern crate log;
mod passthrough;
mod overlayfs;
//mod store;
mod fuse;
mod dicfuse;


use std::sync::Arc;
use std::io::Result;
use fuse_backend_rs::{api::{filesystem::FileSystem, server::Server}, transport::FuseChannel};


pub struct FuseServer<FS:FileSystem+std::marker::Sync> {
    server: Arc<Server<FS>>,
    ch: FuseChannel,
}
impl <FS:FileSystem+std::marker::Sync>FuseServer<FS> {
    pub fn svc_loop(&mut self) -> Result<()> {
        let _ebadf = std::io::Error::from_raw_os_error(libc::EBADF);
        println!("entering server loop");
        loop {
            if let Some((reader, writer)) = self
                .ch
                .get_request()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?
            {
                if let Err(e) = self
                    .server
                    .handle_message(reader, writer.into(), None, None)
                {
                    match e {
                        fuse_backend_rs::Error::EncodeMessage(_ebadf) => {
                            break;
                        }
                        _ => {
                            print!("Handling fuse message failed");
                            continue;
                        }
                    }
                }
            } else {
                print!("fuse server exits");
                break;
            }
        }
        Ok(())
    }
}
