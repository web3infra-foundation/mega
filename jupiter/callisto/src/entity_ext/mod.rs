pub mod item_assignees;
pub mod item_labels;
pub mod label;
pub mod mega_conversation;
pub mod mega_issue;
pub mod mega_mr;
pub mod reactions;

use idgenerator::IdInstance;
use rand::Rng;

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

#[cfg(test)]
mod test {
    use crate::entity_ext::{generate_link, generate_public_id};

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
        println!("MR Link: '{link:?}'");
        assert!(
            link.chars().count() == 8
                && link.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
        )
    }
}
