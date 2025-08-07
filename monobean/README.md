# Monobean

Monobean is a cross-platform desktop application designed to manage Mega and Fuse in a user-friendly way. This module is currently in early development.

## Goals

- Provide a cross-platform desktop client for Mega
- Enable quick and easy management of Mega and Fuse
- Implement monorepo management with a focus on user experience
- Add support for Mega's P2P functionality in the future

## Development Status

This project is in the early stages of development. The core functionality is being implemented and the architecture is being designed.

## Building Guidelines
As this module uses Gtk4 and libadwaita to construct the GUI, you should additionally install the following packages:
- libgtk-4-dev
- libadwaita-1-0
- libadwaita-1-dev
- libgtksourceview-5-dev

### For Ubuntu Users
Tested on Ubuntu 24.04 (Noble), other Ubuntu versions should work as well.
Simply type the following commands to build:

```bash
sudo apt update
sudo apt install -y libgtk-4-dev libadwaita-1-0 libadwaita-1-dev libgtksourceview-5-dev
cargo build --bin monobean
```

### For macOS Users
Install GTK 4 by executing the following in your terminal, then build:
```bash
brew install gtk4 gtksourceview5 libadwaita
cargo build --bin monobean
```

### For Windows Users
> You can either use the python script [setup.py](setup.py) (Recommended) or follow the instructions below. It's equivalent to what the script does.

Download precompiled Gtk4 libraries from [gvsbuild](https://github.com/wingtk/gvsbuild#development-environment). Then set up environmental variables like [this](https://github.com/wingtk/gvsbuild?tab=readme-ov-file#environmental-variables):
```powershell
$env:Path = "C:\gtk\bin;" + $env:Path
$env:LIB = "C:\gtk\lib;" + $env:LIB
$env:PKG_CONFIG_PATH = "C:\gtk\lib\pkgconfig" + $env:PKG_CONFIG_PATH
$env:INCLUDE = "C:\gtk\include;C:\gtk\include\cairo;C:\gtk\include\glib-2.0;C:\gtk\include\gobject-introspection-1.0;C:\gtk\lib\glib-2.0\include;" + $env:INCLUDE
```

Then build the package:

```pwsh
cargo build --bin monobean
```
Also refer to the [gtk-rs documentation](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html) for more detailed instructions.

## Troubleshooting
1. `error: process didn't exit successfully: 'monobean.exe' (exit code: 0xc0000139, STATUS_ENTRYPOINT_NOT_FOUND)`
   - When building in the Windows CLI environment, you may encounter this error and the process exits unexpectedly.
   - This error might occur because another program's PATH is overriding the GTK runtime PATH (e.g., Anaconda). To resolve this, try exiting the conda environment.

## Contributing

Contributions are welcome! Please see the main [Mega repository](https://github.com/web3infra-foundation/mega) for contribution guidelines.

## License

This project is licensed under the terms of the MIT license. See the [LICENSE](../LICENSE-MIT) file for details.
