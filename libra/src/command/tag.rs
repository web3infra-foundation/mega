use crate::{
    command::get_target_commit, internal::{tag::TagInfo, head::Head}, internal::config
};
use clap::Parser;
use mercury::internal::object::{commit::Commit, signature::SignatureType, tag::Tag, types::ObjectType};
use mercury::internal::object::signature::Signature;
use mercury::hash::SHA1;


use crate::command::load_object;
use super::save_object;

#[derive(Parser, Debug)]
pub struct TagArgs {
    /// Name of the tag to create, delete, or inspect
    #[clap(group = "create")]
    tag_name: Option<String>,

    /// Commit hash or tag name to tag (default is HEAD)
    #[clap(requires = "create")]
    commit_hash: Option<String>,

    /// List all tags
    #[clap(short, long, group = "sub", default_value = "true")]
    list: bool,

    /// Create a new tag (lightweight or annotated)
    #[clap(short = 'a', long, group = "sub", group = "create")]
    annotate: Option<String>,

    /// The message for an annotated tag
    #[clap(short = 'm', long, requires = "annotate")]
    message: Option<String>,

    /// Delete a tag
    #[clap(short = 'd', long, group = "sub", requires = "tag_name")]
    delete: bool,

    /// change current tag
    #[clap(short = 'f', long, group = "sub")]
    force: bool,
}

pub async fn execute(args: TagArgs) {
    if args.delete {
        delete_tag(args.tag_name.unwrap()).await;
    } else if args.tag_name.is_some() {
        create_tag(args.tag_name.unwrap(), args.commit_hash).await;
    } else if args.annotate.is_some() {
        create_annotated_tag(args.annotate.unwrap(), args.message,  args.commit_hash).await;
    } else if args.list {
        // default behavior
        list_tags().await;
    } else {
        panic!("should not reach here")
    }
}

pub async fn create_tag(tag_name: String, commit_hash: Option<String>){
    tracing::debug!("create tag: {} from {:?}", tag_name, commit_hash);

    if !is_valid_git_tag_name(&tag_name) {
        eprintln!("fatal: invalid tag name: {}", tag_name);
        return;
    }

    // check if tag exists
    let tag = TagInfo::find_tag(&tag_name).await;
    if tag.is_some() {
        panic!("fatal: A tag named '{}' already exists.", tag_name);
    }

    let commit_id = match commit_hash {
        Some(commit_hash) => {
            let commit = get_target_commit(&commit_hash).await;
            match commit {
                Ok(commit) => commit,
                Err(e) => {
                    eprintln!("fatal: {}", e);
                    return;
                }
            }
        }
        None => Head::current_commit().await.unwrap(),
    };
    tracing::debug!("base commit_id: {}", commit_id);

    // check if commit_hash exists
    let _ = load_object::<Commit>(&commit_id)
        .unwrap_or_else(|_| panic!("fatal: not a valid object name: '{}'", commit_id));

    // create tag
    TagInfo::update_tag(&tag_name, &commit_id.to_string()).await;
}


async fn create_annotated_tag(tag_name: String, message: Option<String>, commit_hash: Option<String>) {
    create_tag(tag_name.clone(), commit_hash.clone()).await;
    let commit_id = match commit_hash {
        Some(commit_hash) => {
            let commit = get_target_commit(&commit_hash).await;
            match commit {
                Ok(commit) => commit,
                Err(e) => {
                    eprintln!("fatal: {}", e);
                    return;
                }
            }
        }
        None => Head::current_commit().await.unwrap(),
    };
    //let author = config::Config::get("user", None, "name").await.unwrap();
    //let email = config::Config::get("user", None, "email").await.unwrap();
    let author = "hemu";
    let email = "hemu@buaa.edu.cn";
    let tag = Tag {
        id: SHA1::default(),
        object_hash: commit_id,
        object_type: ObjectType::Tag,
        tag_name: tag_name,
        tagger: Signature::new(SignatureType::Tagger, author.to_owned(), email.to_owned()),
        message: message.unwrap_or_else(|| "".to_string()),
    };
    save_object(&tag, &tag.id).unwrap();
}

async fn delete_tag(tag_name: String) {
    let _ = TagInfo::find_tag(&tag_name)
        .await
        .unwrap_or_else(|| panic!("fatal: tag '{}' not found", tag_name));

    TagInfo::delete_tag(&tag_name).await;
}

async fn list_tags() {
    let tags = TagInfo::list_tags().await;
    for tag in tags {
        println!("{}", tag.name);
    }
}



fn is_valid_git_tag_name(tag_name: &String) -> bool {
    // Check for empty or invalid length
    if tag_name.is_empty() || tag_name.len() > 255 {
        return false;
    }

    // Reserved names
    let reserved_names = [
        "HEAD", "@", "FETCH_HEAD", "ORIG_HEAD", "MERGE_HEAD", "REBASE_HEAD",
    ];
    if reserved_names.contains(&tag_name.as_str()) {
        return false;
    }

    // Check for forbidden characters
    let forbidden_chars = [' ', '~', '^', ':', '?', '*', '[', '\x00', '\x7f'];
    if tag_name.chars().any(|c| forbidden_chars.contains(&c) || c.is_control()) {
        return false;
    }

    // Check for invalid start or end characters
    if tag_name.starts_with('.') || tag_name.starts_with('/') || tag_name.ends_with('.') || tag_name.ends_with('/') {
        return false;
    }

    // Check for double slashes
    if tag_name.contains("//") {
        return false;
    }

    // Check for trailing '@'
    if tag_name.ends_with('@') {
        return false;
    }

    true
}
