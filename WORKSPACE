load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
http_archive(
    name = "rules_rust",
    sha256 = "c46bdafc582d9bd48a6f97000d05af4829f62d5fee10a2a3edddf2f3d9a232c1",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.28.0/rules_rust-v0.28.0.tar.gz"],
)

load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")

rules_rust_dependencies()

rust_register_toolchains(
    edition = "2021",
)

load("@rules_rust//crate_universe:repositories.bzl", "crate_universe_dependencies")

crate_universe_dependencies()

load("@rules_rust//crate_universe:defs.bzl", "crates_repository")

crates_repository(
    name = "crate_index",
    cargo_lockfile = "//:Cargo.lock",
    lockfile = "//:cargo-bazel-lock.json",
    manifests = [
        "//:Cargo.toml",
        "//:gateway/Cargo.toml",
        "//:common/Cargo.toml",
        "//:git/Cargo.toml",
        "//:database/Cargo.toml",
        "//:database/entity/Cargo.toml",
        "//:p2p/Cargo.toml",
        "//:mda/Cargo.toml",
        "//:kvcache/Cargo.toml",
        "//:sync/Cargo.toml"
    ],
)

load("@crate_index//:defs.bzl", "crate_repositories")

crate_repositories()
