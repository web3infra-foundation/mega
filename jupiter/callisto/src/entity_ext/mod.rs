pub mod buck_session;
pub mod buck_session_file;
pub mod check_result;
pub mod item_assignees;
pub mod item_labels;
pub mod label;
pub mod mega_cl;
pub mod mega_code_review_anchor;
pub mod mega_code_review_comment;
pub mod mega_code_review_position;
pub mod mega_code_review_thread;
pub mod mega_conversation;
pub mod mega_issue;
pub mod mega_refs;
pub mod reactions;

use idgenerator::IdInstance;
use rand::Rng;
use sha2::{Digest, Sha256};

pub fn generate_id() -> i64 {
    // Call `next_id` to generate a new unique id.
    IdInstance::next_id()
}

pub fn generate_link() -> String {
    let rng = rand::rng();
    let str: String = rng
        .sample_iter(rand::distr::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    str.to_uppercase()
}

pub fn generate_public_id() -> String {
    let rng = rand::rng();
    let str: String = rng
        .sample_iter(rand::distr::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    str.to_lowercase()
}

pub fn generate_hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn is_same_content(content: &str, hash: &str) -> bool {
    generate_hash_content(content) == hash
}

#[cfg(test)]
mod test {
    use crate::entity_ext::{
        generate_hash_content, generate_link, generate_public_id, is_same_content,
    };

    #[test]
    fn test_pub_id_generate() {
        let link = generate_public_id();
        println!("public id: {link:?}");
        assert!(
            link.chars().count() == 12
                && link
                    .chars()
                    .all(|c| c.is_alphanumeric() || c.is_lowercase())
        )
    }

    #[test]
    fn test_link_generate() {
        let link = generate_link();
        println!("CL Link: '{link:?}'");
        assert!(
            link.chars().count() == 8
                && link.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
        )
    }

    #[test]
    fn test_generate_hash_content() {
        let input = "hello world";
        let hash = generate_hash_content(input);
        println!("Generated hash: {}", hash);

        // Expected SHA256 hash for "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(hash, expected, "Hash generation is incorrect");
    }

    #[test]
    fn test_is_same_content() {
        let content = "test string";
        let hash = generate_hash_content(content);
        println!("Content: '{}', Hash: {}", content, hash);

        // Correct match
        assert!(
            is_same_content(content, &hash),
            "Content should match its hash"
        );

        // Incorrect match
        let wrong_hash = generate_hash_content("other string");
        println!("Wrong hash: {}", wrong_hash);
        assert!(
            !is_same_content(content, &wrong_hash),
            "Different content should not match hash"
        );
    }
}
