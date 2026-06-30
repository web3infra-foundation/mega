//! CLA (Contributor License Agreement) operations for [`MonoApiService`](super::service::MonoApiService).

use bytes::Bytes;
use common::errors::MegaError;
use futures::{StreamExt, stream};
use io_orbit::object_storage::{ObjectKey, ObjectMeta, ObjectNamespace};

use crate::{application::api_service::mono::MonoApiService, merge_checker::CheckerRegistry};

const CLA_CONTENT_OBJECT_KEY: &str = "cla/content/current.txt";

impl MonoApiService {
    pub async fn get_or_init_cla_sign_status(
        &self,
        username: &str,
    ) -> Result<(bool, Option<chrono::NaiveDateTime>), MegaError> {
        let model = self
            .storage
            .cla_storage()
            .get_or_create_status(username)
            .await?;
        Ok((model.cla_signed, model.cla_signed_at))
    }

    pub async fn get_cla_content(&self) -> Result<String, MegaError> {
        let key = ObjectKey {
            namespace: ObjectNamespace::Log,
            key: CLA_CONTENT_OBJECT_KEY.to_string(),
        };

        let stream = self
            .storage
            .git_service
            .obj_storage
            .inner
            .get_stream(&key)
            .await;
        let (mut stream, _meta) = match stream {
            Ok(result) => result,
            Err(MegaError::ObjStorageNotFound(_)) => return Ok(String::new()),
            Err(e) => return Err(e),
        };

        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
        }

        String::from_utf8(data).map_err(|e| {
            MegaError::Other(format!(
                "Invalid UTF-8 in CLA content from object storage: {e}"
            ))
        })
    }

    pub async fn update_cla_content(&self, content: &str) -> Result<(), MegaError> {
        let key = ObjectKey {
            namespace: ObjectNamespace::Log,
            key: CLA_CONTENT_OBJECT_KEY.to_string(),
        };

        let bytes = Bytes::from(content.as_bytes().to_vec());
        let stream = stream::once(async move { Ok::<Bytes, std::io::Error>(bytes) });
        let meta = ObjectMeta {
            size: content.len() as i64,
            content_type: Some("text/plain; charset=utf-8".to_string()),
            ..Default::default()
        };

        self.storage
            .git_service
            .obj_storage
            .inner
            .put_stream(&key, Box::pin(stream), meta)
            .await
    }

    pub async fn change_cla_sign_status(
        &self,
        username: &str,
    ) -> Result<(bool, Option<chrono::NaiveDateTime>), MegaError> {
        let model = self.storage.cla_storage().sign(username).await?;
        self.refresh_checks_for_open_cls_by_author(username).await?;
        Ok((model.cla_signed, model.cla_signed_at))
    }

    async fn refresh_checks_for_open_cls_by_author(&self, username: &str) -> Result<(), MegaError> {
        let open_cls = self
            .storage
            .cl_storage()
            .get_open_cls()
            .await?
            .into_iter()
            .filter(|cl| cl.username == username)
            .collect::<Vec<_>>();
        if open_cls.is_empty() {
            return Ok(());
        }

        let check_reg = CheckerRegistry::new(self.storage.clone().into(), username.to_string());
        for cl in open_cls {
            check_reg.run_checks(cl.into()).await?;
        }

        Ok(())
    }
}
