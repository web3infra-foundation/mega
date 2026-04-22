use std::time::Duration;

use api_model::artifacts::{
    ARTIFACT_PRESIGN_URL_TTL_SECS, ArtifactBatchRequest, ArtifactBatchResponse,
    ArtifactCommitRequest, ArtifactCommitResponse, ArtifactDiscoveryResponse, ArtifactLink,
    ArtifactListSetsResponse, ArtifactObjectReadActions, ArtifactObjectReadResponse,
    ArtifactObjectType, ArtifactResolveFileResponse, ArtifactSetDetailResponse,
    DownloadArtifactObjectQuery, GetArtifactSetQuery, ListArtifactSetsQuery,
    ResolveArtifactFileQuery,
};
use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use common::errors::MegaError;
use futures::{StreamExt, TryStreamExt};
use http::header;
use jupiter::service::artifact_service::ArtifactService;
use percent_encoding::percent_decode_str;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::ARTIFACTS_TAG, error::ApiError};

/// Artifact HTTP routes merged under [`crate::api::api_router`] → `/api/v1`.
///
/// `#[utoipa::path(path = ...)]` values are **suffixes only**, relative to
/// `nest("/repos/{repo}/artifacts", ...)` here (same pattern as `lfs_router` / `merge_queue_router`).
/// Example: `path = "/discovery"` → OpenAPI `/api/v1/repos/{repo}/artifacts/discovery`.
pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/repos/{repo}/artifacts",
        OpenApiRouter::new()
            .routes(routes!(discovery))
            .routes(routes!(list_artifact_sets))
            .routes(routes!(get_artifact_set))
            .routes(routes!(resolve_artifact_file))
            .routes(routes!(download_object, head_artifact_object))
            .routes(routes!(batch))
            .routes(routes!(commit))
            .routes(routes!(upload_object_fallback)),
    )
}

fn decode_path_segment(segment: &str) -> String {
    percent_decode_str(segment).decode_utf8_lossy().into_owned()
}

fn mega_to_api(err: MegaError) -> ApiError {
    ApiError::from(anyhow::Error::from(err))
}

