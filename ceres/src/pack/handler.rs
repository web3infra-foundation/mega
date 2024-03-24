use std::{
    env,
    io::{Cursor, Write},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

use async_trait::async_trait;
use bytes::Bytes;

use common::utils::{generate_id, MEGA_BRANCH_NAME, ZERO_ID};
use mercury::internal::pack::Pack;
use venus::{
    errors::GitError,
    internal::pack::{
        entry::Entry,
        reference::{RefCommand, Refs},
    },
    repo::Repo,
};

use crate::pack::{import_repo::ImportRepo, monorepo::MonoRepo};
use crate::protocol::SmartProtocol;

#[async_trait]
pub trait PackHandler: Send + Sync {
    async fn head_hash(&self) -> (String, Vec<Refs>);

    async fn unpack(&self, pack_file: Bytes) -> Result<(), GitError>;

    /// Asynchronously retrieves the full pack data for the specified repository path.
    /// This function collects commits and nodes from the storage and packs them into
    /// a single binary vector. There is no need to build the entire tree; the function
    /// only sends all the data related to this repository.
    ///
    /// # Arguments
    /// * `repo_path` - The path to the repository.
    ///
    /// # Returns
    /// * `Result<Vec<u8>, GitError>` - The packed binary data as a vector of bytes.
    ///
    async fn full_pack(&self) -> Result<Vec<u8>, GitError>;

    async fn check_commit_exist(&self, hash: &str) -> bool;

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<Vec<u8>, GitError>;

    async fn update_refs(&self, refs: &RefCommand) -> Result<(), GitError>;
}

impl SmartProtocol {
    pub async fn pack_handler(&self) -> Box<dyn PackHandler> {
        let import_dir = PathBuf::from(env::var("MEGA_IMPORT_DIRS").unwrap());
        let storage = self.context.services.mega_storage.clone();
        if self.path.starts_with(import_dir.clone()) && self.path != import_dir {
            let path_str = self.path.to_str().unwrap();
            let model = storage.find_git_repo(path_str).await.unwrap();
            let repo = if let Some(repo) = model {
                repo.into()
            } else {
                let repo_name = self.path.file_name().unwrap().to_str().unwrap().to_owned();
                let repo = Repo {
                    repo_id: generate_id(),
                    repo_path: self.path.to_str().unwrap().to_owned(),
                    repo_name,
                };
                storage.save_git_repo(repo.clone()).await.unwrap();
                repo
            };
            Box::new(ImportRepo {
                context: self.context.clone(),
                repo,
            })
        } else {
            Box::new(MonoRepo {
                context: self.context.clone(),
                path: self.path.clone(),
            })
        }
    }
}

pub fn check_head_hash(refs: Vec<Refs>) -> (String, Vec<Refs>) {
    let mut head_hash = ZERO_ID.to_string();
    for git_ref in refs.iter() {
        if git_ref.ref_name == *MEGA_BRANCH_NAME {
            head_hash = git_ref.ref_hash.clone();
        }
    }
    (head_hash, refs)
}

pub fn decode_for_receiver(pack_file: Bytes) -> Result<Receiver<Entry>, GitError> {
    #[cfg(debug_assertions)]
    {
        let datetime = chrono::Utc::now().naive_utc();
        let path = format!("{}.pack", datetime);
        let mut output = std::fs::File::create(path).unwrap();
        output.write_all(&pack_file).unwrap();
    }

    let (sender, receiver) = mpsc::channel();
    let tmp = PathBuf::from("/tmp/.cache_temp");
    let p = Pack::new(None, Some(1024 * 1024 * 1024 * 4), Some(tmp.clone()));
    p.decode_async(Cursor::new(pack_file), sender); //Pack moved here
    Ok(receiver)
}
