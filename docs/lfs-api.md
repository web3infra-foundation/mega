# LFS API Documentation

## Overview
Git LFS is used to store large files in a separate LFS storage. Git repositories only store pointer files, and clients complete object upload, download, and lock management through the `/info/lfs` endpoints.

Base path: `<repo>.git/info/lfs`

Content-Type:
- JSON requests/responses: `application/vnd.git-lfs+json`
- File/chunk downloads: `application/octet-stream`

## Endpoints

- `POST /info/lfs/objects/batch`: Batch request for upload/download operations
- `GET  /info/lfs/objects/{oid}`: Download object
- `PUT  /info/lfs/objects/{oid}`: Upload object (request body is binary file data)
- `GET  /info/lfs/objects/{oid}/chunks`: Get chunk information (when split mode is enabled)
- `GET  /info/lfs/objects/{oid}/{chunk_id}?offset=&size=`: Download a single chunk
- `GET  /info/lfs/locks`: List locks (supports path/cursor/limit/refspec filtering)
- `POST /info/lfs/locks`: Create lock
- `POST /info/lfs/locks/verify`: Verify lock ownership (ours/theirs)
- `POST /info/lfs/locks/{id}/unlock`: Delete lock

## Examples

### Batch (Download)
```bash
curl -X POST \
  -H "Content-Type: application/vnd.git-lfs+json" \
  -d '{
    "operation": "download",
    "transfers": ["basic"],
    "objects": [{"oid": "abc123", "size": 1024}],
    "hash_algo": "sha256"
  }' \
  https://host/repo.git/info/lfs/objects/batch
```

### Upload Object
```bash
curl -X PUT \
  --data-binary @file.bin \
  https://host/repo.git/info/lfs/objects/abc123
```

### Download Object
```bash
curl -L \
  -H "Accept: application/octet-stream" \
  https://host/repo.git/info/lfs/objects/abc123 -o file.bin
```

### Lock Management
- List locks:
  ```bash
  curl "https://host/repo.git/info/lfs/locks?path=foo.bin&limit=50"
  ```
- Create lock:
  ```bash
  curl -X POST \
    -H "Content-Type: application/vnd.git-lfs+json" \
    -d '{"path":"foo.bin","ref":{"name":"main"}}' \
    https://host/repo.git/info/lfs/locks
  ```
- Delete lock:
  ```bash
  curl -X POST \
    -H "Content-Type: application/vnd.git-lfs+json" \
    -d '{"force":false,"ref":{"name":"main"}}' \
    https://host/repo.git/info/lfs/locks/{id}/unlock
  ```

## Developer Notes
- Download/chunk endpoints return binary streams using `application/octet-stream`.
- Error code conventions: 404 for not found objects/locks, 400 for parameter errors, 500 for other errors.
- Batch download missing objects should return `error` field at object level while maintaining 200 status code overall.
- If chunking is enabled (config `lfs.local.enable_split=true`), first call `/objects/{oid}/chunks` to get the chunk list, then download each chunk individually.

## Related Files
- Routes: `mono/src/api/router/lfs_router.rs`
- Business logic: `ceres/src/lfs/handler.rs`
- Data structures: `ceres/src/lfs/lfs_structs.rs`
