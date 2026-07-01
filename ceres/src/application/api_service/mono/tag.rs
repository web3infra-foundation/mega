//! Tag CRUD and helpers for [`MonoApiService`](super::service::MonoApiService).

use api_model::common::Pagination;
use callisto::{mega_refs, mega_tag};
use git_internal::errors::GitError;
use tracing;

use crate::{
    application::api_service::{
        mono::MonoApiService,
        tag_ops::{
            self, build_git_internal_tag, db_error, format_tagger_info, is_annotated_tag,
            lightweight_commit_tag, merge_paginated_tags, tag_already_exists, tags_full_ref,
            validate_tag_name,
        },
    },
    model::tag::TagInfo,
};

impl MonoApiService {
    pub(crate) fn tag_model_to_info(&self, tag: mega_tag::Model) -> TagInfo {
        TagInfo {
            name: tag.tag_name,
            tag_id: tag.tag_id,
            object_id: tag.object_id,
            object_type: tag.object_type,
            tagger: tag.tagger,
            message: tag.message,
            created_at: tag.created_at.and_utc().to_rfc3339(),
        }
    }

    pub async fn create_tag_impl(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_name: Option<String>,
        tagger_email: Option<String>,
        message: Option<String>,
    ) -> Result<TagInfo, GitError> {
        validate_tag_name(&name)?;
        let mono_storage = self.storage().mono_storage();
        let tagger_info = format_tagger_info(tagger_name, tagger_email);

        self.validate_target_commit_mono(target.as_ref()).await?;

        let full_ref = tags_full_ref(&name);

        match mono_storage.get_tag_by_name(&name).await {
            Ok(Some(_)) => return Err(tag_already_exists(&name).into()),
            Ok(None) => {}
            Err(e) => {
                tracing::error!("DB error while checking tag existence: {}", e);
                return Err(db_error().into());
            }
        }

        if let Ok(Some(_)) = mono_storage.get_ref_by_name(&full_ref).await {
            return Err(tag_already_exists(&name).into());
        }

        if is_annotated_tag(&message) {
            return self
                .create_annotated_tag_mono(repo_path, name, target, tagger_info, message, full_ref)
                .await;
        }

        self.create_lightweight_tag_mono(repo_path, name, target, tagger_info, full_ref)
            .await
    }

