use clap::{Args, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;

use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::ObjectTrait;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tag::Tag;
use mercury::internal::refs::tag_ref::TagRef;

use crate::command;
use crate::internal::head::Head;
use crate::utils::util;

#[derive(Args, Debug)]
pub struct TagArgs {
    /// Tag name
    #[arg(default_value = "")]
    name: String,

    /// Specify commit to tag (default: HEAD)
    #[arg(long)]
    commit: Option<String>,

    /// Create an annotated tag
    #[arg(short, long)]
    annotate: bool,

    /// Use the given message for the tag
    #[arg(short, long)]
    message: Option<String>,

    /// Delete a tag
    #[arg(short, long)]
    delete: bool,

    /// Show tag details
    #[arg(short, long)]
    verbose: bool,
}

/// Execute tag command
pub async fn execute(args: TagArgs) -> Result<(), GitError> {
    if args.delete {
        // Delete tag
        if args.name.is_empty() {
            eprintln!("Error: Tag name must be specified when deleting a tag");
            exit(1);
        }
        return delete_tag(&args.name).await;
    } else if args.name.is_empty() {
        // List all tags
        return list_tags(args.verbose).await;
    } else {
        // Create tag
        return create_tag(
            &args.name,
            args.commit.as_deref(),
            args.annotate,
            args.message.as_deref(),
        )
        .await;
    }
}

/// List all tags
async fn list_tags(verbose: bool) -> Result<(), GitError> {
    let tags = TagRef::list_tags().await?;
    
    if tags.is_empty() {
        return Ok(());
    }

    // Sort tags alphabetically
    let mut tags = tags;
    tags.sort_by(|a, b| a.name.cmp(&b.name));
    
    for tag in tags {
        if verbose {
            let tag_obj_result = TagRef::get_tag_object(&tag.name).await;
            match tag_obj_result {
                Ok(tag_obj) => {
                    println!("{} {} {}", tag.name, tag_obj.target, tag_obj.message);
                }
                Err(_) => {
                    // Lightweight tag
                    println!("{} {}", tag.name, tag.commit);
                }
            }
        } else {
            println!("{}", tag.name);
        }
    }
    
    Ok(())
}

/// Create a tag
async fn create_tag(
    name: &str,
    commit_ref: Option<&str>,
    annotate: bool,
    message: Option<&str>,
) -> Result<(), GitError> {
    // Check if tag already exists
    if TagRef::exists(name).await? {
        eprintln!("fatal: tag '{}' already exists", name);
        exit(1);
    }

    // Get target commit
    let commit_sha = match commit_ref {
        Some(commit_ref) => command::get_target_commit(commit_ref).await.map_err(|e| {
            GitError::InvalidArgument(format!("Could not find specified commit: {}", e))
        })?,
        None => Head::current_commit().await?,
    };

    // Verify commit is valid
    let commit = command::load_object::<Commit>(&commit_sha)?;

    if annotate {
        // Create annotated tag
        let tag_message = match message {
            Some(msg) => msg.to_string(),
            None => {
                // Prompt for message if not provided
                print!("Enter tag message: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                input.trim().to_string()
            }
        };

        // Create tag object
        let tag = Tag::new(name, &commit_sha, &tag_message);
        
        // Save tag object
        command::save_object(&tag, &tag.id)?;
        
        // Create tag reference
        TagRef::create(name, &tag.id).await?;
        
        println!("Created tag '{}' (annotated)", name);
    } else {
        // Create lightweight tag (points directly to commit)
        TagRef::create(name, &commit_sha).await?;
        println!("Created tag '{}'", name);
    }

    Ok(())
}

/// Delete a tag
async fn delete_tag(name: &str) -> Result<(), GitError> {
    // Check if tag exists
    if !TagRef::exists(name).await? {
        eprintln!("Error: tag '{}' does not exist", name);
        exit(1);
    }

    // Delete tag reference
    TagRef::delete(name).await?;
    println!("Deleted tag '{}'", name);
    
    Ok(())
} 