/// Discover artifact protocol capabilities for a repo (see `docs/artifacts-protocol.md`).
#[utoipa::path(
    get,
    path = "/discovery",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)")
    ),
    responses(
        (status = 200, description = "Discovery payload", body = ArtifactDiscoveryResponse, content_type = "application/json"),
        (status = 401, description = "Missing or invalid Bearer token when the deployment requires auth")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn discovery(
    State(state): State<MonoApiServiceState>,
    Path(_repo): Path<String>,
) -> Result<Json<ArtifactDiscoveryResponse>, ApiError> {
    Ok(Json(state.storage.artifact_service.discovery_response()))
}

/// List committed artifact sets for a repo (paginated).
#[utoipa::path(
    get,
    path = "/sets",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)"),
        ("namespace" = String, Query, description = "Artifact namespace"),
        ("object_type" = ArtifactObjectType, Query, description = "Object type label"),
        ("limit" = Option<u32>, Query, description = "Page size"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor"),
        ("run_id" = Option<String>, Query, description = "Filter metadata.run_id"),
        ("commit_sha" = Option<String>, Query, description = "Filter metadata.commit_sha")
    ),
    responses(
        (status = 200, description = "Paged sets", body = ArtifactListSetsResponse, content_type = "application/json"),
        (status = 400, description = "Invalid cursor", content_type = "application/json"),
        (status = 500, description = "Database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn list_artifact_sets(
    State(state): State<MonoApiServiceState>,
    Path(repo): Path<String>,
    Query(q): Query<ListArtifactSetsQuery>,
) -> Result<Json<ArtifactListSetsResponse>, ApiError> {
    let repo = decode_path_segment(&repo);
    let body = state
        .storage
        .artifact_service
        .list_artifact_sets(&repo, &q)
        .await
        .map_err(mega_to_api)?;
    Ok(Json(body))
}

/// Get one artifact set manifest (metadata + files).
#[utoipa::path(
    get,
    path = "/sets/{artifact_set_id}",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)"),
        ("artifact_set_id" = String, Path, description = "Client commit idempotency key for the set"),
        ("namespace" = String, Query, description = "Artifact namespace"),
        ("object_type" = ArtifactObjectType, Query, description = "Object type label")
    ),
    responses(
        (status = 200, description = "Set detail", body = ArtifactSetDetailResponse, content_type = "application/json"),
        (status = 404, description = "Set not found", content_type = "application/json"),
        (status = 500, description = "Database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn get_artifact_set(
    State(state): State<MonoApiServiceState>,
    Path((repo, artifact_set_id)): Path<(String, String)>,
    Query(q): Query<GetArtifactSetQuery>,
) -> Result<Json<ArtifactSetDetailResponse>, ApiError> {
    let repo = decode_path_segment(&repo);
    let artifact_set_id = decode_path_segment(&artifact_set_id);
    let body = state
        .storage
        .artifact_service
        .get_artifact_set_detail(&repo, &artifact_set_id, &q)
        .await
        .map_err(mega_to_api)?;
    Ok(Json(body))
}

/// Resolve latest committed file row by logical path and optional metadata filters.
#[utoipa::path(
    get,
    path = "/resolve-file",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)"),
        ("namespace" = String, Query, description = "Artifact namespace"),
        ("object_type" = ArtifactObjectType, Query, description = "Object type label"),
        ("path" = String, Query, description = "Logical artifact path"),
        ("run_id" = Option<String>, Query, description = "Filter metadata.run_id"),
        ("commit_sha" = Option<String>, Query, description = "Filter metadata.commit_sha")
    ),
    responses(
        (status = 200, description = "Resolved file row", body = ArtifactResolveFileResponse, content_type = "application/json"),
        (status = 404, description = "No matching file", content_type = "application/json"),
        (status = 500, description = "Database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn resolve_artifact_file(
    State(state): State<MonoApiServiceState>,
    Path(repo): Path<String>,
    Query(q): Query<ResolveArtifactFileQuery>,
) -> Result<Json<ArtifactResolveFileResponse>, ApiError> {
    let repo = decode_path_segment(&repo);
    let body = state
        .storage
        .artifact_service
        .resolve_artifact_file(&repo, &q)
        .await
        .map_err(mega_to_api)?;
    Ok(Json(body))
}

/// Download object bytes, redirect to signed URL, or return a JSON download link (see protocol §8.7.4).
#[utoipa::path(
    get,
    path = "/objects/{oid}",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)"),
        ("oid" = String, Path, description = "Artifact object id (UUID string per RFC 4122)"),
        ("mode" = Option<String>, Query, description = "Use `link` for JSON `actions.download` when presigned GET is supported; omit for 302 or proxied bytes")
    ),
    responses(
        (status = 200, description = "Object bytes", content_type = "application/octet-stream"),
        (status = 200, description = "Download link wrapper", body = ArtifactObjectReadResponse, content_type = "application/json"),
        (status = 206, description = "Partial content (Range)", content_type = "application/octet-stream"),
        (status = 302, description = "Redirect to signed GET URL"),
        (status = 304, description = "Not modified (If-None-Match)"),
        (status = 400, description = "Bad query (e.g. mode=link without presign support)", content_type = "application/json"),
        (status = 404, description = "Object not available for this repo", content_type = "application/json"),
        (status = 416, description = "Range not satisfiable", content_type = "application/json"),
        (status = 500, description = "Storage or database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn download_object(
    State(state): State<MonoApiServiceState>,
    Path((repo, oid)): Path<(String, String)>,
    Query(q): Query<DownloadArtifactObjectQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let repo = decode_path_segment(&repo);
    let oid = decode_path_segment(&oid);
    let svc = &state.storage.artifact_service;

    let model = svc
        .artifact_object_model_for_committed_repo_download(&repo, &oid)
        .await
        .map_err(mega_to_api)?;

    let etag = ArtifactService::weak_etag_for_oid_size(&model.oid, model.size_bytes);
    let range_hdr = headers.get(header::RANGE).and_then(|v| v.to_str().ok());

    if range_hdr.is_none()
        && let Some(inm) = headers.get(header::IF_NONE_MATCH)
        && inm.to_str().ok().map(str::trim) == Some(etag.trim())
    {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .body(Body::empty())
            .map_err(ApiError::internal);
    }

    if q.mode.as_deref() == Some("link") && !svc.supports_artifact_presigned_urls() {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "mode=link requires presigned URL support (S3/GCS); omit mode for server-proxied download"
        )));
    }

    let presign_ttl = Duration::from_secs(ARTIFACT_PRESIGN_URL_TTL_SECS);
    let signed_get = svc
        .artifact_object_signed_get_url(&oid, presign_ttl)
        .await
        .map_err(mega_to_api)?;

    if let Some(url) = signed_get {
        if q.mode.as_deref() == Some("link") {
            let expires_at = (Utc::now()
                + chrono::Duration::seconds(ARTIFACT_PRESIGN_URL_TTL_SECS as i64))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let body = ArtifactObjectReadResponse {
                actions: ArtifactObjectReadActions {
                    download: ArtifactLink {
                        href: url,
                        header: None,
                        expires_at: Some(expires_at),
                    },
                },
            };
            return Ok(Json(body).into_response());
        }
        return Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, url)
            .body(Body::empty())
            .map_err(ApiError::internal);
    }

    let len = model.size_bytes.max(0) as u64;
    let range_parsed = match ArtifactService::parse_artifact_object_range(range_hdr, len) {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("[code:416]") {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{len}"))
                    .body(Body::empty())
                    .map_err(ApiError::internal);
            }
            return Err(mega_to_api(e));
        }
    };

    let content_type = model
        .content_type
        .as_deref()
        .unwrap_or("application/octet-stream");

    match range_parsed {
        None => {
            let stream = svc
                .get_artifact_object_byte_stream(&oid)
                .await
                .map_err(mega_to_api)?;
            let mapped = stream.map(|r| r.map_err(std::io::Error::other));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CONTENT_LENGTH, model.size_bytes.to_string())
                .header(header::ETAG, &etag)
                .body(Body::from_stream(mapped))
                .map_err(ApiError::internal)
        }
        Some((start, end_exclusive)) => {
            let stream = svc
                .get_artifact_object_range_byte_stream(&oid, start, end_exclusive)
                .await
                .map_err(mega_to_api)?;
            let mapped = stream.map(|r| r.map_err(std::io::Error::other));
            let range_len = end_exclusive.saturating_sub(start);
            let last = end_exclusive.saturating_sub(1);
            let content_range = format!("bytes {start}-{last}/{len}");
            Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CONTENT_LENGTH, range_len.to_string())
                .header(header::CONTENT_RANGE, content_range)
                .header(header::ETAG, &etag)
                .body(Body::from_stream(mapped))
                .map_err(ApiError::internal)
        }
    }
}

