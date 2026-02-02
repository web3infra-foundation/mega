#!/usr/bin/env python3

"""
Import Buck2 third-party Rust dependencies into Mega as independent Git repositories.

This script scans a local directory tree (by default <repo-root>/third-party),
finds leaf directories that look like a crate version (contain a BUCK file), and for
each such directory it:

1) Initializes a Git repository rooted at that directory
2) Creates an initial commit (default: Signed-off-by and GPG-signed)
3) Adds a remote pointing to Mega using a path-based URL
4) Pushes the specified branch to Mega

The remote URL is computed as: <git-base-url>/<rel-path-from-third-party>
Default git-base-url is "https://git.gitmega.com".
"""

from __future__ import annotations

import argparse
import collections
import os
import re
import shutil
import subprocess
import sys
import threading
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from queue import Empty, Queue
from typing import Iterable, Optional, Union


SEMVER_DIR_RE = re.compile(r"^\d+\.\d+\.\d+([\-+._][0-9A-Za-z.\-+_]+)?$")
BUCK_FILE_NAMES = ("BUCK",)


@dataclass(frozen=True)
class RepoSpec:
    """A single import target discovered from the filesystem."""

    repo_dir: Path
    rel_path: str
    crate_name: str
    version: str


class GitError(RuntimeError):
    pass


def _eprint(msg: str) -> None:
    print(msg, file=sys.stderr)


def _run(
    args: list[str],
    *,
    cwd: Path,
    check: bool = True,
    capture_output: bool = True,
) -> subprocess.CompletedProcess:
    """Run a subprocess command and return the CompletedProcess."""

    return subprocess.run(
        args,
        cwd=str(cwd),
        check=check,
        capture_output=capture_output,
        text=True,
    )


def _git(
    repo_dir: Path,
    args: list[str],
    *,
    check: bool = True,
) -> subprocess.CompletedProcess:
    """Run a git command in repo_dir and raise GitError on failure."""

    try:
        return _run(["git", *args], cwd=repo_dir, check=check)
    except subprocess.CalledProcessError as e:
        raise GitError(
            "Git command failed: "
            + " ".join(["git", *args])
            + "\n"
            + (e.stdout or "")
            + (e.stderr or "")
        ) from e


def _repo_root(start_dir: Path) -> Path:
    """Return the root of the current git checkout (where this script is run)."""

    try:
        res = _run(["git", "rev-parse", "--show-toplevel"], cwd=start_dir)
        return Path(res.stdout.strip())
    except subprocess.CalledProcessError as e:
        raise RuntimeError(
            "Failed to locate git repository root (git rev-parse --show-toplevel). "
            "Run this script inside a git checkout."
        ) from e


def _iter_dirs(root: Path) -> Iterable[Path]:
    """Iterate all directories under root, skipping common build/VCS directories."""

    def onerror(err: OSError) -> None:
        if err.filename:
            _eprint(f"Warning: failed to access '{err.filename}': {err.strerror}")
        else:
            _eprint(f"Warning: filesystem access failed: {err}")

    for dirpath, dirnames, _ in os.walk(root, onerror=onerror):
        dirnames[:] = [
            d
            for d in dirnames
            if d not in {".git", "buck-out", "target", ".hg", ".svn", "__pycache__"}
        ]
        yield Path(dirpath)


def _is_git_repo_dir(p: Path) -> bool:
    """True if p is a git repository root directory."""

    return (p / ".git").exists()


def _has_buck_file(p: Path) -> bool:
    """True if p contains a Buck build file (used as the version directory marker)."""

    for name in BUCK_FILE_NAMES:
        if (p / name).is_file():
            return True
    return False


def _first_existing_ancestor_git_repo(p: Path, stop_at: Path) -> Optional[Path]:
    """Find nearest ancestor git repo root between p and stop_at (inclusive)."""

    cur = p
    stop_at = stop_at.resolve()
    while True:
        if _is_git_repo_dir(cur):
            return cur
        if cur.resolve() == stop_at:
            return None
        if cur.parent == cur:
            return None
        cur = cur.parent


