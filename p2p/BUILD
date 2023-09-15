
load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_doc_test")


rust_library(
    name = "p2p",
    srcs = glob([
        "src/**/*.rs",
    ]),
    aliases = aliases(),
    deps = all_crate_deps() + [
        "//git",
        "//common",
        "//database",
        "//database/entity"
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
    ),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "test",
    crate = ":p2p",
    aliases = aliases(
        normal_dev = True,
        proc_macro_dev = True,
    ),
    deps = all_crate_deps(
        normal_dev = True,
    ) + [
        "//git",
        "//common",
        "//database",
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro_dev = True,
    ),
)

rust_doc_test(
    name = "doctests",
    crate = ":p2p",
)
