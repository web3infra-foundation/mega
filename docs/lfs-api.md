# Git LFS API

Git LFS stores large blobs outside the Git object graph. Clients negotiate upload/download through batch requests and use separate object endpoints for binary transfer.

Official references:

- [Batch API](https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md)
- [Locking API](https://github.com/git-lfs/git-lfs/blob/main/docs/api/locking.md)
- [Server discovery](https://github.com/git-lfs/git-lfs/blob/main/docs/api/server-discovery.md)

Interactive docs: start `mono` and open Swagger UI at `/swagger-ui` (LFS routes under the LFS tag). See [architecture.md](architecture.md#http-api-discovery).

## URL layout

Mega exposes the same handlers on two prefixes:

| Audience | Base path | Example |
|----------|-----------|---------|
| Git LFS clients | `<repo>.git/info/lfs` | `/project/foo.git/info/lfs/objects/batch` |
| OpenAPI / tools | `/api/v1/lfs` | `/api/v1/lfs/objects/batch` |

Repo paths follow monorepo layout (`/project/...`, `/third-party/...`). The HTTP server rewrites `.../info/lfs/...` request URIs so handlers see a normalized path (`mono/src/server/http_server.rs`).

## Content types

| Use | Content-Type |
|-----|--------------|
| JSON request/response | `application/vnd.git-lfs+json` |
| Object download body | `application/octet-stream` |

## Endpoints

Relative to either base path above:

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/objects/batch` | Batch upload/download negotiation |
| `GET` | `/objects/{oid}` | Download object (binary stream) |
| `PUT` | `/objects/{oid}` | Upload object (binary body) |
| `GET` | `/locks` | List locks (`path`, `cursor`, `limit`, `refspec` query params) |
| `POST` | `/locks` | Create lock |
| `POST` | `/locks/verify` | Verify locks before push |
| `POST` | `/locks/{id}/unlock` | Delete lock |

Chunk download endpoints (`/objects/{oid}/chunks/...`) are **not** exposed in the current router.

## Examples

### Batch (download)

```bash
curl -X POST \
  -H "Content-Type: application/vnd.git-lfs+json" \
  -d '{
    "operation": "download",
    "transfers": ["basic"],
    "objects": [{"oid": "abc123...", "size": 1024}],
    "hash_algo": "sha256"
  }' \
  http://localhost:8000/project/demo.git/info/lfs/objects/batch
```

### Upload object

```bash
curl -X PUT \
  --data-binary @file.bin \
  http://localhost:8000/project/demo.git/info/lfs/objects/abc123...
```

### Download object

```bash
curl -L \
  -H "Accept: application/octet-stream" \
  http://localhost:8000/project/demo.git/info/lfs/objects/abc123... -o file.bin
```

### Lock management

```bash
# List locks
curl "http://localhost:8000/project/demo.git/info/lfs/locks?path=foo.bin&limit=50"

# Create lock
curl -X POST \
  -H "Content-Type: application/vnd.git-lfs+json" \
  -d '{"path":"foo.bin","ref":{"name":"main"}}' \
  http://localhost:8000/project/demo.git/info/lfs/locks

# Delete lock
curl -X POST \
  -H "Content-Type: application/vnd.git-lfs+json" \
  -d '{"force":false,"ref":{"name":"main"}}' \
  http://localhost:8000/project/demo.git/info/lfs/locks/{id}/unlock
```

## Implementation notes

- **Error mapping:** Router maps handler messages to HTTP status — `404` for not found, `400` for invalid input, `500` otherwise (`map_lfs_error` in `mono/src/api/router/lfs_router.rs`).
- **Batch download:** Missing objects should appear as per-object `error` fields with overall HTTP `200`, not a top-level failure.
- **Download Content-Type:** Object downloads return `application/octet-stream`, not LFS JSON.

## Source files

| Layer | Path |
|-------|------|
| Routes | `mono/src/api/router/lfs_router.rs` |
| Business logic | `ceres/src/lfs/handler.rs` |
| Types | `ceres/src/lfs/lfs_structs.rs` |
