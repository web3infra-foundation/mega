//! HTTP request/response models for the repo-scoped artifacts protocol (`docs/artifacts-protocol.md`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Canonical object type labels aligned with `git-internal`'s `ObjectType` (0.7.4).
///
/// This enum intentionally uses the **string labels** defined by `ObjectType` in
/// `git-internal/src/internal/object/types.rs` (e.g. `snapshot`, `context_frame`, `plan_step_event`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactObjectType {
    Snapshot,
    Decision,
    Evidence,
    Patchset,
    Plan,
    Provenance,
    Run,
    Task,
    Intent,
    Invocation,
    ContextFrame,
    IntentEvent,
    TaskEvent,
    RunEvent,
    PlanStepEvent,
    RunUsage,
}

impl ArtifactObjectType {
    /// Parse a protocol / DB `object_type` label (`snake_case`).
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.as_label() == label)
    }

    /// JSON / protocol string label (`snake_case`, aligned with `git-internal` 0.7.4 `ObjectType`).
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Decision => "decision",
            Self::Evidence => "evidence",
            Self::Patchset => "patchset",
            Self::Plan => "plan",
            Self::Provenance => "provenance",
            Self::Run => "run",
            Self::Task => "task",
            Self::Intent => "intent",
            Self::Invocation => "invocation",
            Self::ContextFrame => "context_frame",
            Self::IntentEvent => "intent_event",
            Self::TaskEvent => "task_event",
            Self::RunEvent => "run_event",
            Self::PlanStepEvent => "plan_step_event",
            Self::RunUsage => "run_usage",
        }
    }

    /// Canonical ordering for discovery (`docs/artifacts-protocol.md` §5 / §8.2).
    pub const ALL: &'static [Self] = &[
        Self::Snapshot,
        Self::Decision,
        Self::Evidence,
        Self::Patchset,
        Self::Plan,
        Self::Provenance,
        Self::Run,
        Self::Task,
        Self::Intent,
        Self::Invocation,
        Self::ContextFrame,
        Self::IntentEvent,
        Self::TaskEvent,
        Self::RunEvent,
        Self::PlanStepEvent,
        Self::RunUsage,
    ];
}

