# Ceres

Monorepo domain library for Mega: Git transport, REST application logic, and shared models.

## Module layout

```
ceres/src/
├── lib.rs
├── bus/                    # Transport ↔ application event bus
├── infra/                  # Shared infrastructure (GitObjectCache)
├── transport/
│   ├── protocol/           # Smart HTTP/SSH Git protocol
│   └── pack/               # receive-pack / upload-pack handlers
├── application/
│   ├── api_service/        # MonoApiService, REST-facing ops
│   ├── code_edit/          # CL create/update pipelines + post-receive handlers
│   └── build_trigger/      # Orion build dispatch
├── model/                  # HTTP/API DTOs
├── diff/, merge_checker/, lfs/
```

Legacy paths (`ceres::protocol`, `ceres::pack`, `ceres::api_service`, etc.) remain as re-exports in `lib.rs` for compatibility with `mono`.

## Dependency rules

| Module | May depend on | Must not depend on |
|--------|---------------|-------------------|
| `transport` | `bus`, `infra`, `model`, `jupiter`, `git-internal` | `application::*` |
| `application` | `bus`, `infra`, `model`, `jupiter`, `git-internal` | `transport::*` (except bus event DTOs) |
| `bus` | Minimal shared types for events | `transport` / `application` implementations |
| `mono` (binary) | Assembles `TransportRuntime` + injects handlers | — |

## Model boundary

Three DTO layers; keep imports aligned with this table:

| Layer | Crate / path | Role | Consumers |
|-------|--------------|------|-----------|
| Wire | `api-model` | mono ↔ orion cross-process protocol (buck2, artifacts, shared pagination wrappers) | `mono`, `orion`, `orion-client`, `ceres` (pagination only where needed) |
| HTTP / OpenAPI | `ceres/model` | All mono REST request/response types + `utoipa` schemas | `mono` routers, `ceres` application |
| Storage assembly | `jupiter/model` | Bundles of `callisto` entities from storage/services; no serde/utoipa | `jupiter` storage/service, `ceres` application only |

Rules:

- `mono` routers must **not** `use jupiter::model` — map via `ceres::model` and `MonoApiService` facades.
- `api-model` is **not** mono HTTP schema (except shared wrappers like `CommonPage` / `Pagination`).
- `ceres/model` is the mapping hub: `impl From<jupiter::model::*>` and `impl From<callisto::*>` live here.
- `application/build_trigger/model` is a ceres subdomain API schema (build triggers); same HTTP rules as `ceres/model`, kept alongside orchestration until a later consolidation.

Long term: extract `ceres/model` → `mega-api-types` only if a non-mono consumer needs HTTP DTOs without ceres domain code.

## Git push event flow

```mermaid
sequenceDiagram
    participant Client
    participant Protocol as transport/protocol
    participant Pack as transport/pack/MonoRepo
    participant Bus as bus
    participant App as application/post_receive

    Client->>Protocol: git-receive-pack
    Protocol->>Pack: unpack + save_entry
    Pack->>Pack: persist_mono_refs + filepath update
    Pack->>Bus: MonoReceivePackFinalized
    Bus->>App: handle(event)
    App->>App: OnpushCodeEdit / bootstrap / build / reanchor
```

Import-repo pushes follow the same pattern via `ImportReceivePackFinalized` → `application/code_edit/post_receive/import.rs`.

## Assembly (`mono`)

`mono` constructs a `TransportRuntime` (alias: `ProtocolApiState`) with storage, `GitObjectCache`, and `RuntimeApplicationHandler`, then passes it to HTTP/SSH Git routers and REST handlers.

```rust
let runtime = TransportRuntime::new(storage, git_object_cache);
// runtime.application handles MonoReceivePackFinalized / ImportReceivePackFinalized
```
