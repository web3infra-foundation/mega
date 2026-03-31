#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import tarfile
import urllib.request
from collections import defaultdict
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, as_completed, wait
from datetime import datetime, timezone
from pathlib import Path
import threading
from typing import Dict, Tuple

# ANSI color codes
GREEN = '\033[92m'
BLUE = '\033[94m'
RED = '\033[91m'
RESET = '\033[0m'

VERBOSE = False

def _log(level: str, msg: str) -> None:
    # Standardized, low-noise logging. Use --verbose for command outputs.
    if level == "INFO":
        c = BLUE
    elif level == "WARN":
        c = RED
    elif level == "OK":
        c = GREEN
    else:
        c = ""
    prefix = f"[{level}]"
    if c:
        print(f"{c}{prefix}{RESET} {msg}")
    else:
        print(f"{prefix} {msg}")

def info(msg: str) -> None:
    _log("INFO", msg)

def warn(msg: str) -> None:
    _log("WARN", msg)

def ok(msg: str) -> None:
    _log("OK", msg)

def _fmt_repo(crate_name: str, version: str) -> str:
    return f"{crate_name}@{version}"

def ensure_directory(path):
    # Create a directory if it doesn't exist
    if not os.path.exists(path):
        os.makedirs(path)
        if VERBOSE:
            info(f"Created directory: {path}")

def crates_io_index_rel_path(crate_name: str) -> str:
    """
    Return crates.io index relative path segments for a crate name.

    See: https://doc.rust-lang.org/cargo/reference/registries.html#index-format
    Examples:
      - "a"      -> "1/a"
      - "ab"     -> "2/ab"
      - "abc"    -> "3/a/abc"
      - "tokio"  -> "to/ki/tokio"
    """
    name = crate_name.strip()
    if not name:
        raise ValueError("crate_name is empty")
    n = len(name)
    if n == 1:
        return f"1/{name}"
    if n == 2:
        return f"2/{name}"
    if n == 3:
        return f"3/{name[0]}/{name}"
    return f"{name[0:2]}/{name[2:4]}/{name}"

def crates_io_index_file_path(index_root: str, crate_name: str) -> str:
    # The index "path" is also the on-disk file path inside a crates.io-index checkout.
    return os.path.join(index_root, crates_io_index_rel_path(crate_name))

def mega_third_party_crates_rel_path(crate_name: str, crate_version: str) -> str:
    # Target layout: third-party/rust/crates/<index-path>/<version>/
    # Note: crates.io index path already ends with the crate name (e.g. to/ki/tokio),
    # so do NOT append crate_name again.
    return (
        "third-party/rust/crates/"
        + crates_io_index_rel_path(crate_name)
        + f"/{crate_version}"
    )

def check_and_download_crate(crates_dir, crate_name, crate_version, dl_base_url):
    # Construct the filename and path for the crate
    crate_filename = f"{crate_name}-{crate_version}.crate"
    crate_path = os.path.join(crates_dir, crate_name, crate_filename)

    # Download the crate if it doesn't exist locally
    if not os.path.exists(crate_path):
        ensure_directory(os.path.dirname(crate_path))  # Ensure the directory exists
        download_url = f"{dl_base_url}/{crate_name}/{crate_filename}"
        try:
            info(f"{_fmt_repo(crate_name, crate_version)} download {download_url}")
            urllib.request.urlretrieve(download_url, crate_path)  # Download the file
            if VERBOSE:
                info(f"Downloaded: {crate_path}")
        except Exception as e:
            warn(f"{_fmt_repo(crate_name, crate_version)} download failed: {e}")
    return crate_path

def run_git_command(repo_path, command, *, check: bool = True, log_on_error: bool = True):
    # Run a git command in the specified repository
    try:
        result = subprocess.run(command, cwd=repo_path, check=check, capture_output=True, text=True)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        if log_on_error:
            warn(f"Git command failed: {e}")
            if VERBOSE:
                out = (e.stdout or "").strip()
                err = (e.stderr or "").strip()
                if out:
                    warn(f"stdout: {out}")
                if err:
                    warn(f"stderr: {err}")
        return None

def ensure_git_remote(repo_path: str, remote_name: str, remote_url: str) -> None:
    existing = subprocess.run(
        ["git", "remote", "get-url", remote_name],
        cwd=repo_path,
        capture_output=True,
        text=True,
    )
    if existing.returncode == 0:
        cur = (existing.stdout or "").strip()
        if cur != remote_url:
            run_git_command(repo_path, ["git", "remote", "set-url", remote_name, remote_url])
    else:
        run_git_command(repo_path, ["git", "remote", "add", remote_name, remote_url])

