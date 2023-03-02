//!
//!
//!
//!
//!

use crate::git::hash::Hash;

struct Tree {

}

impl Tree {
    #[allow(unused)]
    pub fn empty_tree_hash() -> Hash {
        Hash::default()
    }
}

mod tests {
    #[test]
    fn test_empty_tree_hash() {
        let hash = super::Tree::empty_tree_hash();
        assert_eq!(hash.to_plain_str(), "0000000000000000000000000000000000000000");
    }
}