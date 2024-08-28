

use fuse_backend_rs::api::filesystem::{Context, DirEntry, Entry};
use std::sync::Arc;
use std::{cell::RefCell, io::Result, rc::Rc};
use std::collections::VecDeque;
use crate::util::GPath;

use super::{BoxedLayer, OverlayFs};
#[allow(dead_code)]
pub trait FSdiff {
    fn diff(&self);
}
#[allow(unused)]
impl FSdiff for OverlayFs{
    fn diff(&self) {
        println!("root work path :{} mount point :{}",self.config.work, self.config.mountpoint);
        let upper = match &self.upper_layer{
            Some(a) => a.clone(),
            None => return ,
        };
        let _lower  = self.lower_layers.clone();
        let ctx= Context::new();
        let upper_inodes = traverse_directory(&ctx, upper).unwrap();
        // Traverse all modified nodes and search for corresponding files in the lower level file library
        for upder_inode in upper_inodes{
           
            println!("inode:{},path:{}",upder_inode.0, upder_inode.1);
        }
        
    }
}

// traverse a file Dictionary and find all inodes.
fn traverse_directory(ctx: &Context, fs: Arc<BoxedLayer>) -> Result<Vec<(u64,GPath)>> {

    let buffer_size = 1024; 
    // return diff inodes
    let mut entrys_inode: Vec<(u64, GPath)> = Vec::new();
    println!("root ino:{}",fs.root_inode());
    // bfs lookup inodes. 
    let dir_inodes = Rc::new(RefCell::new(VecDeque::from([(fs.root_inode(), GPath::new())])));
    let path = Rc::new(RefCell::new(GPath::new())) ;
    
    let mut add_entry = |entry: DirEntry, e:Entry| -> Result<usize> {
        let node_name = std::str::from_utf8(entry.name).unwrap().to_string();
        println!("inode:{}, type:{}, name:{} ",e.inode, entry.type_, node_name);
        let mut gpath = path.borrow().clone();
        gpath.push(node_name);
        entrys_inode.push((e.inode,gpath.clone()));
        if entry.type_ == libc::DT_DIR as u32 {
            println!("push dir ino:{}", e.inode);
            dir_inodes.borrow_mut().push_back((e.inode,gpath.clone()));
        }
        Ok(1)
    };
    while !dir_inodes.borrow().is_empty() {
        let mut b= dir_inodes.borrow_mut();
        let current_inode = b.pop_front().unwrap();
        *path.borrow_mut() = current_inode.1;
        drop(b);
        // call the [readdir] func. to fill 
        let attrs = fs.getattr(ctx,current_inode.0,None).unwrap();
        println!("inode1:{}, inode2:{}", attrs.0.st_ino, current_inode.0);
        let handle = fs.opendir(ctx, current_inode.0,100352).unwrap().0;
        let result = fs.readdirplus(
            ctx,
            current_inode.0,
            handle.unwrap(),
            buffer_size as u32,
            0,
            &mut add_entry,
        );
        // deeal with result，update offset or ERROR
        match result {
            Ok(_) => {
                 //pass
            }
            Err(e) => {
                // ERRER pass
                return Err(e);
            }
        }
    }
    Ok(entrys_inode)
}

   
#[cfg(test)]
mod tests{
    use std::{ffi::CStr, sync::Arc};
    use fuse_backend_rs::{abi::fuse_abi::FsOptions, api::filesystem::Context};
    use crate::passthrough::new_passthroughfs_layer;
    use super::traverse_directory;

    #[test]
    fn test_tracerse_drectory(){
        let fs = new_passthroughfs_layer("/home/luxian/megatest/lower").unwrap();
        fs.init(FsOptions::empty()).unwrap();
        let ctx = Context::new();
        let afs = Arc::new(fs);
        let _out = traverse_directory(&ctx,afs.clone()).unwrap();

        let bytes_with_nul: &[u8] = b".\0";
        let cstr = CStr::from_bytes_with_nul(bytes_with_nul).expect("CStr creation failed");
        let e = afs.lookup(&ctx, afs.root_inode(),cstr).unwrap();
        //afs.readdirplus(ctx, inode, handle, size, offset, add_entry);
        println!("{:?}",e)
    }
}