    pub async fn list_tags_impl(
        &self,
        repo_path: Option<String>,
        pagination: Pagination,
    ) -> Result<(Vec<TagInfo>, u64), GitError> {
        let mono_storage = self.storage().mono_storage();
        let (annotated_page, annotated_total) =
            match mono_storage.get_tags_by_page(pagination.clone()).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("DB error while listing tags: {}", e);
                    return Err(db_error().into());
                }
            };

        let annotated: Vec<TagInfo> = annotated_page
            .into_iter()
            .map(|t| self.tag_model_to_info(t))
            .collect();

        let repo_path = repo_path.as_deref().unwrap_or("/");
        let mut lightweight_refs = Vec::new();
        if let Ok(refs) = mono_storage.get_all_refs(repo_path, false).await {
            for r in refs {
                if r.ref_name.starts_with("refs/tags/") {
                    let tag_name = r.ref_name.trim_start_matches("refs/tags/").to_string();
                    if annotated.iter().any(|t| t.name == tag_name) {
                        continue;
                    }
                    lightweight_refs.push(lightweight_commit_tag(
                        tag_name,
                        r.ref_commit_hash.clone(),
                        "",
                        r.created_at.and_utc().to_rfc3339(),
                    ));
                }
            }
        }

        Ok(merge_paginated_tags(
            annotated,
            lightweight_refs,
            annotated_total,
            pagination.per_page,
        ))
    }

    pub async fn get_tag_impl(
        &self,
        _repo_path: Option<String>,
        name: String,
    ) -> Result<Option<TagInfo>, GitError> {
        let mono_storage = self.storage().mono_storage();
        match mono_storage.get_tag_by_name(&name).await {
            Ok(Some(tag)) => return Ok(Some(self.tag_model_to_info(tag))),
            Ok(None) => {}
            Err(e) => {
                tracing::error!("DB error while getting tag: {}", e);
                return Err(db_error().into());
            }
        }

        let full_ref = tags_full_ref(&name);
        if let Ok(Some(r)) = mono_storage.get_ref_by_name(&full_ref).await {
            return Ok(Some(lightweight_commit_tag(
                name,
                r.ref_commit_hash.clone(),
                "",
                r.created_at.and_utc().to_rfc3339(),
            )));
        }
        Ok(None)
    }

    pub async fn delete_tag_impl(
        &self,
        _repo_path: Option<String>,
        name: String,
    ) -> Result<(), GitError> {
        let mono_storage = self.storage().mono_storage();
        match mono_storage.get_tag_by_name(&name).await {
            Ok(Some(_tag)) => {
                let full_ref = tags_full_ref(&name);
                if let Ok(Some(r)) = mono_storage.get_ref_by_name(&full_ref).await {
                    mono_storage.remove_ref(r).await.map_err(|e| {
                        tracing::error!("Failed to remove ref while deleting annotated tag: {}", e);
                        GitError::CustomError("[code:500] Failed to remove ref".to_string())
                    })?;
                }
                mono_storage.delete_tag_by_name(&name).await.map_err(|e| {
                    tracing::error!("DB delete error when deleting annotated tag: {}", e);
                    GitError::CustomError("[code:500] DB delete error".to_string())
                })?;
                Ok(())
            }
            Ok(None) => {
                let full_ref = tags_full_ref(&name);
                if let Ok(Some(r)) = mono_storage.get_ref_by_name(&full_ref).await {
                    mono_storage.remove_ref(r).await.map_err(|e| {
                        tracing::error!(
                            "Failed to remove ref while deleting lightweight tag: {}",
                            e
                        );
                        GitError::CustomError("[code:500] Failed to remove ref".to_string())
                    })?;
                    Ok(())
                } else {
                    Err(GitError::CustomError(
                        "[code:404] Tag not found".to_string(),
                    ))
                }
            }
            Err(e) => {
                tracing::error!("DB error while deleting tag: {}", e);
                Err(db_error().into())
            }
        }
    }

    async fn create_annotated_tag_mono(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_info: String,
        message: Option<String>,
        full_ref: String,
    ) -> Result<TagInfo, GitError> {
        let mono_storage = self.storage().mono_storage();

        let (tag_id_hex, object_id) =
            build_git_internal_tag(name.clone(), target, tagger_info.clone(), message.clone())?;
        let tag_model = self.build_mega_tag_model(
            tag_id_hex,
            object_id.clone(),
            name.clone(),
            tagger_info,
            message,
        );

        match mono_storage.insert_tag(tag_model).await {
            Ok(saved_tag) => {
                let path_str = repo_path.unwrap_or_else(|| "/".to_string());
                let tree_hash = self.resolve_tree_hash_for_commit(&object_id).await?;
                let refs = mega_refs::Model::new(&path_str, full_ref, object_id, tree_hash, false);

                if let Err(e) = mono_storage.save_refs(refs, None).await {
                    if let Err(del_e) = mono_storage.delete_tag_by_name(&name).await {
                        tracing::error!(
                            "Failed to rollback tag DB record after ref write failure: {}",
                            del_e
                        );
                    }
                    tracing::error!("Failed to write ref after DB insert: {}", e);
                    return Err(GitError::CustomError(
                        "[code:500] Failed to write ref".to_string(),
                    ));
                }
                Ok(self.tag_model_to_info(saved_tag))
            }
            Err(e) => {
                tracing::error!("DB insert error when creating annotated tag: {}", e);
                Err(GitError::CustomError(
                    "[code:500] DB insert error".to_string(),
                ))
            }
        }
    }

    async fn create_lightweight_tag_mono(
        &self,
        repo_path: Option<String>,
        name: String,
        target: Option<String>,
        tagger_info: String,
        full_ref: String,
    ) -> Result<TagInfo, GitError> {
        let mono_storage = self.storage().mono_storage();

        let path_str = repo_path.unwrap_or_else(|| "/".to_string());
        let object_id = target.unwrap_or_default();
        if object_id.is_empty() {
            return Err(GitError::CustomError(
                "[code:400] Missing target commit for lightweight tag".to_string(),
            ));
        }
        let tree_hash = self.resolve_tree_hash_for_commit(&object_id).await?;

        let refs = mega_refs::Model::new(
            &path_str,
            full_ref.clone(),
            object_id.clone(),
            tree_hash,
            false,
        );
        mono_storage.save_refs(refs, None).await.map_err(|e| {
            tracing::error!("Failed to write lightweight tag ref: {}", e);
            GitError::CustomError("[code:500] Failed to write lightweight tag ref".to_string())
        })?;

        let saved_ref = mono_storage
            .get_ref_by_name(&full_ref)
            .await
            .map_err(|e| GitError::CustomError(e.to_string()))?
            .ok_or_else(|| GitError::CustomError("Ref not found after creation".to_string()))?;

        Ok(lightweight_commit_tag(
            name,
            object_id,
            tagger_info,
            saved_ref.created_at.and_utc().to_rfc3339(),
        ))
    }

    async fn resolve_tree_hash_for_commit(&self, commit_id: &str) -> Result<String, GitError> {
        let mono_storage = self.storage().mono_storage();
        match mono_storage.get_commit_by_hash(commit_id).await {
            Ok(Some(commit_model)) => Ok(commit_model.tree.clone()),
            Ok(None) => {
                tracing::error!(
                    "Target commit '{}' not found while resolving tree hash",
                    commit_id
                );
                Err(tag_ops::commit_not_found(commit_id).into())
            }
            Err(e) => {
                tracing::error!(
                    "DB error fetching commit '{}' for tree hash resolution: {}",
                    commit_id,
                    e
                );
                Err(db_error().into())
            }
        }
    }

    async fn validate_target_commit_mono(&self, target: Option<&String>) -> Result<(), GitError> {
        let mono_storage = self.storage().mono_storage();
        if let Some(t) = target {
            match mono_storage.get_commit_by_hash(t).await {
                Ok(commit_opt) => {
                    if commit_opt.is_none() {
                        return Err(tag_ops::commit_not_found(t).into());
                    }
                }
                Err(e) => {
                    tracing::error!("DB error while fetching commit by hash: {}", e);
                    return Err(db_error().into());
                }
            }
        }
        Ok(())
    }

    fn build_mega_tag_model(
        &self,
        tag_id_hex: String,
        object_id: String,
        name: String,
        tagger_info: String,
        message: Option<String>,
    ) -> mega_tag::Model {
        mega_tag::Model {
            id: common::utils::generate_id(),
            tag_id: tag_id_hex,
            object_id,
            object_type: "commit".to_string(),
            tag_name: name,
            tagger: tagger_info,
            message: message.unwrap_or_default(),
            pack_id: String::new(),
            pack_offset: 0,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}