def _looks_like_version_dir(dir_name: str) -> bool:
    """Heuristic for crate version directories (semver-like by default)."""

    return SEMVER_DIR_RE.match(dir_name) is not None


def _derive_mega_rel_path(scan_root: Path, p: Path) -> str:
    """
    Derive a stable Mega path for p.

    Prefer using the suffix starting at the first "third-party" path component,
    so scanning a directory outside the current git repo still yields consistent
    mega:/third-party/... remotes.
    """

    resolved = p.resolve()
    parts = resolved.parts
    try:
        idx = parts.index("third-party")
        return Path(*parts[idx:]).as_posix()
    except ValueError:
        base = scan_root.resolve().parent
        try:
            return resolved.relative_to(base).as_posix()
        except ValueError as e:
            raise RuntimeError(
                f"Failed to compute Mega path for '{p}'. "
                "Make sure the scanned directory is under a 'third-party/' tree."
            ) from e


def cleanup_git(repo_dir: Path) -> None:
    """Remove the .git directory if it exists."""
    git_dir = repo_dir / ".git"
    if git_dir.exists() and git_dir.is_dir():
        try:
            shutil.rmtree(git_dir)
        except Exception as e:
            _eprint(f"Warning: failed to remove {git_dir}: {e}")



def discover_repos(
    scan_root: Path,
    *,
    include_non_semver: bool,
) -> list[RepoSpec]:
    """Discover crate version directories under scan_root and return RepoSpec list."""

    specs: list[RepoSpec] = []
    for d in _iter_dirs(scan_root):
        if not _has_buck_file(d):
            continue
        if _first_existing_ancestor_git_repo(d, scan_root) is not None:
            continue

        version = d.name
        crate_name = d.parent.name
        if not include_non_semver and not _looks_like_version_dir(version):
            continue

        rel = _derive_mega_rel_path(scan_root, d)
        specs.append(RepoSpec(repo_dir=d, rel_path=rel, crate_name=crate_name, version=version))
    return sorted(specs, key=lambda s: s.rel_path)


def ensure_git_repo(repo_dir: Path, *, branch: str) -> str:
    """Initialize a git repository and ensure the target branch exists."""

    if not _is_git_repo_dir(repo_dir):
        _git(repo_dir, ["init", "-b", branch])
        return "initialized"
    else:
        _git(repo_dir, ["checkout", "-B", branch], check=False)
        return "exists"


def has_any_commit(repo_dir: Path) -> bool:
    """True if the repository already has at least one commit."""

    res = _git(repo_dir, ["rev-parse", "--verify", "HEAD"], check=False)
    return res.returncode == 0


def ensure_initial_commit(
    repo_dir: Path,
    *,
    crate_name: str,
    version: str,
    signoff: bool,
    gpg_sign: bool,
) -> str:
    """Create the initial import commit if the repository has no commits yet."""

    _git(repo_dir, ["add", "-A"])
    if has_any_commit(repo_dir):
        return "skipped_existing"

    msg = f"Import {crate_name} {version}"
    cmd = ["commit", "--allow-empty", "-m", msg]
    if signoff:
        cmd.append("-s")
    if gpg_sign:
        cmd.append("-S")
    _git(repo_dir, cmd)
    return "created"


def ensure_remote(repo_dir: Path, *, remote_name: str, remote_url: str) -> str:
    """Add or update a remote to match remote_url."""

    existing = _git(repo_dir, ["remote", "get-url", remote_name], check=False)
    if existing.returncode == 0:
        if existing.stdout.strip() != remote_url:
            _git(repo_dir, ["remote", "set-url", remote_name, remote_url])
            return "updated"
        return "unchanged"
    _git(repo_dir, ["remote", "add", remote_name, remote_url])
    return "added"


