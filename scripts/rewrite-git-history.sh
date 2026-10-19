#!/usr/bin/env bash
# Rewrite a local mirror so generated artifacts are removed from history.
#
# This script never pushes and never updates a remote ref. Publishing the
# result onto main requires a force-push that the repository owner must run
# after review. See docs/history-cleanup.md.

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/rewrite-git-history.sh [--source DIR] [--dest DIR] [--force-dest]

Clone DIR (default: this repository) into a new mirror at DEST and run
git filter-repo --invert-paths on these paths:

  node_modules/
  src-tauri/target/
  graphify-out/
  src-tauri/gen/
  .claude/

The script prints before/after pack size and the rewritten main commit.
It does not push. There is no flag that pushes.

Requires git-filter-repo (https://github.com/newren/git-filter-repo).
EOF
}

SOURCE=""
DEST=""
FORCE_DEST=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)
      SOURCE="${2:-}"
      shift 2
      ;;
    --dest)
      DEST="${2:-}"
      shift 2
      ;;
    --force-dest)
      FORCE_DEST=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$SOURCE" ]]; then
  SOURCE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi
SOURCE="$(cd "$SOURCE" && pwd)"

if ! git -C "$SOURCE" rev-parse --is-inside-work-tree >/dev/null 2>&1 \
  && ! git -C "$SOURCE" rev-parse --is-bare-repository >/dev/null 2>&1; then
  echo "Source is not a git repository: $SOURCE" >&2
  exit 1
fi

if ! git -C "$SOURCE" rev-parse --verify main >/dev/null 2>&1; then
  echo "Source has no main branch: $SOURCE" >&2
  exit 1
fi

if [[ -z "$DEST" ]]; then
  DEST="$(mktemp -d "${TMPDIR:-/tmp}/taskmanager-history.XXXXXX")"
  rm -rf "$DEST"
fi

if [[ "$DEST" == "$SOURCE" || "$DEST" == "$SOURCE/.git" ]]; then
  echo "Refusing to rewrite the source repository in place." >&2
  exit 1
fi

if [[ -e "$DEST" ]]; then
  if [[ "$FORCE_DEST" -ne 1 ]]; then
    echo "Destination already exists: $DEST" >&2
    echo "Pass --force-dest to replace it." >&2
    exit 1
  fi
  rm -rf "$DEST"
fi

FILTER_REPO=""
if command -v git-filter-repo >/dev/null 2>&1; then
  FILTER_REPO="$(command -v git-filter-repo)"
elif python3 -m git_filter_repo --version >/dev/null 2>&1; then
  FILTER_REPO="python3 -m git_filter_repo"
else
  echo "git-filter-repo is not installed." >&2
  echo "Install it with: python3 -m pip install --user git-filter-repo" >&2
  exit 1
fi

echo "Source: $SOURCE"
echo "Mirror: $DEST"
echo "Tool: $FILTER_REPO"
echo "This command does not push."

echo
echo "=== Before (reachable from main) ==="
before_clone="$(mktemp -d "${TMPDIR:-/tmp}/taskmanager-before.XXXXXX")"
rm -rf "$before_clone"
git clone --quiet --no-local --bare --single-branch --branch main "file://${SOURCE}" "$before_clone"
git -C "$before_clone" count-objects -vH
du -sh "$before_clone"
old_main="$(git -C "$SOURCE" rev-parse main)"
old_tree="$(git -C "$SOURCE" rev-parse 'main^{tree}')"
old_count="$(git -C "$SOURCE" rev-list --count main)"
echo "main $old_main"
echo "tree $old_tree"
echo "commits $old_count"

run_filter_repo() {
  # --force is required because the fresh mirror still has an origin remote.
  # That remote is the local source path. filter-repo removes it and does not push.
  if [[ "$FILTER_REPO" == "python3 -m git_filter_repo" ]]; then
    python3 -m git_filter_repo --invert-paths \
      --path node_modules/ \
      --path src-tauri/target/ \
      --path graphify-out/ \
      --path src-tauri/gen/ \
      --path .claude/ \
      --force
  else
    "$FILTER_REPO" --invert-paths \
      --path node_modules/ \
      --path src-tauri/target/ \
      --path graphify-out/ \
      --path src-tauri/gen/ \
      --path .claude/ \
      --force
  fi
}

echo
echo "=== Rewriting mirror ==="
git clone --quiet --mirror "$SOURCE" "$DEST"
( cd "$DEST" && run_filter_repo )

echo
echo "=== After (mirror, all rewritten refs) ==="
git -C "$DEST" count-objects -vH
du -sh "$DEST"
new_main="$(git -C "$DEST" rev-parse main)"
new_tree="$(git -C "$DEST" rev-parse 'main^{tree}')"
new_count="$(git -C "$DEST" rev-list --count main)"
echo "main $new_main"
echo "tree $new_tree"
echo "commits $new_count"

leftovers="$(git -C "$DEST" rev-list --objects --all \
  | awk 'NF>1 { $1=""; sub(/^ /,""); print }' \
  | grep -E '^(node_modules/|src-tauri/target/|graphify-out/|src-tauri/gen/|\.claude/)' \
  || true)"
if [[ -n "$leftovers" ]]; then
  echo "Removed paths are still present:" >&2
  echo "$leftovers" >&2
  exit 1
fi
echo "Removed paths are absent from all rewritten refs."

if [[ "$old_tree" == "$new_tree" ]]; then
  echo "main tree is unchanged ($new_tree)."
else
  echo "main tree changed." >&2
  echo "That means main still tracked one of the removed paths." >&2
  echo "Inspect the result before publishing it." >&2
  exit 1
fi

if git -C "$DEST" remote get-url origin >/dev/null 2>&1; then
  echo "Unexpected origin remote remains on the mirror." >&2
  exit 1
fi

rm -rf "$before_clone"

cat <<EOF

Rewritten mirror is ready at:
  $DEST

Review it, then follow docs/history-cleanup.md.
Do not force-push main from this script. The owner must publish the
reviewed history explicitly.
EOF
