import os
import sys
import json
import urllib.request
import tarfile
import subprocess
import shutil
from collections import defaultdict
from packaging import version

# Create a directory if it doesn't exist
def ensure_directory(path):
    if not os.path.exists(path):
        os.makedirs(path)
        print(f"Created directory: {path}")

# Construct the filename and path for the crate
def check_and_download_crate(crates_dir, crate_name, crate_version, dl_base_url):
    crate_filename = f"{crate_name}-{crate_version}.crate"
    crate_path = os.path.join(crates_dir, crate_name, crate_filename)

    # Download the crate if it doesn't exist locally
    if not os.path.exists(crate_path):
        ensure_directory(os.path.dirname(crate_path))  # Ensure the directory exists
        download_url = f"{dl_base_url}/{crate_name}/{crate_filename}"
        try:
            print(f"Downloading: {download_url}")
            urllib.request.urlretrieve(download_url, crate_path)  # Download the file
            print(f"Downloaded: {crate_path}")
        except Exception as e:
            print(f"Error downloading {crate_filename}: {str(e)}")
    return crate_path

# Run a git command in the specified repository
def run_git_command(repo_path, command):
    try:
        result = subprocess.run(command, cwd=repo_path, check=True, capture_output=True, text=True)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"Warning: Git command failed: {e}")
        print(f"Command output: {e.output}")
        return None

# Initialize a git repository if it doesn't exist
def init_git_repo(repo_path):
    if not os.path.exists(os.path.join(repo_path, '.git')):
        run_git_command(repo_path, ['git', 'init', '-b', 'main'])
        print(f"Initialized git repository in {repo_path}")

def extract_crate(crate_path, extract_path):
    # Check if a path is within a directory (for security)
    def is_within_directory(directory, target):
        abs_directory = os.path.abspath(directory)
        abs_target = os.path.abspath(target)
        prefix = os.path.commonprefix([abs_directory, abs_target])
        return prefix == abs_directory

    # Safely extract files from a tar archive
    def safe_extract(tar, path=".", members=None, *, numeric_owner=False):
        for member in tar.getmembers():
            member_path = os.path.join(path, member.name)
            if not is_within_directory(path, member_path):
                raise Exception("Attempted Path Traversal in Tar File")

        # Filter function to ensure extracted files are within the target directory
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

# Process a specific version of a crate
def process_crate_version(crate_name, version, crate_path, git_repos_dir, git_base_url):
    repo_path = os.path.join(git_repos_dir, crate_name, version)
    ensure_directory(repo_path)

    # Extract crate directly to the repo directory
    if not extract_crate(crate_path, repo_path):
        print(f"Skipping processing for {crate_name} version {version} due to extraction failure.")
        return

    # Initialize git repo
    init_git_repo(repo_path)

    # Add all files to git
    run_git_command(repo_path, ['git', 'add', '.'])

    # Commit changes with updated message format
    commit_message = f"{crate_name} {version}"
    run_git_command(repo_path, ['git', 'commit', '-a', '-s', '-S', '-m', commit_message])

    # Add remote and push
    remote_url = f"{git_base_url}/third-part/rust/crates/{crate_name}/{version}.git"
    run_git_command(repo_path, ['git', 'remote', 'add', 'mega', remote_url])

    # Push to remote
    push_result = run_git_command(repo_path, ['git', 'push', '-u', 'mega', 'main'])
    if push_result is None:
        print(f"Warning: Failed to push {crate_name} version {version} to remote repository.")
    else:
        print(f"Successfully pushed {crate_name} version {version} to remote repository.")

# Process all versions of a crate
def process_crate(crate_name, versions, crates_dir, git_repos_dir, dl_base_url, git_base_url):
    for v in versions:
        repo_path = os.path.join(git_repos_dir, crate_name, v)
        if os.path.exists(repo_path):
            print(f"Repository for {crate_name} version {v} already exists. Skipping.")
            continue

        crate_path = check_and_download_crate(crates_dir, crate_name, v, dl_base_url)
        process_crate_version(crate_name, v, crate_path, git_repos_dir, git_base_url)

    print(f"Finished processing {crate_name}")

# Scan the crates.io index and process all crates
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
        dirs[:] = [d for d in dirs if d not in ['.git', '.github']]  # Exclude certain directories

        for file in files:
            if (root == index_path and file == 'config.json') or file == 'README.md':
                continue  # Skip config.json and README.md

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

# Main function to run the script
def main():
    if len(sys.argv) != 5:
        print("Usage: python script.py <path_to_crates.io-index> <path_to_crates_directory> <path_to_git_repos_directory> <git_base_url>")
        sys.exit(1)

    index_path, crates_dir, git_repos_dir, git_base_url = sys.argv[1:5]

    total_crates = scan_and_process_crates(index_path, crates_dir, git_repos_dir, git_base_url)
    print(f"\nTotal number of crates processed: {total_crates}")

# Run the main function if this script is executed directly
if __name__ == "__main__":
    main()
