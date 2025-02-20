use mercury::internal::object::{commit::Commit, tree::Tree};
use tokio::sync::mpsc::Receiver;
use std::{io::Result, path::PathBuf};

use crate::util::GPath;

pub trait TreeStore{

    fn insert_tree(&self,path:PathBuf, tree:Tree)-> Result<()>;
    fn get_bypath(&self,path:PathBuf)-> Result<Tree>;
}

impl  TreeStore for sled::Db {
    fn insert_tree(&self,path:PathBuf, tree:Tree)-> Result<()> {
        let value = bincode::serialize(&tree).unwrap();
        let key = path.to_str().unwrap();
        self.insert(key, value).unwrap();
        Ok(())
    }

    fn get_bypath(&self,path:PathBuf)-> Result<Tree> {
        let key = path.to_str().unwrap();
        let encoded_value= self.get(key)?;
        let decoded: Result<Tree> = bincode::deserialize(&encoded_value.unwrap()).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Deserialization error"));
        let decoded: Tree = decoded?;
        Ok(decoded)
    }
}
pub trait CommitStore{
    fn store_commit(&self,commit:Commit) -> Result<()>;
    fn get_commit(&self) -> Result<Commit>;
}
impl CommitStore for sled::Db{
    fn store_commit(&self,commit:Commit) -> Result<()> {
        let re = self.insert("COMMIT",bincode::serialize(&commit).unwrap())?;
        if re.is_some(){
            Ok(())
        }else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to store commit"))
        }
    }

    fn get_commit(&self) -> Result<Commit> {
        let encoded_value= self.get("COMMIT")?;
        let decoded: Result<Commit> = bincode::deserialize(&encoded_value.unwrap()).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Deserialization error"));
        decoded
    }
}
pub async fn store_trees(storepath:&str,mut tree_channel: Receiver<(GPath,Tree)>) {
    let db = sled::open(storepath).unwrap();
    while let Some((path,tree)) = tree_channel.recv().await {
            println!("new tree:{}",tree.id);
            let re = db.insert_tree(path.into(),tree);
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
            TreeItem::new(TreeItemMode::Blob, SHA1::new(&[4u8,4u8,4u8,64u8,84u8,84u8]), String::from("test") )
        ]).unwrap();
            
        if let Some(encoded_value) = db.get(t.id.as_ref()).unwrap() {
            // use bincode to deserialize the value .
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
                    let key_str = std::str::from_utf8(&key).unwrap();
                    
                    println!("path:{}", key_str);
                 
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