def git_bearer_auth_extra_header(token: str) -> str:
    return f"Authorization: Bearer {token}"

def maybe_wrap_git_with_bearer(cmd: list[str], token: str | None) -> list[str]:
    if not token:
        return cmd
    header = git_bearer_auth_extra_header(token)
    # Use per-command config; do not store credentials in remote URL or global git config.
    return ["git", "-c", f"http.extraHeader={header}", *cmd[1:]]

def _git_has_any_commit(repo_path: str) -> bool:
    res = subprocess.run(
        ["git", "rev-parse", "--verify", "HEAD"],
        cwd=repo_path,
        capture_output=True,
        text=True,
    )
    return res.returncode == 0

def ensure_remote_and_push_existing(
    repo_path: str,
    rel: str,
    git_base_url: str,
    *,
    crate_name: str,
    version: str,
    commit_signoff: bool,
    auth_token: str | None,
    force: bool,
    force_with_lease: bool,
    push_sema: threading.Semaphore,
) -> bool:
    remote_url = f"{git_base_url.rstrip('/')}/{rel}"
    info(f"{_fmt_repo(crate_name, version)} push {rel}")
    if VERBOSE:
        info(f"remote: {remote_url}")
    # Make sure we're on main (and create it if missing)
    run_git_command(repo_path, ['git', 'checkout', '-B', 'main'])

    # If this repo has no commits (e.g. leftover from a previous dry-run),
    # create an initial import commit so `git push main` has a ref.
    if not _git_has_any_commit(repo_path):
        run_git_command(repo_path, ['git', 'add', '-A'])
        msg = f"Import {crate_name} {version}"
        cmd = ['git', 'commit', '--allow-empty', '-m', msg]
        if commit_signoff:
            cmd.append('-s')
        run_git_command(repo_path, cmd)

    ensure_git_remote(repo_path, "mega", remote_url)

    with push_sema:
        push_args = ['git', 'push', '-u', 'mega', 'main']
        if force_with_lease:
            push_args.insert(2, '--force-with-lease')
        elif force:
            push_args.insert(2, '--force')
        push_cmd = maybe_wrap_git_with_bearer(push_args, auth_token)
        res = run_git_command(repo_path, push_cmd, log_on_error=True)
    if res is None:
        # Keep repo on disk for troubleshooting
        return False

    # On success, remove local repo directory to save disk space.
    try:
        shutil.rmtree(repo_path)
        if VERBOSE:
            info(f"Removed local repo (existing): {repo_path}")
    except Exception as e:
        warn(f"Failed to remove local repo {repo_path}: {e}")
    return True

def init_git_repo(repo_path):
    # Initialize a git repository if it doesn't exist
    new_repo = not os.path.exists(os.path.join(repo_path, '.git'))
    if new_repo:
        run_git_command(repo_path, ['git', 'init', '-b', 'main'])
        if VERBOSE:
            info(f"Initialized git repository in {repo_path}")

        # Avoid requiring GPG on fresh machines
        run_git_command(repo_path, ['git', 'config', 'commit.gpgsign', 'false'])
    # Provide a default identity so commits work even when global git config is empty.
    # This is applied both for new repos and existing ones created by older runs.
    run_git_command(repo_path, ['git', 'config', 'user.name', 'crates-sync'])
    run_git_command(repo_path, ['git', 'config', 'user.email', 'crates-sync@example.com'])

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
                warn(f"Empty crate file {crate_path}. Skipping extraction.")
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
        warn(f"Failed to read crate file {crate_path}. Skipping extraction.")
        return False

