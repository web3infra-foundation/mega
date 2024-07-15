
#[allow(unused)]
const READONLY_INODE: u64 = 0xffff_ffff;

#[allow(unused)]
pub trait RepoStore {}
#[allow(unused)]
struct WorkSpace {
    inode: u64,
    path: String ,
}
#[allow(unused)]
impl WorkSpace {
    pub fn init(path:String ) {
      
        let ovl = WorkSpace::new(path, READONLY_INODE);
    }
    pub fn new(path: String, inode: u64) -> WorkSpace {
        WorkSpace { inode, path }
    }
}
pub struct FileStore {}

