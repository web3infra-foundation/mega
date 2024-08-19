#[allow(unused_imports)]
pub use fuse_backend_rs::passthrough;
use std::io::Result;
use crate::overlayfs::BoxedLayer;
pub mod logwrapper;
pub fn new_passthroughfs_layer(rootdir: &str) -> Result<BoxedLayer> {
    let config = fuse_backend_rs::passthrough::Config { 
        root_dir: String::from(rootdir), 
        // enable xattr`
        xattr: true, 
        do_import: true, 
        ..Default::default() };

    let fs = Box::new(passthrough::PassthroughFs::<()>::new(config)?);
    
    fs.import()?;
    Ok(fs as BoxedLayer)
}