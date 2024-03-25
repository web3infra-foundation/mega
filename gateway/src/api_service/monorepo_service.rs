use std::sync::Arc;

use common::errors::MegaError;
use ganymede::model::create_file::CreateFileInfo;
use jupiter::storage::mega_storage::MegaStorage;

use crate::model::mr::{MergeOperation, MergeResult};

#[derive(Clone)]
pub struct MonorepoService {
    pub storage: Arc<MegaStorage>,
}

impl MonorepoService {
    pub async fn init_monorepo(&self) {
        self.storage.init_monorepo().await
    }

    pub async fn create_mega_file(&self, file_info: CreateFileInfo) -> Result<(), MegaError> {
        self.storage.create_mega_file(file_info).await
    }

    pub async fn merge_mr(&self, op: MergeOperation) -> Result<MergeResult, MegaError> {
        let mut res = MergeResult {
            result: true,
            err_message: "".to_owned(),
        };
        if let Some(mut mr) = self.storage.get_open_mr_by_id(op.mr_id).await.unwrap() {
            //check from_hash
            let refs = self.storage.get_ref(&mr.path).await.unwrap();

            if mr.from_hash == refs[0].ref_hash {
                mr.merge(op.message);
                self.storage.update_mr(mr).await;
            } else {
                res.result = false;
                res.err_message = "ref hash conflict".to_owned();
            }
        } else {
            res.result = false;
            res.err_message = "Invalid mr id".to_owned();
        }
        Ok(res)
    }
}
