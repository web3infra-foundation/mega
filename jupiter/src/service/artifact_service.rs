use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use api_model::artifacts::{
    ARTIFACT_PRESIGN_URL_TTL_SECS, ArtifactActions, ArtifactBatchHints,
    ArtifactBatchObjectResponse, ArtifactBatchRequest, ArtifactBatchResponse,
    ArtifactCommitRequest, ArtifactCommitResponse, ArtifactDiscoveryResponse,
    ArtifactDiscoveryTransfers, ArtifactFileDescriptor, ArtifactIntent, ArtifactLink,
    ArtifactListSetsResponse, ArtifactObjectDescriptor, ArtifactObjectType,
    ArtifactResolveFileResponse, ArtifactSetDetailResponse, ArtifactSetListItem,
    DEFAULT_DISCOVERY_MAX_CONCURRENCY, DEFAULT_MAX_COMMIT_FILES, DEFAULT_MAX_OBJECT_SIZE_BYTES,
    DEFAULT_MAX_OBJECTS_PER_BATCH, DEFAULT_MULTIPART_THRESHOLD_BYTES, GetArtifactSetQuery,
    ListArtifactSetsQuery, ResolveArtifactFileQuery, build_artifact_discovery_response,
};
use callisto::{artifact_objects, artifact_set_files, artifact_sets};
use chrono::Utc;
use common::errors::MegaError;
use idgenerator::IdInstance;
use io_orbit::{
    factory::MegaObjectStorageWrapper,
    object_storage::{ObjectByteStream, ObjectKey, ObjectMeta, ObjectNamespace},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, TransactionTrait};
use uuid::Uuid;

use crate::{
    storage::{
        artifact_storage::{ArtifactSetsPageQuery, ArtifactStorage},
        base_storage::{BaseStorage, StorageConnector},
    },
    utils::{id_generator, into_obj_stream::IntoObjectStream},
};

/// Result of one blob GC pass (`docs/artifacts-protocol.md` §10.6).
#[derive(Debug, Default, Clone, Copy)]
pub struct ArtifactObjectGcStats {
    pub candidates: u32,
    pub deleted: u32,
    pub skipped_still_referenced: u32,
    pub storage_delete_errors: u32,
    pub db_delete_errors: u32,
}

#[derive(Clone)]
pub struct ArtifactService {
    st: ArtifactStorage,
    obj_storage: MegaObjectStorageWrapper,
}

impl ArtifactService {
    pub fn new(base: BaseStorage, obj_storage: MegaObjectStorageWrapper) -> Self {
        Self {
            st: ArtifactStorage { base },
            obj_storage,
        }
    }

    pub fn mock() -> Self {
        Self::new(BaseStorage::mock(), MegaObjectStorageWrapper::mock())
    }

    /// §8.2 discovery: `transfers.*` reflects this process's object-store backend.
    pub fn discovery_response(&self) -> ArtifactDiscoveryResponse {
        let transfers = ArtifactDiscoveryTransfers {
            signed_url_put: self.obj_storage.supports_presigned_urls(),
            server_fallback_put: true,
            signed_url_get: self.obj_storage.supports_presigned_urls(),
            server_proxy_get: true,
        };
        build_artifact_discovery_response(transfers)
    }

    pub fn supports_artifact_presigned_urls(&self) -> bool {
        self.obj_storage.supports_presigned_urls()
    }

