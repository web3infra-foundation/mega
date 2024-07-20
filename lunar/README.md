## Lunar Module

## bundle

Because Lunar depends on Mega, which in turn relies on Neptune’s dynamic library pipy, it’s important to handle libpipy carefully during compilation. 

### Linux

- deb 

Currently, on Linux Debian, you can complete the compilation directly using cargo tauri build -b deb.

- AppImage

However, compilation for AppImage will fail. After encountering the error “libpipy.so Not found” when executing `cargo tauri build -b appimage,` you need to manually copy `target/release/libpipy.so` to the `target/release/bundle/appimage/lunar.AppDir/usr/bin` folder and then manually execute `target/release/bundle/appimage/build_appimage.sh`.

> [!NOTE]
> If you use archlinux to bundle, according to [\[bug\]\[linuxdeploy\] Linux AppImage script fails](https://github.com/tauri-apps/tauri/issues/8929), you may need to run build with `NO_STRIP=true cargo tauri **`.
