# Intro
We use a patched version of zed to help with the data edition in the monorepo that mega managing.

# How to use
Currently, the script for patching and building zed is only used for helping making up the docker image.

However, you can still use the script or manually build the patched version of zed.

## With unix-shell environment

Simply execute the script and you shall get the target binary right in the:
```shell
# build the patched zed
./build.sh

# run zed
./zed/target/release/zed
```

## Without unix-shell environment

You can still follow the steps below to build:
- Clone zed
- Checkout to the specific version mentioned in `build.sh`
- Use the patches in `./patches/` with `git am xxx.patch`
- Run `cargo build -r` and enjoy:)