    /// Weak ETag for conditional GET (`If-None-Match`) / HEAD.
    pub fn weak_etag_for_oid_size(oid: &str, size_bytes: i64) -> String {
        format!(r#"W/"artifact-{oid}-{size_bytes}""#)
    }

    /// Parse a single `Range: bytes=...` header for an object of `len` bytes. Returns
    /// `(start, end_exclusive)` for [`MegaObjectStorage::get_range_stream`], or `None` for full body.
    pub fn parse_artifact_object_range(
        range_header_value: Option<&str>,
        len: u64,
    ) -> Result<Option<(u64, u64)>, MegaError> {
        let Some(raw) = range_header_value.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        if len == 0 {
            return Ok(None);
        }
        let lower = raw.to_ascii_lowercase();
        let prefix = "bytes=";
        if !lower.starts_with(prefix) {
            return Err(MegaError::Other(
                "[code:400] unsupported Range unit (only bytes=)".to_string(),
            ));
        }
        let spec = raw[prefix.len()..].trim();
        if spec.contains(',') {
            return Err(MegaError::Other(
                "[code:400] multiple byte ranges not supported".to_string(),
            ));
        }
        let (a, b) = spec
            .split_once('-')
            .ok_or_else(|| MegaError::Other("[code:400] invalid Range syntax".to_string()))?;
        let (start, end_inclusive): (u64, u64) = if a.is_empty() {
            let suffix_len: u64 = b.parse().map_err(|_| {
                MegaError::Other("[code:400] invalid suffix byte range".to_string())
            })?;
            if suffix_len == 0 || suffix_len > len {
                return Err(MegaError::Other(
                    "[code:416] unsatisfiable range".to_string(),
                ));
            }
            let start = len - suffix_len;
            (start, len - 1)
        } else {
            let start: u64 = a
                .parse()
                .map_err(|_| MegaError::Other("[code:400] invalid range start".to_string()))?;
            let end_inclusive: u64 = if b.is_empty() {
                len - 1
            } else {
                b.parse()
                    .map_err(|_| MegaError::Other("[code:400] invalid range end".to_string()))?
            };
            (start, end_inclusive)
        };
        if start > end_inclusive || start >= len {
            return Err(MegaError::Other(
                "[code:416] unsatisfiable range".to_string(),
            ));
        }
        let end_inclusive = end_inclusive.min(len - 1);
        let end_exclusive = end_inclusive.saturating_add(1);
        Ok(Some((start, end_exclusive)))
    }

    fn artifact_object_key(oid: &str) -> ObjectKey {
        ObjectKey {
            namespace: ObjectNamespace::Artifact,
            key: oid.to_string(),
        }
    }

    /// Presigned GET when the configured backend supports it; [`None`] for local filesystem.
    pub async fn artifact_object_signed_get_url(
        &self,
        oid: &str,
        expires_in: Duration,
    ) -> Result<Option<String>, MegaError> {
        Self::validate_uuid_oid(oid)?;
        let key = Self::artifact_object_key(oid);
        self.obj_storage
            .inner
            .signed_url(&key, reqwest::Method::GET, expires_in)
            .await
    }

    pub async fn artifact_object_signed_put_url(
        &self,
        oid: &str,
        expires_in: Duration,
    ) -> Result<Option<String>, MegaError> {
        Self::validate_uuid_oid(oid)?;
        if !self.obj_storage.supports_presigned_urls() {
            return Ok(None);
        }
        let key = Self::artifact_object_key(oid);
        self.obj_storage
            .inner
            .signed_url(&key, reqwest::Method::PUT, expires_in)
            .await
    }

    pub async fn get_artifact_object_range_byte_stream(
        &self,
        oid: &str,
        start: u64,
        end_exclusive: u64,
    ) -> Result<ObjectByteStream, MegaError> {
        Self::validate_uuid_oid(oid)?;
        let key = Self::artifact_object_key(oid);
        let (stream, _meta) = self
            .obj_storage
            .inner
            .get_range_stream(&key, start, Some(end_exclusive))
            .await?;
        Ok(stream)
    }

    /// DB row for `oid` when it is referenced by a committed manifest in `repo` (protocol §8.7.4).
    pub async fn artifact_object_model_for_committed_repo_download(
        &self,
        repo: &str,
        oid: &str,
    ) -> Result<artifact_objects::Model, MegaError> {
        Self::validate_uuid_oid(oid)?;
        if !self.st.artifact_oid_committed_in_repo(repo, oid).await? {
            return Err(MegaError::Other(
                "[code:404] artifact object not found in this repository".to_string(),
            ));
        }
        self.st
            .find_artifact_object_by_oid(oid)
            .await?
            .ok_or_else(|| {
                MegaError::Other("[code:404] artifact object metadata missing".to_string())
            })
    }

    /// Raw object-store stream for `oid` (caller enforces access rules).
    pub async fn get_artifact_object_byte_stream(
        &self,
        oid: &str,
    ) -> Result<ObjectByteStream, MegaError> {
        Self::validate_uuid_oid(oid)?;
        let key = Self::artifact_object_key(oid);
        let (stream, _meta) = self.obj_storage.inner.get_stream(&key).await?;
        Ok(stream)
    }

    /// Fallback `PUT .../objects/{oid}`: write bytes to object storage and register `artifact_objects`.
    pub async fn upload_artifact_object_bytes(
        &self,
        oid: &str,
        bytes: Vec<u8>,
    ) -> Result<(), MegaError> {
        Self::validate_uuid_oid(oid)?;
        let size_bytes = bytes.len() as i64;
        if size_bytes as u64 > DEFAULT_MAX_OBJECT_SIZE_BYTES {
            return Err(MegaError::Other(format!(
                "[code:413] object exceeds max_object_size_bytes ({DEFAULT_MAX_OBJECT_SIZE_BYTES})"
            )));
        }
        let key = Self::artifact_object_key(oid);
        let existing = self.st.find_artifact_object_by_oid(oid).await?;
        if let Some(ref row) = existing
            && row.size_bytes != size_bytes
        {
            return Err(MegaError::Other(
                "[code:409] oid already exists with a different size".to_string(),
            ));
        }

        self.obj_storage
            .inner
            .put_stream(
                &key,
                bytes.into_stream(),
                ObjectMeta {
                    size: size_bytes,
                    ..Default::default()
                },
            )
            .await?;

        if existing.is_some() {
            return Ok(());
        }

        let now = Utc::now().naive_utc();
        let storage_key = key.default_sharding();
        let am = artifact_objects::ActiveModel {
            oid: Set(oid.to_string()),
            size_bytes: Set(size_bytes),
            content_type: Set(None),
            storage_key: Set(storage_key),
            created_at: Set(now),
            last_seen_at: Set(now),
            integrity: Set(None),
        };

        if let Err(e) = am.insert(self.st.get_connection()).await {
            match self.st.find_artifact_object_by_oid(oid).await? {
                Some(row) if row.size_bytes == size_bytes => {}
                Some(_) => {
                    return Err(MegaError::Other(
                        "[code:409] oid already exists with a different size".to_string(),
                    ));
                }
                None => return Err(MegaError::Db(e)),
            }
        }

        Ok(())
    }

    /// `GET .../artifact_sets` pagination: `cursor` query and JSON `next_cursor`.
    ///
    /// **Wire format (v1 only):** `asets-v1|<created_at_unix_micros>|<artifact_sets.id>`
    ///
    /// - `asets-v1` — literal prefix (artifact sets list, version 1).
    /// - `created_at_unix_micros` — `artifact_sets.created_at` as Unix time in microseconds (UTC),
    ///   matching the value produced by `encode_set_cursor`.
    /// - `artifact_sets.id` — row primary key; combined with timestamp for a stable keyset page
    ///   under `(created_at DESC, id DESC)`.
    ///
    /// Clients should treat the string as opaque and pass `next_cursor` back verbatim as `cursor`.
    fn decode_set_cursor(s: &str) -> Result<(chrono::NaiveDateTime, i64), MegaError> {
        const V1: &str = "asets-v1|";
        let rest = s.strip_prefix(V1).ok_or_else(|| {
            MegaError::Other(
                "[code:400] invalid cursor: expected \"asets-v1|<unix_us>|<set_id>\"".to_string(),
            )
        })?;
        let (ts, id) = rest.split_once('|').ok_or_else(|| {
            MegaError::Other(
                "[code:400] invalid cursor: expected \"asets-v1|<unix_us>|<set_id>\"".to_string(),
            )
        })?;
        let ts: i64 = ts.parse().map_err(|_| {
            MegaError::Other("[code:400] invalid cursor: bad unix_us component".to_string())
        })?;
        let id: i64 = id.parse().map_err(|_| {
            MegaError::Other("[code:400] invalid cursor: bad set_id component".to_string())
        })?;
        let dt = chrono::DateTime::from_timestamp_micros(ts)
            .ok_or_else(|| {
                MegaError::Other("[code:400] invalid cursor: timestamp out of range".to_string())
            })?
            .naive_utc();
        Ok((dt, id))
    }

    fn encode_set_cursor(created_at: chrono::NaiveDateTime, id: i64) -> String {
        let ts = created_at.and_utc().timestamp_micros();
        format!("asets-v1|{ts}|{id}")
    }

    fn naive_to_rfc3339_utc(dt: chrono::NaiveDateTime) -> String {
        chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    fn validate_logical_path(path: &str) -> Result<(), MegaError> {
        if path.is_empty() {
            return Err(MegaError::Other(
                "[code:400] path must not be empty".to_string(),
            ));
        }
        if path.starts_with('/') {
            return Err(MegaError::Other(
                "[code:400] path must be relative (no leading '/')".to_string(),
            ));
        }
        if path.contains("..") {
            return Err(MegaError::Other(
                "[code:400] path must not contain '..'".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_uuid_oid(oid: &str) -> Result<(), MegaError> {
        Uuid::parse_str(oid).map_err(|_| {
            MegaError::Other(format!(
                "[code:400] oid must be a UUID string (RFC 4122): {oid:?}"
            ))
        })?;
        Ok(())
    }

    fn sorted_manifest_pairs(
        files: &[ArtifactFileDescriptor],
    ) -> Result<Vec<(String, String, i64)>, MegaError> {
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(files.len());
        for f in files {
            Self::validate_logical_path(&f.path)?;
            Self::validate_uuid_oid(&f.oid)?;
            if !seen.insert(f.path.clone()) {
                return Err(MegaError::Other(format!(
                    "[code:400] duplicate path in commit manifest: {}",
                    f.path
                )));
            }
            out.push((f.path.clone(), f.oid.clone(), f.size));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    fn committed_files_match_request(
        rows: &[artifact_set_files::Model],
        want: &[(String, String, i64)],
    ) -> bool {
        if rows.len() != want.len() {
            return false;
        }
        let mut got: Vec<(String, String, i64)> = rows
            .iter()
            .map(|r| (r.path.clone(), r.oid.clone(), r.size_bytes))
            .collect();
        got.sort_by(|a, b| a.0.cmp(&b.0));
        got == *want
    }

    pub async fn list_artifact_sets(
        &self,
        repo: &str,
        q: &ListArtifactSetsQuery,
    ) -> Result<ArtifactListSetsResponse, MegaError> {
        let limit = q.limit.unwrap_or(50).clamp(1, 200);
        let cursor = q
            .cursor
            .as_deref()
            .map(Self::decode_set_cursor)
            .transpose()?;

        let mut models = self
            .st
            .find_artifact_sets_page(ArtifactSetsPageQuery {
                repo,
                namespace: &q.namespace,
                object_type: q.object_type.as_label(),
                run_id: q.run_id.as_deref(),
                commit_sha: q.commit_sha.as_deref(),
                cursor_before: cursor,
                limit_plus_one: limit as u64 + 1,
            })
            .await?;

        let has_next = models.len() > limit as usize;
        if has_next {
            models.truncate(limit as usize);
        }
        let next_cursor = if has_next {
            models
                .last()
                .map(|m| Self::encode_set_cursor(m.created_at, m.id))
        } else {
            None
        };

        let set_ids: Vec<i64> = models.iter().map(|m| m.id).collect();
        let file_rows = self.st.list_artifact_set_files_by_set_ids(&set_ids).await?;
        let mut file_counts = std::collections::HashMap::new();
        for f in file_rows {
            *file_counts.entry(f.set_id).or_insert(0) += 1;
        }

        let sets = models
            .into_iter()
            .map(|m| {
                let object_type =
                    ArtifactObjectType::from_label(&m.object_type).unwrap_or(q.object_type);
                ArtifactSetListItem {
                    artifact_set_id: m.artifact_set_id,
                    namespace: m.namespace,
                    object_type,
                    metadata: m.metadata.clone(),
                    created_at: Self::naive_to_rfc3339_utc(m.created_at),
                    expires_at: m.expires_at.map(Self::naive_to_rfc3339_utc),
                    file_count: file_counts.get(&m.id).copied(),
                }
            })
            .collect();

        Ok(ArtifactListSetsResponse { sets, next_cursor })
    }

    pub async fn get_artifact_set_detail(
        &self,
        repo: &str,
        artifact_set_id: &str,
        q: &GetArtifactSetQuery,
    ) -> Result<ArtifactSetDetailResponse, MegaError> {
        let set = self
            .st
            .find_artifact_set_by_natural_key(
                repo,
                &q.namespace,
                q.object_type.as_label(),
                artifact_set_id,
            )
            .await?
            .ok_or_else(|| {
                MegaError::NotFound(
                    "artifact set not found for (repo, namespace, object_type, artifact_set_id)"
                        .to_string(),
                )
            })?;

        let files_models = self.st.list_artifact_set_files(set.id).await?;
        let files: Vec<ArtifactObjectDescriptor> = files_models
            .into_iter()
            .map(|f| ArtifactObjectDescriptor {
                path: f.path,
                oid: f.oid,
                size: f.size_bytes,
                content_type: f.content_type,
            })
            .collect();

        let object_type = ArtifactObjectType::from_label(&set.object_type).unwrap_or(q.object_type);

        Ok(ArtifactSetDetailResponse {
            artifact_set_id: set.artifact_set_id,
            namespace: set.namespace,
            object_type,
            metadata: set.metadata.clone(),
            created_at: Self::naive_to_rfc3339_utc(set.created_at),
            expires_at: set.expires_at.map(Self::naive_to_rfc3339_utc),
            files,
        })
    }

    pub async fn resolve_artifact_file(
        &self,
        repo: &str,
        q: &ResolveArtifactFileQuery,
    ) -> Result<ArtifactResolveFileResponse, MegaError> {
        let (file, set) = self
            .st
            .find_latest_artifact_file_for_path(
                repo,
                &q.namespace,
                q.object_type.as_label(),
                &q.path,
                q.run_id.as_deref(),
                q.commit_sha.as_deref(),
            )
            .await?
            .ok_or_else(|| {
                MegaError::NotFound("no committed artifact file matches the query".to_string())
            })?;

        Ok(ArtifactResolveFileResponse {
            artifact_set_id: set.artifact_set_id,
            path: file.path,
            oid: file.oid,
            size: file.size_bytes,
            content_type: file.content_type,
            committed_at: Self::naive_to_rfc3339_utc(set.created_at),
        })
    }

    /// `POST .../batch` — negotiate uploads (`docs/artifacts-protocol.md` §8.4).
    ///
    /// `exists=true` only when DB metadata matches the requested size **and** the blob exists in object storage.
    /// Refreshes `last_seen_at` for those `oid`s.
    ///
    /// When the backend supports presigned URLs, `exists=false` entries include `actions.upload`;
    /// otherwise clients use the Mono fallback `PUT .../artifacts/objects/{oid}`.
    pub async fn batch_artifacts(
        &self,
        req: &ArtifactBatchRequest,
    ) -> Result<ArtifactBatchResponse, MegaError> {
        if req.namespace.trim().is_empty() {
            return Err(MegaError::Other(
                "[code:400] namespace must not be empty".to_string(),
            ));
        }
        if !matches!(req.intent, ArtifactIntent::Upload) {
            return Err(MegaError::Other(
                "[code:400] only intent \"upload\" is supported".to_string(),
            ));
        }
        let max = DEFAULT_MAX_OBJECTS_PER_BATCH as usize;
        if req.objects.len() > max {
            return Err(MegaError::Other(format!(
                "[code:400] batch exceeds max_objects_per_batch ({max})"
            )));
        }

        for o in &req.objects {
            Self::validate_logical_path(&o.path)?;
            Self::validate_uuid_oid(&o.oid)?;
            if o.size < 0 {
                return Err(MegaError::Other(
                    "[code:400] object size must be non-negative".to_string(),
                ));
            }
            if o.size as u64 > DEFAULT_MAX_OBJECT_SIZE_BYTES {
                return Err(MegaError::Other(format!(
                    "[code:400] object {} exceeds max_object_size_bytes ({})",
                    o.oid, DEFAULT_MAX_OBJECT_SIZE_BYTES
                )));
            }
        }

        let oids: Vec<String> = req.objects.iter().map(|o| o.oid.clone()).collect();
        let existing = self.st.find_artifact_objects_by_oids(&oids).await?;
        let by_oid: HashMap<String, i64> = existing
            .into_iter()
            .map(|m| (m.oid, m.size_bytes))
            .collect();

        let ttl = Duration::from_secs(ARTIFACT_PRESIGN_URL_TTL_SECS);
        let mut objects = Vec::with_capacity(req.objects.len());
        let mut touch_oids: HashSet<String> = HashSet::new();
        for o in &req.objects {
            let exists = match by_oid.get(&o.oid) {
                None => false,
                Some(db_size) if *db_size != o.size => {
                    return Err(MegaError::Other(format!(
                        "[code:400] oid {} exists with a different size than requested",
                        o.oid
                    )));
                }
                Some(_) => {
                    let key = Self::artifact_object_key(&o.oid);
                    let in_store = self.obj_storage.inner.exists(&key).await?;
                    if in_store {
                        touch_oids.insert(o.oid.clone());
                    }
                    in_store
                }
            };
            let mut actions: Option<ArtifactActions> = None;
            if !exists && self.obj_storage.supports_presigned_urls() {
                match self.artifact_object_signed_put_url(&o.oid, ttl).await {
                    Ok(Some(href)) => {
                        let expires_at = (Utc::now()
                            + chrono::Duration::seconds(ARTIFACT_PRESIGN_URL_TTL_SECS as i64))
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        let mut header = HashMap::new();
                        header.insert(
                            "Content-Type".to_string(),
                            o.content_type
                                .clone()
                                .unwrap_or_else(|| "application/octet-stream".to_string()),
                        );
                        actions = Some(ArtifactActions {
                            upload: Some(ArtifactLink {
                                href,
                                header: Some(header),
                                expires_at: Some(expires_at),
                            }),
                        });
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(
                            oid = %o.oid,
                            error = %e,
                            "artifact batch: presigned PUT failed; client may use fallback PUT"
                        );
                    }
                }
            }
            let entry = ArtifactBatchObjectResponse {
                oid: o.oid.clone(),
                size: o.size,
                exists,
                actions,
            };
            objects.push(entry);
        }

        if !touch_oids.is_empty() {
            let touch: Vec<String> = touch_oids.into_iter().collect();
            self.st.touch_artifact_objects_last_seen_at(&touch).await?;
        }

        Ok(ArtifactBatchResponse {
            transfer: "basic".to_string(),
            objects,
            hints: Some(ArtifactBatchHints {
                max_concurrency: Some(DEFAULT_DISCOVERY_MAX_CONCURRENCY),
                multipart_threshold: Some(DEFAULT_MULTIPART_THRESHOLD_BYTES),
            }),
        })
    }

    /// `POST .../commit` — persist manifest (`docs/artifacts-protocol.md` §8.6).
    pub async fn commit_artifacts(
        &self,
        repo: &str,
        req: &ArtifactCommitRequest,
    ) -> Result<ArtifactCommitResponse, MegaError> {
        if repo.trim().is_empty() {
            return Err(MegaError::Other(
                "[code:400] repo must not be empty".to_string(),
            ));
        }
        if req.namespace.trim().is_empty() {
            return Err(MegaError::Other(
                "[code:400] namespace must not be empty".to_string(),
            ));
        }
        let max_files = DEFAULT_MAX_COMMIT_FILES as usize;
        if req.files.len() > max_files {
            return Err(MegaError::Other(format!(
                "[code:400] commit exceeds max_commit_files ({max_files})"
            )));
        }
        if req.files.is_empty() {
            return Err(MegaError::Other(
                "[code:400] files must not be empty".to_string(),
            ));
        }

        let manifest = Self::sorted_manifest_pairs(&req.files)?;
        let object_type_label = req.object_type.as_label().to_string();
        let artifact_set_id = req
            .artifact_set_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if let Some(existing) = self
            .st
            .find_artifact_set_by_natural_key(
                repo,
                &req.namespace,
                &object_type_label,
                &artifact_set_id,
            )
            .await?
        {
            let rows = self.st.list_artifact_set_files(existing.id).await?;
            if Self::committed_files_match_request(&rows, &manifest) {
                let touch: Vec<String> = manifest.iter().map(|(_, oid, _)| oid.clone()).collect();
                self.st.touch_artifact_objects_last_seen_at(&touch).await?;
                return Ok(ArtifactCommitResponse {
                    artifact_set_id,
                    status: "ok".to_string(),
                    missing_objects: vec![],
                });
            }
            return Err(MegaError::Other(
                "[code:409] artifact_set_id already committed with different manifest".to_string(),
            ));
        }

        let oids: Vec<String> = manifest.iter().map(|(_, oid, _)| oid.clone()).collect();
        let objs = self.st.find_artifact_objects_by_oids(&oids).await?;
        let by_oid: HashMap<String, i64> =
            objs.into_iter().map(|m| (m.oid, m.size_bytes)).collect();

        let mut missing_objects: HashSet<String> = HashSet::new();
        for (_, oid, size) in &manifest {
            match by_oid.get(oid) {
                None => {
                    missing_objects.insert(oid.clone());
                }
                Some(db) if db != size => {
                    return Err(MegaError::Other(format!(
                        "[code:400] oid {oid} exists with size different from manifest"
                    )));
                }
                Some(_) => {
                    let key = Self::artifact_object_key(oid);
                    if !self.obj_storage.inner.exists(&key).await? {
                        missing_objects.insert(oid.clone());
                    }
                }
            }
        }
        if !missing_objects.is_empty() {
            return Ok(ArtifactCommitResponse {
                artifact_set_id,
                status: "missing_objects".to_string(),
                missing_objects: missing_objects.into_iter().collect(),
            });
        }

        id_generator::ensure_initialized();

        let conn = self.st.get_connection();
        let txn = conn.begin().await?;

        let now = Utc::now().naive_utc();
        let expires_at = req.expires_in_seconds.map(|secs| {
            use chrono::Duration;
            now + Duration::try_seconds(secs as i64).unwrap_or_else(|| Duration::seconds(0))
        });

        let set_am = artifact_sets::ActiveModel {
            id: Set(IdInstance::next_id()),
            repo: Set(repo.to_string()),
            namespace: Set(req.namespace.clone()),
            object_type: Set(object_type_label),
            artifact_set_id: Set(artifact_set_id.clone()),
            metadata: Set(req.metadata.clone()),
            created_by: Set(None),
            created_at: Set(now),
            expires_at: Set(expires_at),
        };
        let set = set_am.insert(&txn).await.map_err(|e| {
            tracing::warn!("artifact_sets insert: {e}");
            MegaError::Db(e)
        })?;

        for (path, oid, size) in &manifest {
            let file_am = artifact_set_files::ActiveModel {
                set_id: Set(set.id),
                path: Set(path.clone()),
                oid: Set(oid.clone()),
                size_bytes: Set(*size),
                content_type: Set(None),
            };
            file_am.insert(&txn).await.map_err(|e| {
                tracing::warn!("artifact_set_files insert: {e}");
                MegaError::Db(e)
            })?;
        }

        txn.commit().await?;

        let touch: Vec<String> = manifest.iter().map(|(_, oid, _)| oid.clone()).collect();
        self.st.touch_artifact_objects_last_seen_at(&touch).await?;

        Ok(ArtifactCommitResponse {
            artifact_set_id,
            status: "ok".to_string(),
            missing_objects: vec![],
        })
    }

    /// Reclaim `artifact_objects` rows (and backing object-store keys) that are no longer
    /// referenced by any `artifact_set_files` row, and whose `last_seen_at` is older than
    /// `now - grace` (grace window per protocol §10.6).
    ///
    /// Order: re-check references → delete bytes in object storage → delete DB row.
    /// On object-store errors other than “not found”, the DB row is retained for retry.
    pub async fn gc_unreferenced_artifact_objects_once(
        &self,
        grace: Duration,
        batch_limit: u64,
    ) -> Result<ArtifactObjectGcStats, MegaError> {
        let grace_chrono =
            chrono::Duration::from_std(grace).unwrap_or_else(|_| chrono::Duration::zero());
        let cutoff = Utc::now()
            .naive_utc()
            .checked_sub_signed(grace_chrono)
            .unwrap_or_else(|| Utc::now().naive_utc());

        let rows = self
            .st
            .list_gc_unreferenced_artifact_objects(cutoff, batch_limit)
            .await?;

        let mut stats = ArtifactObjectGcStats {
            candidates: rows.len() as u32,
            ..Default::default()
        };

        for row in rows {
            if self.st.artifact_set_files_references_oid(&row.oid).await? {
                stats.skipped_still_referenced += 1;
                continue;
            }

            let key = Self::artifact_object_key(&row.oid);
            match self.obj_storage.inner.delete(&key).await {
                Ok(()) => {}
                Err(MegaError::ObjStorageNotFound(_)) => {
                    tracing::debug!(oid = %row.oid, "artifact GC: object absent in store; dropping DB row");
                }
                Err(e) => {
                    tracing::warn!(
                        oid = %row.oid,
                        error = %e,
                        "artifact GC: object store delete failed; retaining DB row"
                    );
                    stats.storage_delete_errors += 1;
                    continue;
                }
            }

            if let Err(e) = self.st.delete_artifact_object_row(&row.oid).await {
                tracing::error!(
                    oid = %row.oid,
                    error = %e,
                    "artifact GC: DB delete failed after successful store delete"
                );
                stats.db_delete_errors += 1;
            } else {
                stats.deleted += 1;
            }
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod artifact_sets_cursor_tests {
    use chrono::{TimeZone, Timelike, Utc};

    use super::ArtifactService;

    #[test]
    fn set_cursor_v1_roundtrip() {
        let utc = Utc
            .with_ymd_and_hms(2024, 6, 15, 12, 30, 0)
            .unwrap()
            .with_nanosecond(123_456_000)
            .unwrap();
        let dt = utc.naive_utc();
        let id: i64 = 9_001;
        let wire = ArtifactService::encode_set_cursor(dt, id);
        assert!(wire.starts_with("asets-v1|"));
        let (back, id2) = ArtifactService::decode_set_cursor(&wire).unwrap();
        assert_eq!(back, dt);
        assert_eq!(id2, id);
    }

    #[test]
    fn set_cursor_rejects_missing_v1_prefix() {
        let utc = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();
        let s = format!("{}:9001", utc.timestamp_micros());
        assert!(ArtifactService::decode_set_cursor(&s).is_err());
    }
}
