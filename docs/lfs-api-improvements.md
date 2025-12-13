# LFS API Improvement Recommendations (GitHub Official Specification Comparison)

## Comparison Baseline
- Git LFS Batch API: https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md
- Git LFS Locking API: https://github.com/git-lfs/git-lfs/blob/main/docs/api/locking.md
- Git LFS Server Discovery: https://github.com/git-lfs/git-lfs/blob/main/docs/api/server-discovery.md

## Issues Found and Fix Recommendations

1) Error codes are too uniform, missing 4xx status codes
- Current state: Most errors in `mono/src/api/router/lfs_router.rs` are mapped to 500.
- Specification: Not found objects/locks should return 404; request parameter errors should return 400.
- Completed: Added `map_lfs_error` in router layer, mapping error messages containing `Not found` / `Invalid` to 404/400.
- Follow-up: Use explicit error types instead of strings in `ceres/src/lfs/handler.rs` to avoid string matching.

2) Download endpoint Content-Type
- Current state: Download object/chunk uses `application/vnd.git-lfs+json`.
- Specification: Data streams should return binary stream `application/octet-stream`.
- Completed: Changed router download response to `application/octet-stream`.

3) Batch download missing 404 semantics
- Current state: `lfs_process_batch` returns errors mapped to 500 at router layer when download objects are missing.
- Specification: Not found objects should return `error` at object level, with overall status 200.
- Recommendation: In `handler::lfs_process_batch`, preserve object-level `error` for missing download objects (already implemented), router maintains 200; avoid treating missing as top-level error in handler.

4) Upload pre-validation insufficient
- Current state: `lfs_upload_object` returns generic error if meta doesn't exist (router maps to 404).
- Recommendation: Use explicit "Not found" message or error type in handler to avoid string matching; validate that `size` matches request body length (currently commented out).

5) Chunk download behavior in non-split mode
- Current state: `lfs_download_chunk` validates hash after slicing in non-split mode, returns 500 on mismatch.
- Recommendation: Return 400 for hash mismatch; return 404 for missing chunk.

6) Lock endpoint error semantics
- Current state: Deleting non-existent lock returns 500.
- Recommendation: `lfs_delete_lock` should return 404 for lock not found; return 403 for no permission (currently no owner validation).

7) OpenAPI coverage
- Completed: Added `utoipa::path` to all LFS routes and registered LFS schemas in `ApiDoc`.
- Recommendation: Provide example response (stream) documentation for LFS download endpoints in future versions.

## Priority
- High: Error code semantics (404/400), download Content-Type, lock deletion 404.
- Medium: Batch download missing objects maintain 200 + object-level error, upload size validation.
- Low: Hash mismatch 400, permission/lock owner validation supplement.