/// `HEAD .../objects/{oid}` — metadata only (protocol §8.7.4 optional).
#[utoipa::path(
    head,
    path = "/objects/{oid}",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)"),
        ("oid" = String, Path, description = "Artifact object id (UUID string per RFC 4122)")
    ),
    responses(
        (status = 200, description = "Headers only", content_type = "application/octet-stream"),
        (status = 404, description = "Object not available for this repo", content_type = "application/json"),
        (status = 500, description = "Storage or database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn head_artifact_object(
    State(state): State<MonoApiServiceState>,
    Path((repo, oid)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let repo = decode_path_segment(&repo);
    let oid = decode_path_segment(&oid);
    let svc = &state.storage.artifact_service;
    let model = svc
        .artifact_object_model_for_committed_repo_download(&repo, &oid)
        .await
        .map_err(mega_to_api)?;
    let etag = ArtifactService::weak_etag_for_oid_size(&model.oid, model.size_bytes);
    let content_type = model
        .content_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, model.size_bytes.to_string())
        .header(header::ETAG, etag)
        .body(Body::empty())
        .map_err(ApiError::internal)
}

/// Batch negotiate repo-scoped artifact uploads.
#[utoipa::path(
    post,
    path = "/batch",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)")
    ),
    request_body = ArtifactBatchRequest,
    responses(
        (status = 200, description = "Batch response with upload actions", body = ArtifactBatchResponse, content_type = "application/json"),
        (status = 400, description = "Invalid request (namespace, oid, path, size, limits)", content_type = "application/json"),
        (status = 500, description = "Database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn batch(
    State(state): State<MonoApiServiceState>,
    Path(_repo): Path<String>,
    Json(req): Json<ArtifactBatchRequest>,
) -> Result<Json<ArtifactBatchResponse>, ApiError> {
    let body = state
        .storage
        .artifact_service
        .batch_artifacts(&req)
        .await
        .map_err(mega_to_api)?;
    Ok(Json(body))
}

/// Commit an artifact set manifest to make it queryable.
#[utoipa::path(
    post,
    path = "/commit",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)")
    ),
    request_body = ArtifactCommitRequest,
    responses(
        (status = 200, description = "Commit response (ok or missing_objects)", body = ArtifactCommitResponse, content_type = "application/json"),
        (status = 400, description = "Invalid request or size mismatch", content_type = "application/json"),
        (status = 409, description = "artifact_set_id already committed with a different manifest", content_type = "application/json"),
        (status = 500, description = "Database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn commit(
    State(state): State<MonoApiServiceState>,
    Path(repo): Path<String>,
    Json(req): Json<ArtifactCommitRequest>,
) -> Result<Json<ArtifactCommitResponse>, ApiError> {
    let repo = decode_path_segment(&repo);
    let body = state
        .storage
        .artifact_service
        .commit_artifacts(&repo, &req)
        .await
        .map_err(mega_to_api)?;
    Ok(Json(body))
}

/// Fallback endpoint to upload object bytes through the Mono server (when signed URLs are unavailable).
#[utoipa::path(
    put,
    path = "/objects/{oid}",
    params(
        ("repo" = String, Path, description = "Single URL path segment identifying the repo (use %2F for `/` inside names, e.g. `org%2Fproject`)"),
        ("oid" = String, Path, description = "Artifact object id (UUID string per RFC 4122)")
    ),
    request_body(content = Vec<u8>, content_type = "application/octet-stream", description = "Object bytes"),
    responses(
        (status = 204, description = "Uploaded (or already exists)"),
        (status = 400, description = "Invalid oid or body", content_type = "application/json"),
        (status = 409, description = "Oid exists with a different size", content_type = "application/json"),
        (status = 500, description = "Storage or database error", content_type = "application/json")
    ),
    tag = ARTIFACTS_TAG
)]
pub async fn upload_object_fallback(
    State(state): State<MonoApiServiceState>,
    Path((_repo, oid)): Path<(String, String)>,
    req: Request<Body>,
) -> Result<StatusCode, ApiError> {
    let oid = decode_path_segment(&oid);
    let content_length = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    let body_bytes: Vec<u8> = req
        .into_body()
        .into_data_stream()
        .try_fold(Vec::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .map_err(ApiError::internal)?;

    if let Some(expected) = content_length
        && expected != body_bytes.len()
    {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Content-Length ({expected}) does not match received body size ({})",
            body_bytes.len()
        )));
    }

    state
        .storage
        .artifact_service
        .upload_artifact_object_bytes(&oid, body_bytes)
        .await
        .map_err(mega_to_api)?;
    Ok(StatusCode::NO_CONTENT)
}
