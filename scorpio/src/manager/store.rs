use mercury::{hash::SHA1, internal::object::tree::Tree};
use tokio::sync::mpsc::Receiver;
use std::io::Result;

use crate::util::GPath;

pub trait TreeStore{
    fn get_tree(&self,hash:&SHA1)-> Result<Tree>;
    fn insert_tree(&self,tree:Tree)-> Result<()>;
    fn inser_path(&self,hash:&SHA1,path:GPath) -> Result<()>;
    fn get_hash_bypath(&self,path:GPath) -> Result<SHA1>;
    fn get_bypath(&self,path:GPath)-> Result<Tree>;
}
impl  TreeStore for sled::Db {
    fn get_tree(&self,hash:&SHA1)-> Result<Tree> {
        if let Some(encoded_value) = self.get(hash.as_ref())? {
            // Deserialize the encoded value into the original tree structure using bincode
            let decoded: Result<Tree> = bincode::deserialize(&encoded_value).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Deserialization error"));
            let decoded: Tree = decoded?;
             Ok(decoded)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Tree not found"))
        }
    }
    fn inser_path(&self,hash:&SHA1,path:GPath) -> Result<()>{
        let e = bincode::serialize(hash.as_ref()).unwrap();
        self.insert(path.to_string(), &e[..]).unwrap();
        Ok(())
    }
    fn get_hash_bypath(&self,path:GPath)-> Result<SHA1>{
        let hash = self.get(path.to_string())?;
        bincode::deserialize(&hash.unwrap()).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Deserialization error"))
    }
    fn insert_tree(&self,tree:Tree)-> Result<()> {
        let serialized_tree = bincode::serialize(&tree).unwrap();
        let re = self.insert(tree.id.as_ref(), serialized_tree);

        if re.is_ok(){
            Ok(())
        }else {
           Err(re.err().unwrap().into()) 
        }
    }
    
    fn get_bypath(&self,path:GPath)-> Result<Tree> {
        let hash  = self.get_hash_bypath(path)?;
        self.get_tree(&hash)
    }
}

pub async fn store_trees(storepath:&str,mut tree_channel: Receiver<(GPath,Tree)>) {
    let db = sled::open(storepath).unwrap();
    while let Some((path,tree)) = tree_channel.recv().await {
            println!("new tree:{}",tree.id);
            let _ = db.inser_path(&tree.id, path);
            let re = db.insert_tree(tree);
            if re.is_err(){
                print!("{}",re.err().unwrap());
            }
    }
    
    println!("finish store....");
}
#[cfg(test)]
mod test{
    use std::vec;
    use mercury::{hash::SHA1, internal::object::tree::{Tree, TreeItem, TreeItemMode}};

    #[test]
    fn init_test_d(){
        let db = sled::open("path.db").unwrap();
        let t = Tree::from_tree_items(vec![
            TreeItem::new(TreeItemMode::Blob, SHA1::new(&vec![4u8,4u8,4u8,64u8,84u8,84u8]), String::from("test") )
        ]).unwrap();
            
        if let Some(encoded_value) = db.get(t.id.as_ref()).unwrap() {
            // 使用 bincode 反序列化为原始的结构体
            let decoded:Tree = bincode::deserialize(&encoded_value).unwrap();
            println!(" {}", decoded);
        };
    }


    #[test]
    fn get_tree_test() {
        let db = sled::open("/home/luxian/megadir/store/1b70e8bf4d39d6f5e9dd1637aaa2c221e2d00a27/tree.db").unwrap();
        let iter = db.iter();
        for result in iter {
            match result {
                Ok((key, value)) => {
                    // Deserialize the value into the original tree structure using bincode
                    let decoded: Result<Tree, _> = bincode::deserialize(&value);
                    if let Ok(tree) = decoded {
                        println!("{}", tree);
                    } else {
                        println!("Deserialization error for key: {}", String::from_utf8_lossy(&key));
                    }
                }
                Err(error) => {
                    println!("Error iterating over trees: {}", error);
                }
            }
        }

        

    }

}