def push(repo_dir: Path, *, remote_name: str, branch: str, force: bool) -> None:
    """Push the branch to remote_name, optionally force-pushing."""

    args = ["push"]
    if force:
        args.append("--force")
    args.extend(["--set-upstream", remote_name, branch])
    res = _git(repo_dir, args, check=False)
    if res.returncode != 0:
        raise GitError((res.stdout or "") + (res.stderr or ""))


def rewrite_buck_deps_paths(repo_dir: Path) -> tuple[int, int]:
    total_replacements = 0
    changed_files = 0
    pattern = re.compile(r"([\"'])//third-party/")

    for name in BUCK_FILE_NAMES:
        p = repo_dir / name
        if not p.is_file():
            continue
        original = p.read_text(encoding="utf-8", errors="replace")
        updated, n = pattern.subn(r"\1//", original)
        if n > 0 and updated != original:
            p.write_text(updated, encoding="utf-8")
            total_replacements += n
            changed_files += 1

    return changed_files, total_replacements


ANSI_RESET = "\033[0m"
ANSI_RED = "\033[31m"
ANSI_GREEN = "\033[32m"
ANSI_YELLOW = "\033[33m"
ANSI_DIM = "\033[2m"

DEFAULT_LOG_LINES = 12


def _fmt_status(level: str, msg: str, *, color: bool) -> str:
    if level == "success":
        symbol = "✔"
        c = ANSI_GREEN
        label = "Success"
    elif level == "warning":
        symbol = "⚠"
        c = ANSI_YELLOW
        label = "Warning"
    else:
        symbol = "✖"
        c = ANSI_RED
        label = "Error"
    if not color:
        return f"{symbol} [{label}] {msg}"
    return f"{c}{symbol} [{label}]{ANSI_RESET} {msg}"


def _truncate_middle(s: str, max_len: int) -> str:
    if len(s) <= max_len:
        return s
    if max_len <= 3:
        return s[:max_len]
    keep = max_len - 3
    left = keep // 2
    right = keep - left
    return s[:left] + "..." + s[-right:]


def _one_line(s: str, max_len: int) -> str:
    normalized = " ".join(s.replace("\r", " ").replace("\n", " ").replace("\t", " ").split())
    return _truncate_middle(normalized, max_len)


class PlainUI:
    def __init__(self, *, total: int) -> None:
        self._total = total
        self._completed = 0
        self._current_rel = ""
        self._current_step = ""
        self._lock = threading.Lock()
        self._log_lines = DEFAULT_LOG_LINES
        self._logs: collections.deque[str] = collections.deque(maxlen=self._log_lines)

    def start(self) -> None:
        self._render()

    def stop(self) -> None:
        sys.stderr.write("\n")
        sys.stderr.flush()

    def apply_event(self, ev: dict) -> None:
        t = ev.get("type")
        with self._lock:
            if t == "slot":
                self._current_rel = ev.get("rel", "") or ""
                self._current_step = ev.get("step", "") or ""
            elif t == "done":
                self._completed += 1
            elif t == "log":
                level = ev.get("level", "success")
                msg = ev.get("msg", "")
                self._logs.append(_fmt_status(level, msg, color=True))
                sys.stderr.write("\n" + self._logs[-1] + "\n")
            self._render()

    def _render(self) -> None:
        rel = _truncate_middle(self._current_rel, 70)
        step = _truncate_middle(self._current_step, 50)
        line = f"[{self._completed}/{self._total}] {rel}"
        if step:
            line += f"  {ANSI_DIM}({step}){ANSI_RESET}"
        sys.stderr.write("\r" + line + " " * 10)
        sys.stderr.flush()


def _get_rich():
    try:
        from rich.console import Console, Group
        from rich.live import Live
        from rich.panel import Panel
        from rich.progress import BarColumn, Progress, TaskProgressColumn, TextColumn, TimeElapsedColumn, TimeRemainingColumn
        from rich.table import Table

        return Console, Group, Live, Panel, Progress, Table, BarColumn, TextColumn, TaskProgressColumn, TimeElapsedColumn, TimeRemainingColumn
    except Exception:
        return None


