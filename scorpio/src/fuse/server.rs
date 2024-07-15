
use std::{borrow::BorrowMut, sync::Arc};
use std::io::Result;
use fuse_backend_rs::abi::fuse_abi::InHeader;
use fuse_backend_rs::api::server::Server;
use fuse_backend_rs::transport::FuseChannel;


use crate::overlayfs::OverlayFs;

use super::inode_alloc::InodeAlloc;

#[allow(unused)]
pub struct FuseServer {
    server: Vec<Server<Arc<OverlayFs>>>,
    inodes:InodeAlloc,
    ch: FuseChannel,
}
#[allow(unused)]
impl FuseServer {
    pub fn new(channel: FuseChannel)->Self{
        //input the overlay fs vec  and init the inodes alloctor
        let mut servers = Vec::new();//TODO :init overlay fs by new func.
        let mut inodes = InodeAlloc::new();
        let lens = servers.len();
        for i in 1..=lens{
           let key =  inodes.alloc_inode(i.try_into().unwrap());
           //TODO : Pre-alloc the inodes numbers: 
           //let server_ref = Arc::get_mut(&mut servers[i-1]).unwrap();
           //server_ref.extend_inode_alloc(key);
        }
        
        Self {
            server: servers,
            inodes:InodeAlloc::new(),
            ch: channel,
        }
    }
    pub fn svc_loop(&mut self) -> Result<()> {
        let _ebadf = std::io::Error::from_raw_os_error(libc::EBADF);
        println!("entering server loop");
        loop {
            if let Some((mut reader, writer)) = self
                .ch
                .get_request()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?
            {
                let in_header: InHeader = reader.read_obj().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                        if in_header.len > 0 {
                            let inode = in_header.nodeid;
                            //Assign the request to the corresponding overlay through an inode number
                            let ovl_inode = self.inodes.get_ovl_inode(inode);
                            match ovl_inode {
                                Some(ovl_index) => {
                                    let mut server = self.server[ovl_index as usize].borrow_mut();
                                    if let Err(e) = server.handle_message(reader, writer.into(), None, None){
                                        match e {
                                            fuse_backend_rs::Error::EncodeMessage(_ebadf) => {
                                                break;
                                            }
                                            _ => {
                                                print!("Handling fuse message failed");
                                                continue;
                                            }
                                            //TOSO: to much nesting 
                                        }
                                    }
                                }
                                None => todo!(), //TODO: deal with no path inodes match
                            };
                            
                        }
            } else {
                print!("fuse server exits");
                break;
            }
        }
        Ok(())
    }
}