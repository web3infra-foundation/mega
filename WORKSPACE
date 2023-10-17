load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
http_archive(
    name = "rules_rust",
    sha256 = "814680e1ab535f799fd10e8739ddca901351ceb4d2d86dd8126c22d36e9fcbd9",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.29.0/rules_rust-v0.29.0.tar.gz"],
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
        "//:sync/Cargo.toml",
        "//:build-tool/Cargo.toml"
    ],
)

load("@crate_index//:defs.bzl", "crate_repositories")

crate_repositories()