class RichUI:
    def __init__(self, *, total: int, jobs: int) -> None:
        rich = _get_rich()
        if rich is None:
            raise RuntimeError("rich is not available")
        (
            Console,
            Group,
            Live,
            Panel,
            Progress,
            Table,
            BarColumn,
            TextColumn,
            TaskProgressColumn,
            TimeElapsedColumn,
            TimeRemainingColumn,
        ) = rich
        self._Console = Console
        self._Group = Group
        self._Live = Live
        self._Panel = Panel
        self._Progress = Progress
        self._Table = Table
        self._BarColumn = BarColumn
        self._TextColumn = TextColumn
        self._TaskProgressColumn = TaskProgressColumn
        self._TimeElapsedColumn = TimeElapsedColumn
        self._TimeRemainingColumn = TimeRemainingColumn

        self._console = Console(stderr=True)
        self._total = total
        self._jobs = jobs
        self._log_lines = DEFAULT_LOG_LINES
        self._logs: collections.deque[tuple[str, str]] = collections.deque(maxlen=self._log_lines)
        self._slots: dict[int, tuple[str, str]] = {i: ("", "") for i in range(1, self._jobs + 1)}
        self._final: Optional[tuple[int, int, list[tuple[str, str]]]] = None

        self._progress = Progress(
            TextColumn("[bold]Overall[/bold]"),
            BarColumn(),
            TaskProgressColumn(),
            TimeElapsedColumn(),
            TimeRemainingColumn(),
            console=self._console,
            transient=False,
            refresh_per_second=10,
        )
        self._task_id = self._progress.add_task("crates", total=self._total)
        self._live = None

    def start(self) -> None:
        self._live = self._Live(self._render(), console=self._console, refresh_per_second=10)
        self._live.__enter__()

    def stop(self) -> None:
        if self._live is not None:
            try:
                self._live.update(
                    self._render(include_current_operations=False, include_logs=False, include_results=True),
                    refresh=True,
                )
            finally:
                self._live.__exit__(None, None, None)
            self._live = None

    def apply_event(self, ev: dict) -> None:
        t = ev.get("type")
        if t == "slot":
            slot = int(ev.get("slot", 0) or 0)
            rel = ev.get("rel", "") or ""
            step = ev.get("step", "") or ""
            if slot in self._slots:
                self._slots[slot] = (rel, step)
        elif t == "clear":
            slot = int(ev.get("slot", 0) or 0)
            if slot in self._slots:
                self._slots[slot] = ("", "")
        elif t == "done":
            self._progress.advance(self._task_id, 1)
        elif t == "log":
            level = ev.get("level", "success")
            msg = ev.get("msg", "") or ""
            if level == "success":
                self._logs.append(("green", f"✔ [Success] {msg}"))
            elif level == "warning":
                self._logs.append(("yellow", f"⚠ [Warning] {msg}"))
            else:
                self._logs.append(("red", f"✖ [Error] {msg}"))
        elif t == "final":
            total = int(ev.get("total", 0) or 0)
            succeeded = int(ev.get("succeeded", 0) or 0)
            failures = ev.get("failures", []) or []
            cleaned_failures: list[tuple[str, str]] = []
            for item in failures:
                if not isinstance(item, dict):
                    continue
                rel = str(item.get("rel", "") or "")
                reason = str(item.get("reason", "") or "")
                if rel:
                    cleaned_failures.append((rel, reason))
            self._final = (total, succeeded, cleaned_failures)

        if self._live is not None:
            self._live.update(self._render(), refresh=True)

    def _render(
        self,
        *,
        include_current_operations: bool = True,
        include_logs: bool = True,
        include_results: bool = False,
    ):
        Table = self._Table
        Panel = self._Panel
        Group = self._Group

        panels = [Panel(self._progress, border_style="blue")]
        if include_current_operations:
            status = Table(show_header=True, header_style="bold")
            status.add_column("Worker", style="dim", width=8, no_wrap=True)
            status.add_column("Crate", overflow="fold")
            status.add_column("Step", overflow="fold")
            for slot in range(1, self._jobs + 1):
                rel, step = self._slots.get(slot, ("", ""))
                status.add_row(str(slot), rel or "-", step or "-")
            panels.append(Panel(status, title="Current Operations", border_style="cyan"))
        if include_logs:
            logs = Table(show_header=False, padding=(0, 1))
            logs.add_column("line", overflow="fold")
            if self._logs:
                for color, line in self._logs:
                    logs.add_row(f"[{color}]{line}[/{color}]")
            else:
                logs.add_row("[dim]No messages yet.[/dim]")
            panels.append(Panel(logs, title="Logs", border_style="white"))
        if include_results:
            results_summary = Table(show_header=False, padding=(0, 1))
            results_summary.add_column("k", style="dim", width=10, no_wrap=True)
            results_summary.add_column("v", overflow="fold")
            results_body = results_summary

            if self._final is None:
                results_summary.add_row("Status", "[yellow]No final results.[/yellow]")
            else:
                total, succeeded, failures = self._final
                failed = len(failures)
                results_summary.add_row("Succeeded", f"[green]{succeeded}[/green]")
                results_summary.add_row("Failed", f"[red]{failed}[/red]")
                results_summary.add_row("Total", str(total))

                if failures:
                    failed_table = Table(show_header=True, header_style="bold", padding=(0, 1))
                    failed_table.add_column("Repo", style="red", overflow="fold")
                    failed_table.add_column("Reason", overflow="fold")
                    for rel, reason in failures:
                        failed_table.add_row(rel, reason or "-")
                    results_body = Group(results_summary, failed_table)

            panels.append(Panel(results_body, title="Results", border_style="green"))

        return Group(*panels)


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse CLI arguments."""

    p = argparse.ArgumentParser(prog="import-buck2-deps.py")
    p.add_argument(
        "--scan-root",
        default="",
        help="Directory to scan (default: <repo-root>/third-party).",
    )
    p.add_argument(
        "--git-base-url",
        default="https://git.gitmega.com",
        help='Git base URL (default: "https://git.gitmega.com"). Full remote becomes <git-base-url>/<rel-path>.',
    )
    p.add_argument("--remote-name", default="mega", help='Git remote name (default: "mega").')
    p.add_argument("--branch", default="main", help='Branch name to create/push (default: "main").')
    p.add_argument(
        "--include-non-semver",
        action="store_true",
        help="Also import directories whose last path segment is not semver-like.",
    )
    p.add_argument(
        "--buckal-generated",
        action="store_true",
        help='Rewrite BUCK deps labels from "//third-party/..." to "//..." before committing.',
    )
    p.add_argument(
        "--no-signoff",
        action="store_true",
        help="Do not pass -s to git commit.",
    )
    p.add_argument(
        "--no-gpg-sign",
        action="store_true",
        help="Do not pass -S to git commit.",
    )
    p.add_argument("--force", action="store_true", help="Force-push when pushing.")
    p.add_argument("--dry-run", action="store_true", help="Print actions without running git.")
    p.add_argument(
        "--jobs",
        type=int,
        default=1,
        help="Number of concurrent workers (default: 1).",
    )
    p.add_argument(
        "--ui",
        choices=("auto", "rich", "plain"),
        default="auto",
        help='Output mode: "auto" uses rich if available (default: auto).',
    )
    p.add_argument(
        "--fail-fast",
        action="store_true",
        help="Stop scheduling new work after the first error.",
    )
    p.add_argument(
        "--limit",
        type=int,
        default=0,
        help="Only process first N discovered repos (0 means no limit).",
    )
    p.add_argument(
        "--retry",
        type=int,
        default=0,
        help="Number of retries for failed pushes (default: 0).",
    )
    return p.parse_args(argv)


def main(argv: list[str]) -> int:
    """Entry point. Returns process exit code."""

    args = parse_args(argv)
    if args.scan_root:
        scan_root = Path(args.scan_root).resolve()
    else:
        repo_root = _repo_root(Path.cwd())
        scan_root = repo_root / "third-party"

    if not scan_root.exists():
        _eprint(f"Error: scan root does not exist: {scan_root}")
        return 2
    if not scan_root.is_dir():
        _eprint(f"Error: scan root is not a directory: {scan_root}")
        return 2
    try:
        os.listdir(scan_root)
    except PermissionError as e:
        _eprint(f"Error: no permission to read scan root '{scan_root}': {e}")
        return 2
    except OSError as e:
        _eprint(f"Error: failed to read scan root '{scan_root}': {e}")
        return 2

    try:
        specs = discover_repos(
            scan_root,
            include_non_semver=args.include_non_semver,
        )
    except Exception as e:
        _eprint(f"Error: failed to scan '{scan_root}': {e}")
        return 2

    seen = set()
    uniq_specs: list[RepoSpec] = []
    for s in specs:
        if s.rel_path in seen:
            continue
        seen.add(s.rel_path)
        uniq_specs.append(s)

    if args.limit and args.limit > 0:
        uniq_specs = uniq_specs[: args.limit]

    if not uniq_specs:
        print("No import candidates found.")
        return 0

    total_candidates = len(uniq_specs)
    print(f"Found {total_candidates} import candidates.")

    def plan_line(s: RepoSpec) -> str:
        remote_url = args.git_base_url.rstrip("/") + "/" + s.rel_path.lstrip("/")
        return f"{s.rel_path} -> {remote_url}"

    if args.dry_run:
        for s in uniq_specs:
            print(f"- {plan_line(s)}")
        return 0
    use_rich = False
    if args.ui in ("auto", "rich"):
        use_rich = _get_rich() is not None
        if args.ui == "rich" and not use_rich:
            _eprint("Error: requested --ui rich but the 'rich' library is not installed.")
            return 2

    if not use_rich:
        if total_candidates <= 50:
            for s in uniq_specs:
                print(f"- {plan_line(s)}")
        else:
            print("Tip: use --dry-run to print the full candidate list.")

    signoff = not args.no_signoff
    gpg_sign = not args.no_gpg_sign

    jobs = max(1, int(args.jobs))

    pending_specs = list(uniq_specs)
    retries = args.retry
    
    # We will accumulate final failures if the last attempt still fails.
    # Or should we just exit non-zero if the last attempt fails?
    # The requirement is to retry.
    
    final_exit_code = 0

    for attempt in range(retries + 1):
        if not pending_specs:
            break
            
        current_total = len(pending_specs)
        if attempt > 0:
            print(f"\n[Retry {attempt}/{retries}] Retrying {current_total} repositories...")

        ui: Union[PlainUI, RichUI]
        if use_rich:
            ui = RichUI(total=current_total, jobs=jobs)
        else:
            ui = PlainUI(total=current_total)

        events: Queue[dict] = Queue()
        slot_pool: Queue[int] = Queue()
        for i in range(1, jobs + 1):
            slot_pool.put(i)
        stop_event = threading.Event()
        failures: list[tuple[RepoSpec, str]] = []
        failures_lock = threading.Lock()
        successes: list[str] = []
        successes_lock = threading.Lock()

        def emit(ev: dict) -> None:
            events.put(ev)

        def process_one(s: RepoSpec) -> None:
            if stop_event.is_set():
                emit({"type": "log", "level": "warning", "msg": f"{s.rel_path}: skipped (fail-fast)"})
                emit({"type": "done", "rel": s.rel_path})
                return

            slot = slot_pool.get()
            try:
                if stop_event.is_set():
                    emit({"type": "log", "level": "warning", "msg": f"{s.rel_path}: skipped (fail-fast)"})
                    emit({"type": "done", "rel": s.rel_path})
                    return

                remote_url = args.git_base_url.rstrip("/") + "/" + s.rel_path.lstrip("/")
                emit({"type": "slot", "slot": slot, "rel": s.rel_path, "step": "git init/checkout"})
                git_state = ensure_git_repo(s.repo_dir, branch=args.branch)
                if git_state != "initialized":
                    emit({"type": "log", "level": "warning", "msg": f"{s.rel_path}: existing git repo"})

                if args.buckal_generated:
                    emit({"type": "slot", "slot": slot, "rel": s.rel_path, "step": "rewrite BUCK deps"})
                    _, total_replacements = rewrite_buck_deps_paths(s.repo_dir)
                    if total_replacements > 0:
                        emit(
                            {
                                "type": "log",
                                "level": "warning",
                                "msg": f"{s.rel_path}: rewrote BUCK deps labels ({total_replacements} replacements)",
                            }
                        )

                emit({"type": "slot", "slot": slot, "rel": s.rel_path, "step": "commit"})
                commit_state = ensure_initial_commit(
                    s.repo_dir,
                    crate_name=s.crate_name,
                    version=s.version,
                    signoff=signoff,
                    gpg_sign=gpg_sign,
                )
                if commit_state != "created":
                    emit({"type": "log", "level": "warning", "msg": f"{s.rel_path}: already has commits"})

                emit({"type": "slot", "slot": slot, "rel": s.rel_path, "step": "remote"})
                remote_state = ensure_remote(s.repo_dir, remote_name=args.remote_name, remote_url=remote_url)
                if remote_state == "updated":
                    emit({"type": "log", "level": "warning", "msg": f"{s.rel_path}: remote updated"})

                emit({"type": "slot", "slot": slot, "rel": s.rel_path, "step": "push"})
                push(
                    s.repo_dir,
                    remote_name=args.remote_name,
                    branch=args.branch,
                    force=args.force,
                )
                with successes_lock:
                    successes.append(s.rel_path)
                emit({"type": "done", "rel": s.rel_path})
                emit({"type": "log", "level": "success", "msg": f"{s.rel_path}: imported"})
            except Exception as e:
                err_text = str(e)
                emit({"type": "log", "level": "error", "msg": f"{s.rel_path}: {_one_line(err_text, 10000)}"})
                with failures_lock:
                    failures.append((s, str(e)))
                if args.fail_fast:
                    stop_event.set()
                emit({"type": "done", "rel": s.rel_path})
            finally:
                cleanup_git(s.repo_dir)
                emit({"type": "clear", "slot": slot})
                slot_pool.put(slot)

        ui.start()
        try:
            with ThreadPoolExecutor(max_workers=jobs) as executor:
                futures = [executor.submit(process_one, s) for s in pending_specs]
                def drain_events() -> None:
                    while True:
                        try:
                            ev = events.get_nowait()
                        except Empty:
                            return
                        ui.apply_event(ev)

                while True:
                    if stop_event.is_set():
                        for f in futures:
                            f.cancel()
                    try:
                        ev = events.get(timeout=0.1)
                        ui.apply_event(ev)
                    except Empty:
                        pass
                    if all(f.done() for f in futures):
                        drain_events()
                        break

                with failures_lock:
                    failures_snapshot = list(failures)
                with successes_lock:
                    succeeded_count = len(successes)
                ui.apply_event(
                    {
                        "type": "final",
                        "total": current_total,
                        "succeeded": succeeded_count,
                        "failures": [{"rel": s.rel_path, "reason": msg} for s, msg in failures_snapshot],
                    }
                )
        finally:
            ui.stop()

        if not failures_snapshot:
            # Success!
            final_exit_code = 0
            break
        else:
            # Prepare for next retry
            pending_specs = [f[0] for f in failures_snapshot]
            final_exit_code = 1
            if attempt == retries:
                # Last attempt failed
                _eprint(f"\nFailed to import {len(pending_specs)} repositories after {attempt + 1} attempts.")

    if not use_rich and final_exit_code == 0:
        print("\nAll imports completed successfully.")
        
    return final_exit_code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
