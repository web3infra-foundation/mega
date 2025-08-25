use crate::internal::tag;
use clap::Parser;
use mercury::internal::object::types::ObjectType;
use sea_orm::sqlx::types::chrono;

#[derive(Parser, Debug)]
#[command(about = "Create, list, delete, or verify a tag object")]
pub struct TagArgs {
    /// The name of the tag to create, show, or delete
    #[clap(required = false)]
    pub name: Option<String>,

    /// List all tags
    #[clap(short, long, group = "action")]
    pub list: bool,

    /// Delete a tag
    #[clap(short, long, group = "action")]
    pub delete: bool,

    /// Message for the annotated tag. If provided, creates an annotated tag.
    #[clap(short, long)]
    pub message: Option<String>,
}

pub async fn execute(args: TagArgs) {
    if args.list {
        list_tags().await;
        return;
    }

    if let Some(name) = args.name {
        if args.delete {
            delete_tag(&name).await;
        } else if args.message.is_some() {
            create_tag(&name, args.message).await;
        } else {
            show_tag(&name).await;
        }
    } else {
        list_tags().await;
    }
}

async fn create_tag(tag_name: &str, message: Option<String>) {
    match tag::create(tag_name, message).await {
        Ok(_) => (),
        Err(e) => eprintln!("fatal: {}", e),
    }
}

async fn list_tags() {
    match tag::list().await {
        Ok(tags) => {
            for tag in tags {
                println!("{}", tag.name);
            }
        }
        Err(e) => eprintln!("fatal: {}", e),
    }
}

async fn delete_tag(tag_name: &str) {
    match tag::delete(tag_name).await {
        Ok(_) => println!("Deleted tag '{}'", tag_name),
        Err(e) => eprintln!("fatal: {}", e),
    }
}

async fn show_tag(tag_name: &str) {
    match tag::find_tag_and_commit(tag_name).await {
        Ok(Some((object, commit))) => {
            if object.get_type() == ObjectType::Tag {
                // Access the tag data directly from the object if it is a Tag variant.
                if let tag::TagObject::Tag(tag_object) = &object {
                    println!("tag {}", tag_object.tag_name);
                    println!("Tagger: {}", tag_object.tagger.to_string().trim());
                    println!("\n{}", tag_object.message);
                } else {
                    eprintln!("fatal: object is not a Tag variant");
                    return;
                }
            }

            println!("\ncommit {}", commit.id);
            println!("Author: {}", commit.author.to_string().trim());
            let commit_date =
                chrono::DateTime::from_timestamp(commit.committer.timestamp as i64, 0)
                    .unwrap_or(chrono::DateTime::UNIX_EPOCH);
            println!("Date:   {}", commit_date.to_rfc2822());
            println!("\n    {}", commit.message.trim());
        }
        Ok(None) => eprintln!("fatal: tag '{}' not found", tag_name),
        Err(e) => eprintln!("fatal: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_async;
    use crate::internal::tag;
    use mercury::internal::object::types::ObjectType;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    async fn setup_repo_with_commit() -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        parse_async(Some(&["libra", "init"])).await.unwrap();
        fs::write("test.txt", "hello").unwrap();
        parse_async(Some(&["libra", "add", "test.txt"]))
            .await
            .unwrap();
        parse_async(Some(&["libra", "commit", "-m", "Initial commit"]))
            .await
            .unwrap();
        temp_dir
    }

    #[tokio::test]
    #[serial]
    async fn test_create_and_list_lightweight_tag() {
        let _temp_dir = setup_repo_with_commit().await;
        create_tag("v1.0-light", None).await;
        let tags = tag::list().await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0-light");
        assert_eq!(tags[0].object.get_type(), ObjectType::Commit);
    }

    #[tokio::test]
    #[serial]
    async fn test_create_and_list_annotated_tag() {
        let _temp_dir = setup_repo_with_commit().await;
        create_tag("v1.0-annotated", Some("Release v1.0".to_string())).await;
        let tags = tag::list().await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0-annotated");
        assert_eq!(tags[0].object.get_type(), ObjectType::Tag);
    }

    #[tokio::test]
    #[serial]
    async fn test_show_lightweight_tag() {
        let _temp_dir = setup_repo_with_commit().await;
        create_tag("v1.0-light", None).await;
        let result = tag::find_tag_and_commit("v1.0-light").await;
        assert!(result.is_ok());
        let (object, commit) = result.unwrap().unwrap();
        assert_eq!(object.get_type(), ObjectType::Commit);
        assert_eq!(commit.message.trim(), "Initial commit");
    }

    #[tokio::test]
    #[serial]
    async fn test_show_annotated_tag() {
        let _temp_dir = setup_repo_with_commit().await;
        create_tag("v1.0-annotated", Some("Test message".to_string())).await;
        let result = tag::find_tag_and_commit("v1.0-annotated").await;
        assert!(result.is_ok());
        let (object, commit) = result.unwrap().unwrap();
        assert_eq!(object.get_type(), ObjectType::Tag);
        assert_eq!(commit.message.trim(), "Initial commit");

        // Verify tag object content directly from the TagObject enum
        if let tag::TagObject::Tag(tag_object) = object {
            assert_eq!(tag_object.message, "Test message");
        } else {
            panic!("Expected Tag object type");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_tag() {
        let _temp_dir = setup_repo_with_commit().await;
        create_tag("v1.0", None).await;
        delete_tag("v1.0").await;
        let tags = tag::list().await.unwrap();
        assert!(tags.is_empty());
    }
}
