use std::path::{Path as StdPath, PathBuf};

use common::errors::MegaError;

use super::service::MonoApiService;
use crate::model::commit::CommitBindingResponse;

impl MonoApiService {
    pub async fn upsert_commit_binding(
        &self,
        sha: &str,
        username: Option<String>,
        is_anonymous: bool,
    ) -> Result<CommitBindingResponse, MegaError> {
        let final_username = if is_anonymous {
            None
        } else {
            username.and_then(|u| {
                let t = u.trim();
                if t.is_empty() || t.eq_ignore_ascii_case("anonymous") {
                    None
                } else {
                    Some(t.to_string())
                }
            })
        };

        self.storage
            .commit_binding_storage()
            .upsert_binding(sha, final_username.clone(), final_username.is_none())
            .await?;

        Ok(CommitBindingResponse {
            username: final_username,
        })
    }

    pub fn import_dir(&self) -> PathBuf {
        self.storage.config().monorepo.import_dir.clone()
    }

    pub async fn find_git_repo_like_path(
        &self,
        path: &str,
    ) -> Result<Option<callisto::git_repo::Model>, MegaError> {
        self.storage
            .git_db_storage()
            .find_git_repo_like_path(path)
            .await
    }

    pub async fn resolve_target_commit_id(
        &self,
        path_context: Option<&str>,
        target_opt: Option<&str>,
    ) -> Result<String, MegaError> {
        if let Some(t) = target_opt
            && t != "HEAD"
            && !t.is_empty()
        {
            return Ok(t.to_string());
        }

        let import_dir = self.import_dir();
        if let Some(path) = path_context {
            let std_path = StdPath::new(path);
            if std_path.starts_with(&import_dir) && std_path != StdPath::new(&import_dir) {
                if let Some(repo_model) = self.find_git_repo_like_path(path).await? {
                    let git = self.storage.git_db_storage();
                    if let Ok(Some(r)) = git.get_default_ref(repo_model.id).await {
                        return Ok(r.ref_git_id);
                    }
                    if let Ok(refs) = git.get_ref(repo_model.id).await
                        && let Some(r) = refs.into_iter().next()
                    {
                        return Ok(r.ref_git_id);
                    }
                    return Ok("HEAD".to_string());
                }
            } else {
                let mono = self.storage.mono_storage();
                let resolved_path = path_context.unwrap_or("/");
                if let Ok(Some(r)) = mono.get_main_ref(resolved_path).await {
                    return Ok(r.ref_commit_hash);
                }
                if let Ok(Some(root_ref)) = mono.get_main_ref("/").await {
                    return Ok(root_ref.ref_commit_hash);
                }
                return Ok("HEAD".to_string());
            }
        }

        let mono = self.storage.mono_storage();
        if let Ok(Some(root_ref)) = mono.get_main_ref("/").await {
            return Ok(root_ref.ref_commit_hash);
        }
        Ok("HEAD".to_string())
    }
}
