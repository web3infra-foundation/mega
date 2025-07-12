use std::path::Path;
use std::fs;
use std::io::{self, Write};

use async_trait::async_trait;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::tag::Tag;

use crate::utils::util;
use crate::command;

/// Represents a Git tag reference
#[derive(Debug, Clone)]
pub struct TagRef {
    /// Tag name
    pub name: String,
    /// Commit hash the tag points to
    pub commit: SHA1,
}

impl TagRef {
    /// Get the path for a tag reference
    fn get_tag_path(name: &str) -> String {
        format!("{}/refs/tags/{}", util::git_dir(), name)
    }

    /// Create a tag reference
    pub async fn create(name: &str, target: &SHA1) -> Result<(), GitError> {
        let tag_path = Self::get_tag_path(name);
        
        // Ensure parent directory exists
        if let Some(parent) = Path::new(&tag_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write tag reference file
        let mut file = fs::File::create(&tag_path)?;
        file.write_all(target.to_string().as_bytes())?;
        
        Ok(())
    }

    /// Check if a tag exists
    pub async fn exists(name: &str) -> Result<bool, GitError> {
        let tag_path = Self::get_tag_path(name);
        Ok(Path::new(&tag_path).exists())
    }

    /// Delete a tag reference
    pub async fn delete(name: &str) -> Result<(), GitError> {
        let tag_path = Self::get_tag_path(name);
        
        if Path::new(&tag_path).exists() {
            fs::remove_file(&tag_path)?;
            Ok(())
        } else {
            Err(GitError::InvalidReference(format!("Tag '{}' does not exist", name)))
        }
    }

    /// Get a tag reference
    pub async fn get(name: &str) -> Result<TagRef, GitError> {
        let tag_path = Self::get_tag_path(name);
        
        if !Path::new(&tag_path).exists() {
            return Err(GitError::InvalidReference(format!("Tag '{}' does not exist", name)));
        }
        
        let content = fs::read_to_string(&tag_path)?;
        let commit = SHA1::from_str(&content.trim())?;
        
        Ok(TagRef {
            name: name.to_string(),
            commit,
        })
    }

    /// List all tags
    pub async fn list_tags() -> Result<Vec<TagRef>, GitError> {
        let tags_dir = format!("{}/refs/tags", util::git_dir());
        let tags_path = Path::new(&tags_dir);
        
        if !tags_path.exists() {
            return Ok(Vec::new());
        }
        
        let mut tags = Vec::new();
        
        // Read tags from directory
        Self::read_tags_from_dir(tags_path, &mut tags)?;
        
        Ok(tags)
    }
    
    /// Recursively read tags from directory
    fn read_tags_from_dir(dir: &Path, tags: &mut Vec<TagRef>) -> Result<(), GitError> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively read subdirectories
                Self::read_tags_from_dir(&path, tags)?;
            } else {
                let relative_path = path.strip_prefix(format!("{}/refs/tags/", util::git_dir())).unwrap();
                let tag_name = relative_path.to_str().unwrap().to_string();
                
                // Read tag content
                let content = fs::read_to_string(&path)?;
                let commit = SHA1::from_str(&content.trim())?;
                
                tags.push(TagRef {
                    name: tag_name,
                    commit,
                });
            }
        }
        
        Ok(())
    }
    
    /// Get a tag object (annotated tag)
    pub async fn get_tag_object(name: &str) -> Result<Tag, GitError> {
        let tag_ref = Self::get(name).await?;
        
        // Try to load tag object
        let tag = command::load_object::<Tag>(&tag_ref.commit)?;
        
        Ok(tag)
    }
} 