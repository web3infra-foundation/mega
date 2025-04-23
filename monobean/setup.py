import os
import requests
import argparse
import zipfile
from tqdm import tqdm

def get_redirected_url(url: str) -> str:
    try:
        response = requests.get(url, timeout=5)
        return response.url
    except requests.exceptions.RequestException as e:
        print(f"Error in request: {e}")
        return None

def download_file_with_resume(url, save_path):
    try:
        headers = {}
        if os.path.exists(save_path):
            downloaded_size = os.path.getsize(save_path)
            headers['Range'] = f'bytes={downloaded_size}-'
        else:
            downloaded_size = 0

        response = requests.get(url, headers=headers, stream=True, timeout=10)
        response.raise_for_status()

        total_size = int(response.headers.get('content-length', 0)) + downloaded_size

        mode = 'ab' if downloaded_size else 'wb'
        with open(save_path, mode) as file, tqdm(
            desc=save_path,
            total=total_size,
            unit='B',
            unit_scale=True,
            unit_divisor=1024,
            initial=downloaded_size,
        ) as bar:
            for chunk in response.iter_content(chunk_size=8192):
                file.write(chunk)
                bar.update(len(chunk))
        print(f"Download success!")
    except requests.exceptions.RequestException as e:
        print(f"Failed to download: {e}")

def setup_environmental_variables():
    if os.path.abspath("resources/lib/bin") in os.environ["Path"].split(os.pathsep):
        print("Environment variables already set!")
        return

    env_vars_set = {
        "Path": "resources/lib/bin;",
        "LIB": "resources/lib/lib;",
        "PKG_CONFIG_PATH": "resources/lib/lib/pkgconfig;",
        "INCLUDE": "resources/lib/include;resources/lib/include/cairo;resources/lib/include/glib-2.0;resources/lib/include/gobject-introspection-1.0;resources/lib/lib/glib-2.0/include;",
    }

    commands = []
    for key, value in env_vars_set.items():
        # convert to abs path
        value = os.pathsep.join([os.path.abspath(p) for p in value.split(';')[:-1]])

        commands.append(f'$env:{key} = "{value};$env:{key}"')

    print("Copy and paste these commands to set environment variables in PowerShell:\n")
    for cmd in commands:
        print(cmd)


if __name__ == "__main__":
    if os.name != "nt":
        print("This script is only for Windows!")
        exit()

    parser = argparse.ArgumentParser(description="Setup GTK4 for Gvsbuild")
    parser.add_argument("--upgrade", "-u", action="store_true", help="Upgrade GTK4 to the latest version")
    args = parser.parse_args()

    cwd = os.getcwd()
    cur = os.path.dirname(os.path.abspath(__file__))
    os.chdir(cur)

    try:
        # Check if the setup is already done
        if os.path.exists("resources/lib/DONE") and not args.upgrade:
            print("Setup already done!")
            setup_environmental_variables()
            exit()

        # concatenate the URL with the latest release tag
        GTK_PKG = "https://github.com/wingtk/gvsbuild/releases/latest"
        gtk_ver = get_redirected_url(GTK_PKG).split("/")[-1]
        gtk_url = f"https://github.com/wingtk/gvsbuild/releases/download/{gtk_ver}/GTK4_Gvsbuild_{gtk_ver}_x64.zip"

        if os.path.exists("resources/lib/DONE"):
            with open ("resources/lib/DONE", "r") as f:
                try:
                    installed = f.read().strip().split(".")
                except:
                    installed = ["0", "0", "0"]
                installed = [int(i) for i in installed]
                remote = [int(i) for i in gtk_ver.strip().split(".")]

                if installed >= remote:
                    print("Already up to date!")
                    exit()
                elif args.upgrade:
                    print(f"Upgrading GTK4 {installed} -> {remote}...")
                else:
                    print("GTK4 is outdated!")
                    exit()

        # download the GTK4 package
        print(f"Downloading GTK4 package with version {gtk_ver}...")
        download_file_with_resume(gtk_url, "GTK4_Gvsbuild.zip")

        with zipfile.ZipFile("GTK4_Gvsbuild.zip", "r") as zip_ref:
            zip_ref.extractall("resources/lib")

        os.remove("GTK4_Gvsbuild.zip")

        setup_environmental_variables()

        with open("resources/lib/DONE", "w") as f:
            f.write(gtk_ver)

        print("Setup complete!")

    except Exception as e:
        print(f"Error: {e}")

    os.chdir(cwd)
