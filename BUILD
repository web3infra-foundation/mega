load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test", "rust_doc_test")


rust_binary(
    name = "mega",
    srcs = glob([
        "src/**/*.rs",
    ]),
    aliases = aliases(),
    deps = all_crate_deps() + [
        "//gateway",
        "//common",
        "//p2p",
        "//mda"
    ],
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
    ),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "tests",
    crate = ":mega", 
    deps = all_crate_deps(
        normal_dev = True,
    ),
    proc_macro_deps = all_crate_deps(
        proc_macro_dev = True,
    ),
)


rust_doc_test(
    name = "doctests",
    crate = ":mega",
)
