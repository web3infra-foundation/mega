

use fuse_backend_rs::api::filesystem::{Context, DirEntry, Entry};
use std::sync::Arc;
use std::{cell::RefCell, io::Result, rc::Rc};
use std::collections::VecDeque;
use super::{BoxedLayer, OverlayFs};
#[allow(dead_code)]
pub trait FSdiff {
    fn diff(&self);
}
#[allow(unused)]
impl FSdiff for OverlayFs{
    fn diff(&self) {
        let upper = match &self.upper_layer{
            Some(a) => a.clone(),
            None => return ,
        };
        let _lower  = self.lower_layers.clone();
        let ctx= Context::new();
        let _upper_inodes = traverse_directory(&ctx, upper);
    }
}
// 用于遍历目录的函数
fn traverse_directory(ctx: &Context, fs: Arc<BoxedLayer>) -> Result<Vec<u64>> {

    let buffer_size = 1024; 

    let mut entrys_inode = Vec::new();
    println!("root ino:{}",fs.root_inode());
    let dir_inodes = Rc::new(RefCell::new(VecDeque::from([fs.root_inode()])));

    let mut add_entry = |entry: DirEntry, e:Entry| -> Result<usize> {
        println!("inode:{}, type:{}, name:{} ",entry.ino, entry.type_, std::str::from_utf8(entry.name).unwrap());
        entrys_inode.push(e.inode);
        if entry.type_ == libc::DT_DIR as u32 {
            println!("push dir ino:{}", e.inode);
            dir_inodes.borrow_mut().push_back(e.inode);
        }
        Ok(1)
    };
    while !dir_inodes.borrow().is_empty() {
        let mut  b= dir_inodes.borrow_mut();
        let current_inode = b.pop_front().unwrap();
        drop(b);
        // 调用 readdir 方法来填充条目
        let attrs = fs.getattr(ctx,current_inode,None).unwrap();
        println!("inode1:{}, inode2:{}", attrs.0.st_ino, current_inode);
        let handle = fs.opendir(ctx, current_inode,100352).unwrap().0;
        let result = fs.readdirplus(
            ctx,
            current_inode,
            handle.unwrap(),
            buffer_size as u32,
            0,
            &mut add_entry,
        );
        // 处理 result，更新 offset 或进行其他处理
        match result {
            Ok(_) => {
                // 可以根据需要更新 offset
                // offset = 更新后的 offset;
            }
            Err(e) => {
                // 处理错误
                return Err(e);
            }
        }
    }

    Ok(entrys_inode)
}

   
#[cfg(test)]
mod tests{
    use std::sync::Arc;
    use fuse_backend_rs::{abi::fuse_abi::FsOptions, api::filesystem::Context};
    use crate::passthrough::new_passthroughfs_layer;
    use super::traverse_directory;

    #[test]
    fn test_tracerse_drectory(){
        let fs = new_passthroughfs_layer("/home/luxian/megatest/lower").unwrap();
        fs.init(FsOptions::empty()).unwrap();
        let ctx = Context::new();
        let out = traverse_directory(&ctx,Arc::new(fs)).unwrap();
        for ino in  out{
            println!("inode: {}", ino);
        }
    }
}