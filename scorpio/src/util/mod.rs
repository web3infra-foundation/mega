use std::fmt::Display;

use serde::{Deserialize, Serialize};
#[derive(Debug,Deserialize, Serialize,Clone,Default)]
pub struct GPath{
   pub path:Vec<String>
}

#[allow(unused)]

impl GPath{
    pub fn new() -> GPath{
        GPath{
            path:Vec::new()        
        }
    }
    pub fn push(&mut self, path:String){
        self.path.push(path);
    }
    pub fn name(&self) -> String{
        self.path.last().unwrap().clone()
    }

}

impl From<String> for GPath{
    fn from(mut s: String) -> GPath {
        if s.starts_with('/'){
            s.remove(0);
        }
        GPath {
            path: s.split('/').map(String::from).collect(),
        }
    }
}


impl Display for GPath{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.path.join("/"))
    }
}

#[cfg(test)]
mod tests{
    use super::GPath;

    #[test]
    fn test_from_string(){
        let path  = String::from("/release");
        let gapth  = GPath::from(path);
        assert_eq!(gapth.to_string(),String::from("release"))
    }
}
