use std::{fmt::Display, path::PathBuf};
use serde::{Deserialize, Serialize};

pub mod scorpio_config;

#[derive(Debug,Deserialize, Serialize,Clone,Default)]
pub struct GPath{
   pub path:Vec<String>
}


impl GPath{
    pub fn new() -> GPath{
        GPath{
            path:Vec::new()        
        }
    }
    pub fn push(&mut self, path: String) {
        if path.contains('/') {
            for part in path.split('/') {
                if !part.is_empty() {
                    self.path.push(part.to_string());
                }
            }
        } else {
            self.path.push(path);
        }
    }
    pub fn pop(&mut self)->Option<String>  {
        self.path.pop()
    }
    pub fn name(&self) -> String{
        self.path.last().unwrap().clone()
    }
    pub fn part(&self,i:usize,j :usize) ->String{
        self.path[i..j].join("/")
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

impl  From<GPath> for PathBuf {
    fn from(val: GPath) -> Self {
        let path_str = val.path.join("/");
        PathBuf::from(path_str)
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
