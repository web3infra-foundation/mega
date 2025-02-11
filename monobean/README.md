# Monobean

The Monobean is a cross-platform desktop application for managing Mega and Fuse in a user-friendly way. This module is currently in early development.

## Goals

- Provide a cross-platform desktop client for Mega
- Enable quick and easy management of Mega and Fuse
- Implement monorepo management with a focus on user experience
- Add support for Mega's P2P functionality in the future

## Development Status

This project is in the early stages of development. The core functionality is being implemented and the architecture is being designed.

## Building Guidelines
As this module uses Gtk4 and libadwaita to contruct GUI, you should addtionally install the following packages:
- libgtk-4-dev
- libadwaita-1-0
- libadwaita-1-dev

### For Ubuntu-24.04(noble) users
Simply Type the commands as follow to build:
```bash
sudo apt update
sudo apt install -y libgtk-4-dev libadwaita-1-0 libadwaita-1-dev
cargo build -p monobean
```

### For MacOS Users
Install GTK 4 by executing the following in your terminal then build:
```bash
brew install gtk4
cargo build -p monobean
```

### For Windows users
Things would be a little bit complex here.
You can download precompiled Gtk4 libraries from [gvsbuild](https://github.com/wingtk/gvsbuild#development-environment). Then setup environmental variables like [this](https://github.com/wingtk/gvsbuild?tab=readme-ov-file#environmental-variables):
```powershell
$env:Path = "C:\gtk\bin;" + $env:Path
$env:LIB = "C:\gtk\lib;" + $env:LIB
$env:INCLUDE = "C:\gtk\include;C:\gtk\include\cairo;C:\gtk\include\glib-2.0;C:\gtk\include\gobject-introspection-1.0;C:\gtk\lib\glib-2.0\include;" + $env:INCLUDE
```
Then build the package:
```powershell
cargo build -p monobean
```
You can also refer to the [gtk-rs doc](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html) for more detailed instructions.

## Contributing

Contributions are welcome! Please see the main [Mega repository](https://github.com/web3infra-foundation/mega) for contribution guidelines.

## License

This project is licensed under the terms of the MIT license. See the [LICENSE](../LICENSE-MIT) file for details.
