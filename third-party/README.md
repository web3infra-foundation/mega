# Buck2 third-party dependencies
This folder manages third-party dependencies for the `Mega` project built with Buck2. 

We utilize `Cargo.toml` to list all required crates.io dependencies across all `Mega` modules. 
[`Reindeer`](https://github.com/facebookincubator/reindeer) 
is then used to analyze these dependencies and generate Buck2 build rules in the `./BUCK` file. 

This setup allows you to reference dependencies using the format `//third-party:<package name>` in your own build rules.

## Attention
You need to pay special attention to these `fixups.toml` to ensure cross device compatibility:
- borsh
- openssl-sys

## Update dependencies
### 1. Update `Cargo.toml`
You can treat **third-party** as a normal Cargo project that manages **all** dependencies for the `Mega` project.

To add/update dependencies, simply modify `third-party/Cargo.toml` as you would for any other Cargo project.

For example, to add `tokio`, you would add the following line to `third-party/Cargo.toml`:
```toml
[dependencies]
tokio = { version = "1.43.0", features = ["full"] }
```
And then, add `//third-party:tokio` to the specific `BUCK` file in your module.

### 2. Update `Cargo.lock`
You can update the `Cargo.lock` file by running the following command:
```bash
cd ../
cargo build
```
But the `Cargo.lock` file is in root directory, so you need to run this command to copy it to the `third-party` directory:
```bash
cp ../Cargo.lock Cargo.lock
````

### 3. Generate Buck2 build rules via `Reindeer`
After adding or modifying dependencies, regenerate the `./BUCK` file to make your changes available. Run the following command:
```bash
./run.sh
```
This script will analyze the dependencies in `Cargo.toml`, fetching crates to `/vendor` 
and generate the corresponding Buck2 build rules in `./BUCK`.

In the best - and most common - case, generating Buck build rules is completely automated.
If the crate has no build script (e.g. `build.rs`) (and is therefore pure Rust), 
then the chances are high that the generated rules will Just Work.

If any of the packages you're importing (either the ones you're explicitly importing or their dependencies) has a `build.rs` build script, 
you'll need a `fixups.toml` file for that package to tell Reindeer how to handle it. See [fixups](#Fixups) for details.

### 4. Buck Build
After updating the dependencies, you can build the project as usual:

For example, to build the `vault` module:
```bash
buck2 build //vault:vault
```

## Fixups
Fixups are annotations to help `Reindeer` generate correct build rules for the Cargo packages. 
They're generally only needed when the Cargo build does something that's not precisely described by the Cargo metadata, 
such as the arbitrary actions of **build scripts**.

Fixups are defined in `fixups/<package name>/fixups.toml`. The package name is the **base name**, not including any version information. 
The fixups directory also contains other files as needed.

### Extra sources

By default, `Reindeer` will simply add all `*.rs` files as the `srcs` for the rule.
If you're using the `precise_srcs` option then it will attempt to identify all
the sources by actually parsing the code. Both of these can fail from time to
time - such as by `include!()` of unexpected files, or when files or modules are
introduced by macros.

These extra sources can be added with

```
extra_srcs = [ ... ]
```

in `fixups.toml`, where the extra sources are specified as one or more globs.

#### Example
```toml
extra_srcs = [
    "examples/demo.rs",
    "examples/demo.md"
]
```

Attention:
- The paths are relative to the crate root (the directory containing `Cargo.toml`).

### Environment variables

Some packages use version and other information from Cargo via a set of
environment variables. If a build fails with a message about `CARGO_<something>`
not being defined, then you can add `cargo_env = true` to `fixups.toml`.

Sometimes they need an arbitrary environment variable to be defined. You can specify this with
```
env = { "FOO" = "Value of FOO" }
```

### \[\[buildscript\]\]
In official documentation, this part is **TODO**.
So It's inferred based on experience.

It's used for running build scripts (e.g. `build.rs`).
#### \[buildscript.rustc_flags\]
It means passing the Rust compiler flags output by the `:xxx-build-script-run` target to the compilation process.
```buck
cargo.rust_library(
    name = "xxx",
    ...
    rustc_flags = ["@$(location :xxx-build-script-run[rustc_flags])"],
    ...
)
```
#### \[buildscript.gen_srcs\]
It means setting the `OUT_DIR` environment variable to the output directory of the `:xxx-build-script-run` target.
```buck
cargo.rust_library(
    name = "xxx",
    ...
    env = {
        "OUT_DIR": "$(location :xxx-build-script-run[out_dir])",
    },
    ...
)
```

#### environment variables
If you need to set environment variables for the **build script**, you can specify this with **under** \[\[buildscript\]\] section.
```toml
[[buildscript]]
env = { "FOO" = "Value of FOO" }
```
```buck
buildscript_run(
    name = "xxx-build-script-run",
    ...
    env = {
        "OPT_LEVEL": "3",
    },
    ...
)
```
### References
- [Reindeer/example/fixups](https://github.com/facebookincubator/reindeer/blob/main/example/third-party/fixups)
- [Crates-Pro/fixups](https://github.com/crates-pro/crates-pro-infra/tree/main/third-party/fixups)