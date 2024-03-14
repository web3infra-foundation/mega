use std::{
    env,
    io::{Cursor, Write},
    path::PathBuf,
    sync::mpsc,
    thread,
};

use async_trait::async_trait;
use bytes::Bytes;

use callisto::{raw_blob, refs};
use common::utils::{generate_id, ZERO_ID};
use jupiter::storage::batch_query_by_columns;
use jupiter::storage::GitStorageProvider;
use mercury::internal::pack::{encode::PackEncoder, Pack};
use venus::{
    errors::GitError,
    internal::{
        object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree},
        pack::{
            entry::Entry,
            reference::{CommandType, RefCommand},
        },
    },
    mr::MergeRequest,
    repo::Repo,
};

use crate::protocol::SmartProtocol;

#[async_trait]
pub trait PackHandler {
    async fn repo_head_object_id(&self, repo: Repo) -> (String, Vec<refs::Model>);

    async fn unpack(&self, repo: &Repo, pack_file: Bytes) -> Result<(), GitError>;

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
    async fn full_pack(&self, repo: &Repo) -> Result<Vec<u8>, GitError>;

    async fn incremental_pack(
        &self,
        want: Vec<String>,
        have: Vec<String>,
    ) -> Result<Vec<u8>, GitError>;

    async fn open_mr(&self) -> MergeRequest;

    async fn convert_path_to_repo(&self) -> Repo;

    async fn update_refs(&self, repo: &Repo, refs: &RefCommand) -> Result<(), GitError>;
}

#[async_trait]
impl PackHandler for SmartProtocol {
    async fn repo_head_object_id(&self, repo: Repo) -> (String, Vec<refs::Model>) {
        let storage = self.context.services.mega_storage.clone();
        let refs = storage.get_repo_refs(&repo).await.unwrap();

        let mut head_hash = ZERO_ID.to_string();
        for git_ref in refs.iter() {
            if git_ref.ref_name == *"refs/heads/main" {
                head_hash = git_ref.ref_git_id.clone();
            }
        }
        (head_hash, refs)
    }

    async fn unpack(&self, repo: &Repo, pack_file: Bytes) -> Result<(), GitError> {
        let mr = self.open_mr().await;

        #[cfg(debug_assertions)]
        {
            let datetime = chrono::Utc::now().naive_utc();
            let path = format!("{}.pack", datetime);
            let mut output = std::fs::File::create(path).unwrap();
            output.write_all(&pack_file).unwrap();
        }

        let (sender, receiver) = mpsc::channel();
        thread::spawn(|| {
            let tmp = PathBuf::from("/tmp/.cache_temp");
            let mut p = Pack::new(None, Some(1024 * 1024 * 1024 * 4), Some(tmp.clone()));
            p.decode(&mut Cursor::new(pack_file), Some(sender)).unwrap();
        });
        let storage = self.context.services.mega_storage.clone();
        let mut entry_list = Vec::new();

        for entry in receiver {
            entry_list.push(entry);
            if entry_list.len() >= 1000 {
                storage.save_entry(&mr, repo, entry_list).await.unwrap();
                entry_list = Vec::new();
            }
        }
        storage.save_entry(&mr, repo, entry_list).await.unwrap();
        Ok(())
    }

    async fn full_pack(&self, repo: &Repo) -> Result<Vec<u8>, GitError> {
        let (sender, receiver) = mpsc::channel();
        let mut writer: Vec<u8> = Vec::new();

        let storage = self.context.services.mega_storage.clone();
        let total = storage.get_obj_count_by_repo_id(repo).await;
        tracing::info!("total: {}", total);
        let mut encoder = PackEncoder::new(total, 0, &mut writer);


        for m in storage
            .get_commits_by_repo_id(repo)
            .await
            .unwrap()
            .into_iter()
        {
            let c: Commit = m.into();
            let entry: Entry = c.into();
            sender.send(entry).unwrap();
        }

        for m in storage
            .get_trees_by_repo_id(repo)
            .await
            .unwrap()
            .into_iter()
        {
            let c: Tree = m.into();
            let entry: Entry = c.into();
            sender.send(entry).unwrap();
        }

        let bids: Vec<String> = storage
            .get_blobs_by_repo_id(repo)
            .await
            .unwrap()
            .into_iter()
            .map(|b| b.blob_id)
            .collect();

        let raw_blobs = batch_query_by_columns::<raw_blob::Entity, raw_blob::Column>(
            storage.get_connection(),
            raw_blob::Column::Sha1,
            bids,
            None,
            None,
        )
        .await
        .unwrap();

        for m in raw_blobs {
            // todo handle storage type
            let c: Blob = m.into();
            let entry: Entry = c.into();
            sender.send(entry).unwrap();
        }

        for m in storage.get_tags_by_repo_id(repo).await.unwrap().into_iter() {
            let c: Tag = m.into();
            let entry: Entry = c.into();
            sender.send(entry).unwrap();
        }
        drop(sender);
        encoder.encode(receiver).unwrap();

        Ok(writer)
    }

    async fn incremental_pack(
        &self,
        _want: Vec<String>,
        _have: Vec<String>,
    ) -> Result<Vec<u8>, GitError> {
        todo!()
    }

    async fn open_mr(&self) -> MergeRequest {
        let mut mr = MergeRequest::default();
        mr.merge(None);
        self.context
            .services
            .mega_storage
            .save_mr(mr.clone())
            .await
            .unwrap();
        mr
    }

    async fn convert_path_to_repo(&self) -> Repo {
        let import_dir = PathBuf::from(env::var("MEGA_IMPORT_DIRS").unwrap());
        let storgae = self.context.services.mega_storage.clone();
        if self.path.starts_with(import_dir) {
            let path_str = self.path.to_str().unwrap();
            let model = storgae.find_git_repo(path_str).await.unwrap();
            if let Some(repo) = model {
                repo.into()
            } else {
                let repo_name = self.path.file_name().unwrap().to_str().unwrap().to_owned();
                let repo = Repo {
                    repo_id: generate_id(),
                    repo_path: self.path.to_str().unwrap().to_owned(),
                    repo_name,
                };
                storgae.save_git_repo(repo.clone()).await.unwrap();
                repo
            }
        } else {
            Repo::empty()
        }
    }

    async fn update_refs(&self, repo: &Repo, refs: &RefCommand) -> Result<(), GitError> {
        let storage = self.context.services.mega_storage.clone();
        match refs.command_type {
            CommandType::Create => {
                storage.save_ref(repo, refs).await.unwrap();
            }
            CommandType::Delete => storage.remove_ref(repo, refs).await.unwrap(),
            CommandType::Update => {
                storage
                    .update_ref(repo, &refs.ref_name, &refs.new_id)
                    .await
                    .unwrap();
            }
        }
        Ok(())
    }
}
