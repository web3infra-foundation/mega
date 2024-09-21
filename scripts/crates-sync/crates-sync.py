import os
import sys
import json
import urllib.request
import tarfile
import subprocess
import shutil
from collections import defaultdict
from datetime import datetime, timedelta

# ANSI color codes
GREEN = '\033[92m'
BLUE = '\033[94m'
RED = '\033[91m'
RESET = '\033[0m'

def print_green(text):
    # Print text in green color
    print(f"{GREEN}{text}{RESET}")

def print_blue(text):
    # Print text in blue color
    print(f"{BLUE}{text}{RESET}")

def print_red(text):
    # Print text in red color
    print(f"{RED}{text}{RESET}")

def ensure_directory(path):
    # Create a directory if it doesn't exist
    if not os.path.exists(path):
        os.makedirs(path)
        print(f"Created directory: {path}")

def check_and_download_crate(crates_dir, crate_name, crate_version, dl_base_url):
    # Construct the filename and path for the crate
    crate_filename = f"{crate_name}-{crate_version}.crate"
    crate_path = os.path.join(crates_dir, crate_name, crate_filename)

    # Download the crate if it doesn't exist locally
    if not os.path.exists(crate_path):
        ensure_directory(os.path.dirname(crate_path))  # Ensure the directory exists
        download_url = f"{dl_base_url}/{crate_name}/{crate_filename}"
        try:
            print_red(f"Downloading: {download_url}")
            urllib.request.urlretrieve(download_url, crate_path)  # Download the file
            print_red(f"Downloaded: {crate_path}")
        except Exception as e:
            print_red(f"Error downloading {crate_filename}: {str(e)}")
    return crate_path

def run_git_command(repo_path, command):
    # Run a git command in the specified repository
    try:
        result = subprocess.run(command, cwd=repo_path, check=True, capture_output=True, text=True)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print_red(f"Warning: Git command failed: {e}")
        print_red(f"Command output: {e.output}")
        return None

def init_git_repo(repo_path, git_base_url):
    # Initialize a git repository if it doesn't exist
    if not os.path.exists(os.path.join(repo_path, '.git')):
        run_git_command(repo_path, ['git', 'init', '-b', 'main'])
        print_blue(f"Initialized git repository in {repo_path}")

        # Set the LFS domain
        lfs_url = f"{git_base_url}"
        run_git_command(repo_path, ['git', 'config', 'lfs.url', lfs_url])
        print_blue(f"Set LFS domain to: {lfs_url}")

def extract_crate(crate_path, extract_path):
    def is_within_directory(directory, target):
        # Check if a path is within a directory (for security)
        abs_directory = os.path.abspath(directory)
        abs_target = os.path.abspath(target)
        prefix = os.path.commonprefix([abs_directory, abs_target])
        return prefix == abs_directory

    def safe_extract(tar, path=".", members=None, *, numeric_owner=False):
        # Safely extract files from a tar archive
        for member in tar.getmembers():
            member_path = os.path.join(path, member.name)
            if not is_within_directory(path, member_path):
                raise Exception("Attempted Path Traversal in Tar File")

        def filter_member(tarinfo, filterpath):
            # Filter function to ensure extracted files are within the target directory
            if is_within_directory(path, os.path.join(filterpath, tarinfo.name)):
                return tarinfo
            else:
                return None

        tar.extractall(path, members, numeric_owner=numeric_owner, filter=filter_member)

    try:
        with tarfile.open(crate_path, 'r:gz') as tar:
            if not tar.getmembers():
                print_red(f"Warning: Empty crate file {crate_path}. Skipping extraction.")
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
        print_red(f"Warning: Failed to read crate file {crate_path}. Skipping extraction.")
        return False

