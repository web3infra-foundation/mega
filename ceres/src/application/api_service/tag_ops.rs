//! Shared tag helpers used by mono and import API services.

use std::str::FromStr;

use git_internal::{
    errors::GitError,
    hash::ObjectHash,
    internal::object::{
        signature::{Signature, SignatureType},
        tag::Tag,
        types::ObjectType,
    },
};

use crate::model::tag::TagInfo;

pub fn format_tagger_info(tagger_name: Option<String>, tagger_email: Option<String>) -> String {
    match (tagger_name, tagger_email) {
        (Some(n), Some(e)) => format!("{n} <{e}>"),
        (Some(n), None) => n,
        (None, Some(e)) => e,
        (None, None) => "unknown".to_string(),
    }
}

pub fn is_annotated_tag(message: &Option<String>) -> bool {
    message.as_ref().is_some_and(|s| !s.is_empty())
}

pub fn tags_full_ref(name: &str) -> String {
    format!("refs/tags/{name}")
}

pub fn tag_already_exists(name: &str) -> GitError {
    GitError::CustomError(format!("[code:400] Tag '{name}' already exists"))
}

pub fn commit_not_found(commit_id: &str) -> GitError {
    GitError::CustomError(format!("[code:404] Target commit '{commit_id}' not found"))
}

pub fn db_error() -> GitError {
    GitError::CustomError("[code:500] DB error".to_string())
}

/// Build git-internal tag id and resolved object id from create-tag inputs.
pub fn build_git_internal_tag(
    name: String,
    target: Option<String>,
    tagger_info: String,
    message: Option<String>,
) -> Result<(String, String), GitError> {
    let tag_target = target
        .as_ref()
        .ok_or(GitError::InvalidCommitObject)
        .and_then(|t| ObjectHash::from_str(t).map_err(|_| GitError::InvalidCommitObject))?;
    let tagger_sig = Signature::new(SignatureType::Tagger, tagger_info, String::new());
    let git_internal_tag = Tag::new(
        tag_target,
        ObjectType::Commit,
        name,
        tagger_sig,
        message.unwrap_or_default(),
    );
    Ok((
        git_internal_tag.id.to_string(),
        target.unwrap_or_else(|| "HEAD".to_string()),
    ))
}

pub fn lightweight_commit_tag(
    name: impl Into<String>,
    object_id: impl Into<String>,
    tagger_info: impl Into<String>,
    created_at: impl Into<String>,
) -> TagInfo {
    let name = name.into();
    let object_id = object_id.into();
    TagInfo {
        tag_id: object_id.clone(),
        object_id,
        name,
        object_type: "commit".to_string(),
        tagger: tagger_info.into(),
        message: String::new(),
        created_at: created_at.into(),
    }
}

/// Merge annotated tag page with lightweight refs for list_tags pagination.
pub fn merge_paginated_tags(
    mut annotated: Vec<TagInfo>,
    lightweight_refs: Vec<TagInfo>,
    annotated_total: u64,
    per_page: u64,
) -> (Vec<TagInfo>, u64) {
    let per_page = if per_page == 0 { 20 } else { per_page as usize };
    let total = annotated_total + lightweight_refs.len() as u64;
    if annotated.len() < per_page {
        let need = per_page - annotated.len();
        for item in lightweight_refs.into_iter().take(need) {
            annotated.push(item);
        }
    }
    (annotated, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tagger_info_formats_email() {
        assert_eq!(
            format_tagger_info(Some("Alice".into()), Some("a@b.c".into())),
            "Alice <a@b.c>"
        );
    }

    #[test]
    fn is_annotated_tag_requires_non_empty_message() {
        assert!(!is_annotated_tag(&None));
        assert!(!is_annotated_tag(&Some(String::new())));
        assert!(is_annotated_tag(&Some("release".into())));
    }

    #[test]
    fn merge_paginated_tags_fills_from_lightweight() {
        let annotated = vec![lightweight_commit_tag("a", "1", "", "t")];
        let lightweight = vec![
            lightweight_commit_tag("b", "2", "", "t"),
            lightweight_commit_tag("c", "3", "", "t"),
        ];
        let (page, total) = merge_paginated_tags(annotated, lightweight, 1, 2);
        assert_eq!(page.len(), 2);
        assert_eq!(total, 3);
    }
}
