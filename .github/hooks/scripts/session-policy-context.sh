#!/usr/bin/env bash
set -euo pipefail

# Hook payload is available on stdin. This hook only emits context.
cat >/dev/null

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

python3 - <<'PY'
import json
import subprocess


def git_changed_paths():
    try:
        raw = subprocess.check_output(
            ["git", "status", "--porcelain=v1", "--untracked-files=no", "-z"],
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        return []

    parts = raw.decode("utf-8", "replace").split("\0")
    paths = []
    i = 0
    while i < len(parts):
        entry = parts[i]
        if not entry:
            i += 1
            continue

        status = entry[:2]
        path = entry[3:] if len(entry) > 3 else ""

        # In -z mode, rename/copy records include old path in entry and new path as next item.
        if status and status[0] in {"R", "C"}:
            if i + 1 < len(parts) and parts[i + 1]:
                path = parts[i + 1]
                i += 1

        if path and not path.startswith("target/"):
            paths.append(path)
        i += 1

    return paths


def is_yaml(path: str) -> bool:
    return path.endswith(".yaml") or path.endswith(".yml")


def detect_flags(paths):
    flags = {
        "rust_touched": False,
        "config_touched": False,
        "docs_touched": False,
        "backlog_pair_incomplete": False,
    }

    touched = set(paths)
    board = "doc/backlog/TRACKING_BOARD.md"
    backlog = "doc/backlog/BACKLOG.md"

    if (board in touched) ^ (backlog in touched):
        flags["backlog_pair_incomplete"] = True

    for path in paths:
        if path.endswith(".rs") and (path.startswith("src/") or path.startswith("tests/")):
            flags["rust_touched"] = True

        if (
            path.startswith("src/config/")
            or path.startswith("src/import/")
            or path.startswith("src/cli/")
            or (path.startswith("etc/") and is_yaml(path))
        ):
            flags["config_touched"] = True

        if (
            path.startswith("doc/user/")
            or path == "README.md"
        ):
            flags["docs_touched"] = True

    return flags


paths = git_changed_paths()
flags = detect_flags(paths)

summary_paths = paths[:12]
if summary_paths:
    changed_summary = ", ".join(summary_paths)
    if len(paths) > 12:
        changed_summary += ", ..."
else:
    changed_summary = "none detected"

notes = []
notes.append(f"Changed files: {changed_summary}.")

if flags["config_touched"] and not flags["docs_touched"]:
    notes.append(
        "Docs policy: config/import/CLI surfaces changed; verify doc impact and update doc/user/ or README.md in the same task, or track explicit deferment in backlog docs."
    )

if flags["backlog_pair_incomplete"]:
    notes.append(
        "Backlog policy: BACKLOG.md and TRACKING_BOARD.md should be synchronized together when ticket status/dependencies/registry are updated."
    )

if flags["rust_touched"]:
    notes.append(
        "Rust validation reminder: run cargo fmt --all --check, cargo clippy --all-targets --all-features -- -D warnings, and cargo test --quiet when Rust code changes."
    )

if len(notes) == 1:
    notes.append("No policy follow-up detected from current changed-file set.")

message = " ".join(notes)
print(json.dumps({"continue": True, "systemMessage": message}))
PY