def process_crate_version(num, crate_name, version, crate_path, git_repos_dir, git_base_url):
    # Record start time for the entire crate
    crate_start_time = datetime.now()
    print_blue(f"Started processing the crate {crate_name} at {crate_start_time}")

    # Process a specific version of a crate
    repo_path = os.path.join(git_repos_dir, crate_name, version)
    ensure_directory(repo_path)

    # Extract crate directly to the repo directory
    if not extract_crate(crate_path, repo_path):
        print_red(f"Skipping processing for {crate_name} version {version} due to extraction failure.")
        return

    # Initialize git repo
    init_git_repo(repo_path, git_base_url)

    # Add all files to git
    run_git_command(repo_path, ['git', 'add', '.'])

    # Commit changes with updated message format and additional parameters
    commit_message = f"{crate_name} {version}"
    run_git_command(repo_path, ['git', 'commit', '-a', '-s', '-S', '-m', commit_message])

    # Add remote and push
    remote_url = f"{git_base_url}/third-part/rust/crates/{crate_name}/{version}.git"
    run_git_command(repo_path, ['git', 'remote', 'add', 'mega', remote_url])

    # Push to remote
    push_result = run_git_command(repo_path, ['git', 'push', '-u', 'mega', 'main'])
    if push_result is None:
        print_red(f"Warning: Failed to push {crate_name} version {version} to remote repository.")
    else:
        print_green(f"Successfully pushed {crate_name} version {version} to remote repository.")

    # Record end time and calculate duration for the entire crate
    crate_end_time = datetime.now()
    crate_duration = crate_end_time - crate_start_time
    print_blue(f"Finished processing the {num} crate {crate_name} at {crate_end_time}")
    print_blue(f"Total processing time for crate {crate_name}: {crate_duration}")

    # Print separator
    print("------------------")

def process_crate(num, crate_name, versions, crates_dir, git_repos_dir, dl_base_url, git_base_url):
    # Process all versions of a crate
    for v in versions:
        repo_path = os.path.join(git_repos_dir, crate_name, v)
        if os.path.exists(repo_path):
            print_red(f"Repository for {crate_name} version {v} already exists. Skipping.")
            continue

        crate_path = check_and_download_crate(crates_dir, crate_name, v, dl_base_url)
        process_crate_version(num, crate_name, v, crate_path, git_repos_dir, git_base_url)
        num += 1

    print_blue(f"Finished processing  {crate_name}")

def scan_and_process_crates(index_path, crates_dir, git_repos_dir, git_base_url):
    # Scan the crates.io index and process all crates
    crates = defaultdict(set)
    dl_base_url = None

    # Check if the directories exist
    for path in [index_path, crates_dir, git_repos_dir]:
        if not os.path.isdir(path):
            print_red(f"Error: The directory {path} does not exist.")
            sys.exit(1)

    # Read the config.json to get the dl base URL
    config_path = os.path.join(index_path, 'config.json')
    try:
        with open(config_path, 'r') as config_file:
            config = json.load(config_file)
            dl_base_url = config.get('dl')
            if not dl_base_url:
                print_red("Error: 'dl' key not found in config.json")
                sys.exit(1)
    except Exception as e:
        print_red(f"Error reading config.json: {str(e)}")
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
                print_red(f"Error processing file {full_path}: {str(e)}")

    num = 0

    # Process each crate
    for crate_name, versions in crates.items():
        process_crate(num, crate_name, versions, crates_dir, git_repos_dir, dl_base_url, git_base_url)
        num += len(versions)

    return len(crates)

def main():
    # Main function to run the script
    if len(sys.argv) != 5:
        print_red("Usage: python script.py <path_to_crates.io-index> <path_to_crates_directory> <path_to_git_repos_directory> <git_base_url>")
        sys.exit(1)

    index_path, crates_dir, git_repos_dir, git_base_url = sys.argv[1:5]

    total_crates = scan_and_process_crates(index_path, crates_dir, git_repos_dir, git_base_url)
    print_blue(f"\nTotal number of crates processed: {total_crates}")

if __name__ == "__main__":
    main()  # Run the main function if this script is executed directly