def process_crate_version(
    num: int,
    crate_name: str,
    version: str,
    crate_path: str,
    git_repos_dir: str,
    git_base_url: str,
    *,
    commit_signoff: bool,
    dry_run: bool,
    auth_token: str | None,
    force: bool,
    force_with_lease: bool,
    push_sema: threading.Semaphore,
) -> bool:
    # Record start time for the entire crate
    crate_start_time = datetime.now()
    if VERBOSE:
        info(f"Started {crate_name} at {crate_start_time}")

    # Process a specific version of a crate
    rel = mega_third_party_crates_rel_path(crate_name, version)
    repo_path = os.path.join(git_repos_dir, rel)
    ensure_directory(repo_path)

    # Extract crate directly to the repo directory
    if not extract_crate(crate_path, repo_path):
        warn(f"{_fmt_repo(crate_name, version)} skipped: extraction failed")
        return False

    # Check for .gitattributes file and remove if it exists
    gitattributes_path = os.path.join(repo_path, '.gitattributes')
    if os.path.exists(gitattributes_path):
        os.remove(gitattributes_path)
        if VERBOSE:
            info(f"Removed .gitattributes file from {repo_path}")

    # Initialize git repo
    init_git_repo(repo_path)

    if dry_run:
        info(f"{_fmt_repo(crate_name, version)} dry-run {rel}")
        return True

    # Add all files to git
    run_git_command(repo_path, ['git', 'add', '.'])

    # Commit changes
    commit_message = f"Import {crate_name} {version}"
    commit_cmd = ['git', 'commit', '--allow-empty', '-m', commit_message]
    if commit_signoff:
        commit_cmd.append('-s')
    run_git_command(repo_path, commit_cmd)

    # Add remote and push (path-style URL, no .git suffix needed)
    remote_url = f"{git_base_url.rstrip('/')}/{rel}"
    ensure_git_remote(repo_path, "mega", remote_url)

    # Push to remote
    with push_sema:
        push_args = ['git', 'push', '-u', 'mega', 'main']
        if force_with_lease:
            push_args.insert(2, '--force-with-lease')
        elif force:
            push_args.insert(2, '--force')
        push_cmd = maybe_wrap_git_with_bearer(push_args, auth_token)
        push_result = run_git_command(repo_path, push_cmd)
    if push_result is None:
        warn(f"{_fmt_repo(crate_name, version)} push failed")
        return False
    else:
        ok(f"{_fmt_repo(crate_name, version)} pushed")
        # On success, remove local repo directory and cached crate to save disk space.
        try:
            shutil.rmtree(repo_path)
            if VERBOSE:
                info(f"Removed local repo: {repo_path}")
        except Exception as e:
            warn(f"Failed to remove local repo {repo_path}: {e}")
        try:
            if os.path.exists(crate_path):
                os.remove(crate_path)
                if VERBOSE:
                    info(f"Removed cached crate file: {crate_path}")
        except Exception as e:
            warn(f"Failed to remove cached crate file {crate_path}: {e}")
        return True

    # Record end time and calculate duration for the entire crate
    crate_end_time = datetime.now()
    crate_duration = crate_end_time - crate_start_time
    if VERBOSE:
        info(f"Finished {crate_name} at {crate_end_time} (duration {crate_duration})")

    # Keep output minimal by default
    if VERBOSE:
        print("------------------")

def stream_index_crate_versions(index_path: str, max_versions_per_crate: int):
    """
    Walk crates.io-index and yield (crate_name, versions_to_process) per index file.
    Each file corresponds to one crate; versions are sorted and optionally trimmed
    before yielding so work can start without scanning the whole tree into memory.
    """
    if not os.path.isdir(index_path):
        warn(f"Error: The directory {index_path} does not exist.")
        sys.exit(1)

    files_seen = 0
    for root, dirs, files in os.walk(index_path):
        dirs[:] = [d for d in dirs if d not in [".git", ".github"]]

        for file in files:
            if file.startswith("."):
                continue
            if (root == index_path and file == "config.json") or file == "README.md":
                continue

            full_path = os.path.join(root, file)
            if not os.path.isfile(full_path):
                continue

            crate_name = file
            versions: set[str] = set()
            try:
                with open(full_path, "r", encoding="utf-8") as f:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                        crate_info = json.loads(line)
                        versions.add(crate_info["vers"])
            except UnicodeDecodeError:
                continue
            except Exception as e:
                warn(f"Error processing file {full_path}: {e}")
                continue

            if not versions:
                continue

            vs = sorted(versions)
            if max_versions_per_crate > 0:
                vs = vs[-max_versions_per_crate:]

            yield crate_name, vs
            files_seen += 1
            if files_seen % 1000 == 0:
                info(f"Index stream progress: processed {files_seen} crate index files...")

