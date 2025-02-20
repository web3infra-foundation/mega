## Lunar Module

## bundle

Because Lunar depends on Mega, which in turn relies on Neptune’s dynamic library pipy, it’s important to handle libpipy carefully during compilation. 

> [!NOTE]
> The file paths in `src-tauri/tauri.conf.json` are set for `cargo tauri build` because `cargo build` and `cargo tauri build` use different file paths. If you want to use `cargo build`, you need to modify the file paths in `src-tauri/tauri.conf.json` to match the corresponding paths.

### Linux

- deb 

Currently, on Linux Debian, you can complete the compilation directly using cargo tauri build -b deb.

- AppImage

However, compilation for AppImage will fail. After encountering the error “libpipy.so Not found” when executing `cargo tauri build -b appimage,` you need to manually copy `target/release/libpipy.so` to the `target/release/bundle/appimage/lunar.AppDir/usr/bin` folder and then manually execute `target/release/bundle/appimage/build_appimage.sh`.

> [!NOTE]
> If you use archlinux to bundle, according to [\[bug\]\[linuxdeploy\] Linux AppImage script fails](https://github.com/tauri-apps/tauri/issues/8929), you may need to run build with `NO_STRIP=true cargo tauri **`.


### MacOS

- dmg
According to [tauri-apps issue(3055)](https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1866022065), you may need to give `Terminal` app or `vscode` app, wahtever you use to compile, the permissions to control `Finder.app` to finish build `dmg` in Apple Silicon. But this didn't influence the build of `app`
