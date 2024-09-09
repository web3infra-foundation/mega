import os
import sys
import json
import urllib.request
import tarfile
import subprocess
import shutil
from collections import defaultdict
from packaging import version

def ensure_directory(path):
    if not os.path.exists(path):
        os.makedirs(path)
        print(f"Created directory: {path}")

def check_and_download_crate(crates_dir, crate_name, crate_version, dl_base_url):
    crate_filename = f"{crate_name}-{crate_version}.crate"
    crate_path = os.path.join(crates_dir, crate_name, crate_filename)
    if not os.path.exists(crate_path):
        download_url = f"{dl_base_url}/{crate_name}/{crate_filename}"
        try:
            print(f"Downloading: {download_url}")
            urllib.request.urlretrieve(download_url, crate_path)
            print(f"Downloaded: {crate_path}")
        except Exception as e:
            print(f"Error downloading {crate_filename}: {str(e)}")
    return crate_path

def run_git_command(repo_path, command):
    try:
        result = subprocess.run(command, cwd=repo_path, check=True, capture_output=True, text=True)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"Warning: Git command failed: {e}")
        print(f"Command output: {e.output}")
        return None

def init_git_repo(repo_path):
    if not os.path.exists(os.path.join(repo_path, '.git')):
        run_git_command(repo_path, ['git', 'init', '-b', 'main'])
        print(f"Initialized git repository in {repo_path}")

def extract_crate(crate_path, extract_path):
    def is_within_directory(directory, target):
        abs_directory = os.path.abspath(directory)
        abs_target = os.path.abspath(target)
        prefix = os.path.commonprefix([abs_directory, abs_target])
        return prefix == abs_directory

    def safe_extract(tar, path=".", members=None, *, numeric_owner=False):
        for member in tar.getmembers():
            member_path = os.path.join(path, member.name)
            if not is_within_directory(path, member_path):
                raise Exception("Attempted Path Traversal in Tar File")

        def filter_member(tarinfo, filterpath):
            if is_within_directory(path, os.path.join(filterpath, tarinfo.name)):
                return tarinfo
            else:
                return None

        tar.extractall(path, members, numeric_owner=numeric_owner, filter=filter_member)

    try:
        with tarfile.open(crate_path, 'r:gz') as tar:
            if not tar.getmembers():
                print(f"Warning: Empty crate file {crate_path}. Skipping extraction.")
                return False

            # Create a temporary directory for extraction
            temp_extract_path = extract_path + "_temp"
            ensure_directory(temp_extract_path)

            # Extract to the temporary directory
            safe_extract(tar, temp_extract_path)

            # Move contents from the nested directory to the target directory
            nested_dir = os.path.join(temp_extract_path, os.listdir(temp_extract_path)[0])
            for item in os.listdir(nested_dir):
                shutil.move(os.path.join(nested_dir, item), extract_path)

            # Remove the temporary directory
            shutil.rmtree(temp_extract_path)

        print(f"Extracted version to {extract_path}")
        return True
    except tarfile.ReadError:
        print(f"Warning: Failed to read crate file {crate_path}. Skipping extraction.")
        return False

def process_crate_version(crate_name, version, version_path, git_repos_dir, git_base_url):
    repo_path = os.path.join(git_repos_dir, crate_name, version)
    ensure_directory(repo_path)

    # Copy extracted files to the repo directory
    for item in os.listdir(version_path):
        s = os.path.join(version_path, item)
        d = os.path.join(repo_path, item)
        if os.path.isdir(s):
            shutil.copytree(s, d, dirs_exist_ok=True)
        else:
            shutil.copy2(s, d)

    # Initialize git repo
    init_git_repo(repo_path)

    # Add all files to git
    run_git_command(repo_path, ['git', 'add', '.'])

    # Commit changes
    commit_message = f"Add {crate_name} version {version}"
    run_git_command(repo_path, ['git', 'commit', '-m', commit_message])

    # Add remote and push
    remote_url = f"{git_base_url}/third-part/rust/crates/{crate_name}/{version}.git"
    run_git_command(repo_path, ['git', 'remote', 'add', 'mega', remote_url])

    # Push to remote
    push_result = run_git_command(repo_path, ['git', 'push', '-u', 'mega', 'main'])
    if push_result is None:
        print(f"Warning: Failed to push {crate_name} version {version} to remote repository.")
    else:
        print(f"Successfully pushed {crate_name} version {version} to remote repository.")

def process_crate(crate_name, versions, crates_dir, git_repos_dir, dl_base_url, git_base_url):
    def version_key(v):
        try:
            return version.parse(v)
        except version.InvalidVersion:
            print(f"Warning: Invalid version '{v}' for crate '{crate_name}'. Skipping this version.")
            return version.parse("0.0.0")  # Use a default low version number

    versions_to_process = sorted(versions, key=version_key)

    for v in versions_to_process:
        try:
            version.parse(v)  # Check again if the version is valid
        except version.InvalidVersion:
            continue  # Skip invalid version

        version_path = os.path.join(crates_dir, crate_name, v)
        if os.path.exists(version_path):
            print(f"Directory for {crate_name} version {v} already exists. Skipping.")
            continue

        crate_path = check_and_download_crate(crates_dir, crate_name, v, dl_base_url)
        ensure_directory(version_path)
        if extract_crate(crate_path, version_path):
            process_crate_version(crate_name, v, version_path, git_repos_dir, git_base_url)
        else:
            print(f"Skipping processing for {crate_name} version {v} due to extraction failure.")

    print(f"Finished processing {crate_name}")

def scan_and_process_crates(index_path, crates_dir, git_repos_dir, git_base_url):
    crates = defaultdict(set)
    dl_base_url = None

    # Check if the directories exist
    for path in [index_path, crates_dir, git_repos_dir]:
        if not os.path.isdir(path):
            print(f"Error: The directory {path} does not exist.")
            sys.exit(1)

    # Read the config.json to get the dl base URL
    config_path = os.path.join(index_path, 'config.json')
    try:
        with open(config_path, 'r') as config_file:
            config = json.load(config_file)
            dl_base_url = config.get('dl')
            if not dl_base_url:
                print("Error: 'dl' key not found in config.json")
                sys.exit(1)
    except Exception as e:
        print(f"Error reading config.json: {str(e)}")
        sys.exit(1)

    # Walk through the index directory
    for root, dirs, files in os.walk(index_path):
        dirs[:] = [d for d in dirs if d not in ['.git', '.github']]

        for file in files:
            if (root == index_path and file == 'config.json') or file == 'README.md':
                continue

            full_path = os.path.join(root, file)

            try:
                with open(full_path, 'r') as f:
                    for line in f:
                        line = line.strip()
                        if line:
                            crate_info = json.loads(line)
                            crate_name = crate_info['name']
                            crate_version = crate_info['vers']
                            crates[crate_name].add(crate_version)
            except Exception as e:
                print(f"Error processing file {full_path}: {str(e)}")

    # Process each crate
    for crate_name, versions in crates.items():
        process_crate(crate_name, versions, crates_dir, git_repos_dir, dl_base_url, git_base_url)

    return len(crates)

def main():
    if len(sys.argv) != 5:
        print("Usage: python script.py <path_to_crates.io-index> <path_to_crates_directory> <path_to_git_repos_directory> <git_base_url>")
        sys.exit(1)

    index_path, crates_dir, git_repos_dir, git_base_url = sys.argv[1:5]

    total_crates = scan_and_process_crates(index_path, crates_dir, git_repos_dir, git_base_url)
    print(f"\nTotal number of crates processed: {total_crates}")

if __name__ == "__main__":
    main()