/// What the `POST .../batch` call is for. V1 only defines [`Upload`](ArtifactIntent::Upload)
/// (negotiate object uploads). Not to be confused with `ObjectType::Intent`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactIntent {
    /// Prepare or refresh uploads: server returns `exists` and optional `actions.upload`.
    Upload,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactObjectDescriptor {
    /// Logical path within the artifact set (not a filesystem path on the server).
    pub path: String,
    /// Artifact object id: UUID string (RFC 4122), not a Git or LFS content hash.
    pub oid: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactBatchRequest {
    pub namespace: String,
    /// Object semantic type for this batch (aligned to `git-internal` `ObjectType`).
    pub object_type: ArtifactObjectType,
    pub intent: ArtifactIntent,
    pub objects: Vec<ArtifactObjectDescriptor>,
    /// Optional user-provided metadata (commit SHA, task id, etc).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactLink {
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Per-object actions in a batch response (v1 defines `upload` only; omit when `exists`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactActions {
    /// Signed or direct upload URL and headers (`href` required when present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload: Option<ArtifactLink>,
}

/// Optional JSON body when `GET .../objects/{oid}` returns a link instead of bytes (read API §4.4).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactObjectReadActions {
    pub download: ArtifactLink,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactObjectReadResponse {
    pub actions: ArtifactObjectReadActions,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactSetListItem {
    pub artifact_set_id: String,
    pub namespace: String,
    pub object_type: ArtifactObjectType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactListSetsResponse {
    pub sets: Vec<ArtifactSetListItem>,
    /// Pass back verbatim as the `cursor` query on the next list request.
    /// Format: `asets-v1|<created_at_unix_micros_utc>|<artifact_sets.id>` (pipe-separated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactSetDetailResponse {
    pub artifact_set_id: String,
    pub namespace: String,
    pub object_type: ArtifactObjectType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub files: Vec<ArtifactObjectDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactResolveFileResponse {
    pub artifact_set_id: String,
    pub path: String,
    pub oid: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub committed_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListArtifactSetsQuery {
    pub namespace: String,
    pub object_type: ArtifactObjectType,
    pub limit: Option<u32>,
    /// Continuation token from the previous response's `next_cursor`
    /// (`asets-v1|<unix_micros_utc>|<artifact_sets.id>`).
    pub cursor: Option<String>,
    pub run_id: Option<String>,
    pub commit_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetArtifactSetQuery {
    pub namespace: String,
    pub object_type: ArtifactObjectType,
}

#[derive(Debug, Deserialize)]
pub struct ResolveArtifactFileQuery {
    pub namespace: String,
    pub object_type: ArtifactObjectType,
    pub path: String,
    pub run_id: Option<String>,
    pub commit_sha: Option<String>,
}

/// Query for `GET .../artifacts/objects/{oid}` (signed redirect vs JSON link vs proxy body).
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct DownloadArtifactObjectQuery {
    /// When `link`, and the deployment supports presigned GET, respond with **200 JSON**
    /// [`ArtifactObjectReadResponse`] instead of **302** to the same URL.
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactBatchObjectResponse {
    /// UUID string (RFC 4122) identifying the blob.
    pub oid: String,
    pub size: i64,
    pub exists: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<ArtifactActions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactBatchHints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multipart_threshold: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactBatchResponse {
    pub transfer: String,
    pub objects: Vec<ArtifactBatchObjectResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hints: Option<ArtifactBatchHints>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactFileDescriptor {
    pub path: String,
    /// UUID string (RFC 4122), same as in batch.
    pub oid: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactCommitRequest {
    pub namespace: String,
    /// Object semantic type for this commit (aligned to `git-internal` `ObjectType`).
    pub object_type: ArtifactObjectType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_set_id: Option<String>,
    pub files: Vec<ArtifactFileDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactCommitResponse {
    pub artifact_set_id: String,
    pub status: String,
    pub missing_objects: Vec<String>,
}

/// `artifacts/v1` discovery — `docs/artifacts-protocol.md` §8.2.
pub const ARTIFACT_PROTOCOL_VERSION: &str = "artifacts/v1";

/// Default limits advertised in discovery (protocol §8.2 example; may later come from config).
pub const DEFAULT_MAX_OBJECTS_PER_BATCH: u32 = 1000;
pub const DEFAULT_MAX_OBJECT_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_COMMIT_FILES: u32 = 50_000;
pub const DEFAULT_DISCOVERY_MAX_CONCURRENCY: u32 = 8;
pub const DEFAULT_MULTIPART_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;

/// TTL (seconds) for presigned GET/PUT URLs returned by batch and object download JSON.
pub const ARTIFACT_PRESIGN_URL_TTL_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactDiscoveryTransfers {
    pub signed_url_put: bool,
    pub server_fallback_put: bool,
    pub signed_url_get: bool,
    pub server_proxy_get: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactDiscoveryLimits {
    pub max_objects_per_batch: u32,
    pub max_object_size_bytes: u64,
    pub max_commit_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactDiscoveryHints {
    pub default_max_concurrency: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multipart_threshold: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtifactDiscoveryResponse {
    pub protocol_version: String,
    pub supported_object_types: Vec<String>,
    pub transfers: ArtifactDiscoveryTransfers,
    pub limits: ArtifactDiscoveryLimits,
    pub hints: ArtifactDiscoveryHints,
}

impl ArtifactDiscoveryTransfers {
    /// All transfer modes advertised as available (e.g. unit tests).
    pub fn fully_enabled() -> Self {
        Self {
            signed_url_put: true,
            server_fallback_put: true,
            signed_url_get: true,
            server_proxy_get: true,
        }
    }
}

/// Builds the §8.2 discovery document. `transfers` MUST match what `POST .../batch` and object
/// `GET` actually offer for this deployment (e.g. `signed_url_*` false on local disk).
pub fn build_artifact_discovery_response(
    transfers: ArtifactDiscoveryTransfers,
) -> ArtifactDiscoveryResponse {
    ArtifactDiscoveryResponse {
        protocol_version: ARTIFACT_PROTOCOL_VERSION.to_string(),
        supported_object_types: ArtifactObjectType::ALL
            .iter()
            .copied()
            .map(|t| t.as_label().to_string())
            .collect(),
        transfers,
        limits: ArtifactDiscoveryLimits {
            max_objects_per_batch: DEFAULT_MAX_OBJECTS_PER_BATCH,
            max_object_size_bytes: DEFAULT_MAX_OBJECT_SIZE_BYTES,
            max_commit_files: DEFAULT_MAX_COMMIT_FILES,
        },
        hints: ArtifactDiscoveryHints {
            default_max_concurrency: DEFAULT_DISCOVERY_MAX_CONCURRENCY,
            multipart_threshold: Some(DEFAULT_MULTIPART_THRESHOLD_BYTES),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_type_from_label_roundtrip() {
        assert_eq!(
            ArtifactObjectType::from_label("context_frame"),
            Some(ArtifactObjectType::ContextFrame)
        );
        assert_eq!(ArtifactObjectType::from_label("unknown_label_xyz"), None);
    }

    #[test]
    fn discovery_matches_protocol_v1_example() {
        let d = build_artifact_discovery_response(ArtifactDiscoveryTransfers::fully_enabled());
        assert_eq!(d.protocol_version, "artifacts/v1");
        assert_eq!(
            d.supported_object_types.len(),
            ArtifactObjectType::ALL.len()
        );
        assert_eq!(d.supported_object_types[0], "snapshot");
        assert_eq!(d.supported_object_types.last().unwrap(), "run_usage");
        assert!(d.transfers.signed_url_put);
        assert!(d.transfers.server_fallback_put);
        assert!(d.transfers.signed_url_get);
        assert!(d.transfers.server_proxy_get);
        assert_eq!(d.limits.max_objects_per_batch, 1000);
        assert_eq!(d.limits.max_object_size_bytes, 10 * 1024 * 1024 * 1024);
        assert_eq!(d.limits.max_commit_files, 50_000);
        assert_eq!(d.hints.default_max_concurrency, 8);
        assert_eq!(d.hints.multipart_threshold, Some(100 * 1024 * 1024));
    }
}
