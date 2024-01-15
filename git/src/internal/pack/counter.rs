use std::fmt::Display;

use serde::{Deserialize, Serialize};
/// A Counter for counting git object types
#[derive(Default, Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub struct GitTypeCounter {
    pub commit: usize,
    pub tree: usize,
    pub blob: usize,
    pub tag: usize,
    pub ofs_delta: usize,
    pub ref_delta: usize,
}
impl GitTypeCounter {
    pub fn count(&mut self, type_num: u8) {
        match type_num {
            1 => self.commit += 1,
            2 => self.tree += 1,
            3 => self.blob += 1,
            4 => self.tag += 1,
            6 => self.ofs_delta += 1,
            7 => self.ref_delta += 1,
            _ => panic!("unknow git type in GitTypeCounter"),
        }
    }
    #[inline]
    pub fn base_count(&self) -> usize {
        self.commit + self.tree + self.blob + self.tag
    }
    #[inline]
    pub fn delta_count(&self) -> usize {
        self.ref_delta + self.ofs_delta
    }
}
impl Display for GitTypeCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "***Git Type Counter Info***")?;
        writeln!(
            f,
            "commit:{}  tree:{}   blob:{}  tag:{}\tref_delta :{}\tofs_dalta:{}",
            self.commit, self.tree, self.blob, self.tag, self.ref_delta, self.ofs_delta
        )?;

        writeln!(
            f,
            "base_object:{} \t delta_object:{} \t",
            self.base_count(),
            self.delta_count()
        )
    }
}
/// Counter Type in The process of parsing and saving git objects
pub enum CounterType {
    Base,
    Delta,
    CacheHit,
    DB,
}
/// A Counter in The process of parsing and saving git objects
#[derive(Default)]
pub struct DecodeCounter {
    base: usize,
    delta: usize,
    cache_hit: usize,
    db_look: usize,
    delta_depth: usize,
    //TODO time count
}

impl DecodeCounter {
    pub fn count(&mut self, ct: CounterType) {
        match ct {
            CounterType::Base => self.base += 1,
            CounterType::Delta => self.delta += 1,
            CounterType::CacheHit => self.cache_hit += 1,
            CounterType::DB => self.db_look += 1,
        }
    }
    pub fn count_depth(&mut self, depth: usize) {
        self.delta_depth += depth;
    }
}
impl Display for DecodeCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "base:{} delta:{}, cache_hit: :{},Storage operation:{},delta_depth:{}",
            self.base, self.delta, self.cache_hit, self.db_look, self.delta_depth
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::internal::pack::counter::GitTypeCounter;
    #[test]
    fn test_git_type_counter() {
        let mut counter = GitTypeCounter::default();
        counter.count(1);
        counter.count(2);
        counter.count(3);
        counter.count(4);
        counter.count(6);
        counter.count(7);
        assert_eq!(counter.base_count(), 4);
        assert_eq!(counter.delta_count(), 2);
        print!("{}", counter);
    }
}