def scan_selected_crates_index(index_path: str, crate_names: list[str]):
    """
    Read only the index files for the specified crates.
    This avoids a full os.walk over crates.io-index for small-scale tests.
    """
    crates = defaultdict(set)
    for name in crate_names:
        name = name.strip()
        if not name:
            continue
        fp = crates_io_index_file_path(index_path, name)
        if not os.path.isfile(fp):
            warn(f"Crate '{name}' index file not found: {fp}")
            continue
        try:
            with open(fp, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    crate_info = json.loads(line)
                    crates[crate_info["name"]].add(crate_info["vers"])
        except Exception as e:
            warn(f"Error processing crate index file {fp}: {e}")
    return crates

def list_all_crate_names_from_index_tree(index_path: str) -> list[str]:
    """
    List all crate names by walking the index tree, without parsing JSON lines.
    This is significantly cheaper than scanning every file's contents.
    """
    names: list[str] = []
    files_seen = 0
    for root, dirs, files in os.walk(index_path):
        dirs[:] = [d for d in dirs if d not in ['.git', '.github']]
        for file in files:
            if file.startswith("."):
                continue
            if (root == index_path and file == "config.json") or file == "README.md":
                continue
            full_path = os.path.join(root, file)
            if not os.path.isfile(full_path):
                continue
            # In crates.io-index, the filename is the crate name (for all lengths).
            names.append(file)
            files_seen += 1
            if files_seen % 20000 == 0:
                info(f"Crate-name scan progress: visited {files_seen} index files...")
    # De-dup and keep stable order
    return sorted(set(names))

def load_or_build_crate_name_cache(index_path: str, cache_path: str) -> list[str]:
    cache = Path(cache_path)
    if cache.is_file():
        data = cache.read_text(encoding="utf-8", errors="replace").splitlines()
        names = [x.strip() for x in data if x.strip() and not x.strip().startswith("#")]
        if names:
            return names

    info(f"Building crate name cache (one-time): {cache}")
    names = list_all_crate_names_from_index_tree(index_path)
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_text("\n".join(names) + "\n", encoding="utf-8")
    return names


def load_manifest(manifest_path: str) -> Dict[Tuple[str, str], dict]:
    """Load import status manifest from a JSONL file.

    Keyed by (crate_name, version) -> record dict.
    """
    manifest: Dict[Tuple[str, str], dict] = {}
    p = Path(manifest_path)
    if not p.is_file():
        return manifest
    for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
            key = (rec.get("crate") or "", rec.get("version") or "")
            if key[0] and key[1]:
                manifest[key] = rec
        except Exception:
            # Ignore malformed lines, keep going
            continue
    return manifest


def write_manifest(manifest_path: str, manifest: Dict[Tuple[str, str], dict]) -> None:
    """Rewrite manifest file from in-memory mapping (compact, one line per key)."""
    lines = [json.dumps(rec, sort_keys=True) for rec in manifest.values()]
    p = Path(manifest_path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")


def append_manifest_record(manifest_path: str, rec: dict) -> None:
    """Append one manifest line so progress survives crashes before run end."""
    p = Path(manifest_path)
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open("a", encoding="utf-8") as f:
        f.write(json.dumps(rec, sort_keys=True) + "\n")

def scan_and_process_crates(
    index_path: str,
    crates_dir: str,
    git_repos_dir: str,
    git_base_url: str,
    *,
    limit_crates: int,
    only_crates: list[str],
    max_versions_per_crate: int,
    commit_signoff: bool,
    dry_run: bool,
    repush_existing: bool,
    auth_token: str | None,
    force: bool,
    force_with_lease: bool,
    crate_name_cache: str,
    jobs: int,
    manifest: Dict[Tuple[str, str], dict],
    manifest_path: str,
    reimport_ok: bool,
):
    info("Scanning crates.io index...")

    # Read the config.json to get the dl base URL (needed before any processing)
    config_path = os.path.join(index_path, "config.json")
    try:
        with open(config_path, "r") as config_file:
            config = json.load(config_file)
            dl_base_url = config.get("dl")
            if not dl_base_url:
                warn("Error: 'dl' key not found in config.json")
                sys.exit(1)
    except Exception as e:
        warn(f"Error reading config.json: {e}")
        sys.exit(1)

    use_streaming_full_index = not only_crates and not (limit_crates and limit_crates > 0)

    if use_streaming_full_index:
        info("Streaming index: will download/commit/push while walking crate files.")
    elif only_crates:
        crates = scan_selected_crates_index(index_path, only_crates)
        info(f"Found {len(crates)} crates.")
        crates_items = list(crates.items())
        allow = {c.strip() for c in only_crates if c.strip()}
        crates_items = [(n, v) for (n, v) in crates_items if n in allow]
        info(f"Filtered to {len(crates_items)} crates via --crate.")
    else:
        names = load_or_build_crate_name_cache(index_path, crate_name_cache)
        random.shuffle(names)
        picked = names[:limit_crates]
        info(f"Sampling {len(picked)} crates via --limit-crates (no full content scan).")
        crates = scan_selected_crates_index(index_path, picked)
        info(f"Loaded {len(crates)} crates' versions.")
        crates_items = list(crates.items())
        if VERBOSE:
            info("Shuffling crates list...")
        random.shuffle(crates_items)

    push_sema = threading.BoundedSemaphore(max(1, int(jobs)))
    succeeded = 0
    failed = 0
    skipped = 0
    lock = threading.Lock()

    def process_one(crate_name: str, v: str) -> tuple[str, str, str]:
        key = (crate_name, v)
        rec = manifest.get(key)
        # If already successfully imported, skip unless --reimport-ok (git --force does not disable this).
        if rec and rec.get("status") == "ok" and not reimport_ok:
            info(f"{_fmt_repo(crate_name, v)} already imported; skipping (manifest)")
            return ("skip", crate_name, v)

        rel = mega_third_party_crates_rel_path(crate_name, v)
        repo_path = os.path.join(git_repos_dir, rel)

        # Existing repo path
        if os.path.exists(repo_path) and os.path.exists(os.path.join(repo_path, ".git")):
            if repush_existing and not dry_run:
                ok_push = ensure_remote_and_push_existing(
                    repo_path,
                    rel,
                    git_base_url,
                    crate_name=crate_name,
                    version=v,
                    commit_signoff=commit_signoff,
                    auth_token=auth_token,
                    force=force,
                    force_with_lease=force_with_lease,
                    push_sema=push_sema,
                )
                return ("ok" if ok_push else "fail", crate_name, v)
            else:
                info(f"{_fmt_repo(crate_name, v)} exists; skipping")
                return ("skip", crate_name, v)

        crate_path = check_and_download_crate(crates_dir, crate_name, v, dl_base_url)
        try:
            ok_done = process_crate_version(
                0,
                crate_name,
                v,
                crate_path,
                git_repos_dir,
                git_base_url,
                commit_signoff=commit_signoff,
                dry_run=dry_run,
                auth_token=auth_token,
                force=force,
                force_with_lease=force_with_lease,
                push_sema=push_sema,
            )
            return ("ok" if ok_done else "fail", crate_name, v)
        except Exception as e:
            warn(f"{_fmt_repo(crate_name, v)} failed: {e}")
            return ("fail", crate_name, v)

    def record_result(fut) -> None:
        nonlocal succeeded, failed, skipped
        status, c_name, v = fut.result()
        with lock:
            if status == "ok":
                succeeded += 1
            elif status == "skip":
                skipped += 1
            else:
                failed += 1
            key = (c_name, v)
            rel = mega_third_party_crates_rel_path(c_name, v)
            rec = {
                "crate": c_name,
                "version": v,
                "status": status,
                "remote": f"{git_base_url.rstrip('/')}/{rel}",
                "last_import_time": datetime.now(timezone.utc).isoformat(),
            }
            manifest[key] = rec
            append_manifest_record(manifest_path, rec)

    # Concurrency: bounded pending futures in streaming mode to avoid RAM spikes.
    with ThreadPoolExecutor(max_workers=max(1, int(jobs))) as ex:
        if use_streaming_full_index:
            max_pending = max(32, int(jobs) * 8)
            pending: set = set()
            for crate_name, versions in stream_index_crate_versions(
                index_path, max_versions_per_crate
            ):
                for v in versions:
                    pending.add(ex.submit(process_one, crate_name, v))
                    while len(pending) >= max_pending:
                        done, _ = wait(pending, return_when=FIRST_COMPLETED)
                        for df in done:
                            pending.discard(df)
                            record_result(df)
            for df in as_completed(pending):
                record_result(df)
        else:
            tasks: list[tuple[str, str]] = []
            for crate_name, versions in crates_items:
                vs = sorted(versions)
                if max_versions_per_crate > 0:
                    vs = vs[-max_versions_per_crate:]
                for v in vs:
                    tasks.append((crate_name, v))
            info(f"Starting to process {len(tasks)} crate versions...")
            futures = [ex.submit(process_one, c, v) for c, v in tasks]
            for f in as_completed(futures):
                record_result(f)

    info(f"Summary: ok={succeeded}, skipped={skipped}, failed={failed}")
    # Compact manifest (dedupe append-only history to one line per crate@version).
    write_manifest(manifest_path, manifest)
    return succeeded + skipped + failed


def main():
    # Record start time for the entire process
    total_start_time = datetime.now()
    info(f"Started at {total_start_time}")

    p = argparse.ArgumentParser(prog="crates-sync.py")
    p.add_argument("--index", required=True, help="Path to a local crates.io-index checkout.")
    p.add_argument("--crates-dir", required=True, help="Directory to cache downloaded .crate files.")
    p.add_argument("--workdir", required=True, help="Directory to stage per-crate git repos.")
    p.add_argument("--git-base-url", required=True, help="Mega git base URL, e.g. https://git.gitmega.com")
    # Crates.io `.crate` sources should not require Git LFS; we intentionally do not configure LFS here.
    p.add_argument("--limit-crates", type=int, default=0, help="Process only N crates (0 = no limit).")
    p.add_argument(
        "--max-versions-per-crate",
        type=int,
        default=1,
        help="Keep only the last N versions per crate (0 = all). Default: 1 (small-scale test).",
    )
    p.add_argument(
        "--crate",
        action="append",
        default=[],
        help="Only process this crate name (can be provided multiple times). Overrides shuffle/limit selection.",
    )
    p.add_argument("--signoff", action="store_true", help="Add -s to git commit.")
    p.add_argument("--dry-run", action="store_true", help="Do everything except git add/commit/push.")
    p.add_argument("--verbose", action="store_true", help="Verbose logs (print git stdout/stderr).")
    p.add_argument("--jobs", type=int, default=1, help="Concurrent workers for download/extract/commit (default: 1).")
    p.add_argument(
        "--repush-existing",
        action="store_true",
        help="If target repo already exists in workdir, re-push it instead of skipping (no re-extract).",
    )
    p.add_argument("--force", action="store_true", help="Force-push when pushing (overwrites remote history).")
    p.add_argument(
        "--force-with-lease",
        action="store_true",
        help="Force-push with lease (safer than --force).",
    )
    p.add_argument(
        "--token",
        default=os.environ.get("MEGA_TOKEN", ""),
        help="Bearer token for HTTP auth (or env MEGA_TOKEN).",
    )
    p.add_argument(
        "--crate-name-cache",
        default="",
        help="Path to a cached crate name list (one per line). Used to make --limit-crates fast.",
    )
    p.add_argument(
        "--manifest",
        default="",
        help="Path to an import status manifest (JSONL). Defaults to <workdir>/crates-import-manifest.jsonl.",
    )
    p.add_argument(
        "--reimport-ok",
        action="store_true",
        help="Re-process crate versions even if manifest status is ok (default: skip ok).",
    )
    args = p.parse_args()

    global VERBOSE
    VERBOSE = bool(args.verbose)

    if args.force and args.force_with_lease:
        warn("Error: --force and --force-with-lease are mutually exclusive.")
        sys.exit(2)

    index_path = str(Path(args.index).resolve())
    crates_dir = str(Path(args.crates_dir).resolve())
    git_repos_dir = str(Path(args.workdir).resolve())

    ensure_directory(crates_dir)
    ensure_directory(git_repos_dir)

    auth_token = args.token.strip() or None
    crate_name_cache = args.crate_name_cache.strip()
    if not crate_name_cache:
        crate_name_cache = str(Path(git_repos_dir) / ".crates-io-index-crate-names.txt")

    manifest_path = args.manifest.strip()
    if not manifest_path:
        manifest_path = str(Path(git_repos_dir) / "crates-import-manifest.jsonl")
    manifest = load_manifest(manifest_path)

    total_crates = scan_and_process_crates(
        index_path,
        crates_dir,
        git_repos_dir,
        args.git_base_url,
        limit_crates=args.limit_crates,
        only_crates=args.crate,
        max_versions_per_crate=args.max_versions_per_crate,
        commit_signoff=args.signoff,
        dry_run=args.dry_run,
        repush_existing=args.repush_existing,
        auth_token=auth_token,
        force=args.force,
        force_with_lease=args.force_with_lease,
        crate_name_cache=crate_name_cache,
        jobs=args.jobs,
        manifest=manifest,
        manifest_path=manifest_path,
        reimport_ok=args.reimport_ok,
    )

    # Record end time and calculate duration for the entire process
    total_end_time = datetime.now()
    total_duration = total_end_time - total_start_time
    info(f"Total processed: {total_crates}")
    info(f"Finished at {total_end_time} (duration {total_duration})")

if __name__ == "__main__":
    main()  # Run the main function if this script is executed directly
