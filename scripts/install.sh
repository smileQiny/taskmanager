#!/usr/bin/env bash
set -euo pipefail

REPO="${TASKMANAGER_REPO:-smileQiny/taskmanager}"
TAG="${TASKMANAGER_VERSION:-latest}"
DOWNLOAD_DIR="${TASKMANAGER_DOWNLOAD_DIR:-}"
LINUX_PACKAGE="${TASKMANAGER_LINUX_PACKAGE:-auto}"
DRY_RUN="${TASKMANAGER_DRY_RUN:-0}"
APP_NAME="Task Manager"

usage() {
  cat <<'EOF'
Install Task Manager from GitHub Releases.

Usage:
  install.sh [--version <tag>] [--download-dir <path>] [--linux-package auto|appimage|deb|rpm] [--dry-run]

Environment:
  TASKMANAGER_REPO             GitHub repo, default: smileQiny/taskmanager
  TASKMANAGER_VERSION          Release tag, default: latest
  TASKMANAGER_DOWNLOAD_DIR     Directory to keep downloaded installer
  TASKMANAGER_LINUX_PACKAGE    auto, appimage, deb, or rpm
  TASKMANAGER_DRY_RUN          Set to 1 to print the selected asset without installing

Examples:
  curl -fsSL https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.sh | bash
  TASKMANAGER_VERSION=v0.2.1 bash scripts/install.sh
  bash scripts/install.sh --linux-package deb
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --self-test)
      SELF_TEST="1"
      shift
      ;;
    --version)
      TAG="${2:-}"
      shift 2
      ;;
    --download-dir)
      DOWNLOAD_DIR="${2:-}"
      shift 2
      ;;
    --linux-package)
      LINUX_PACKAGE="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN="1"
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

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

extract_mount_point() {
  local output="$1"
  local line
  while IFS= read -r line; do
    case "$line" in
      *"/Volumes/"*)
        printf '/Volumes/%s\n' "${line#*/Volumes/}"
        return 0
        ;;
    esac
  done <<<"$output"
  return 1
}

if [ "${SELF_TEST:-0}" = "1" ]; then
  SAMPLE_OUTPUT=$'/dev/disk4           GUID_partition_scheme\n/dev/disk4s1         Apple_HFS                       /Volumes/Task Manager'
  SAMPLE_RESULT="$(extract_mount_point "$SAMPLE_OUTPUT")"
  if [ "$SAMPLE_RESULT" != "/Volumes/Task Manager" ]; then
    echo "Mount point parser self-test failed: ${SAMPLE_RESULT}" >&2
    exit 1
  fi
  echo "Mount point parser self-test passed"
  exit 0
fi

require_cmd curl

if command -v python3 >/dev/null 2>&1; then
  JSON_TOOL="python3"
else
  echo "Missing required command: python3" >&2
  exit 1
fi

case "$LINUX_PACKAGE" in
  auto|appimage|deb|rpm) ;;
  *)
    echo "Invalid --linux-package value: $LINUX_PACKAGE" >&2
    exit 2
    ;;
esac

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin) PLATFORM="macos" ;;
  Linux) PLATFORM="linux" ;;
  *)
    echo "Unsupported OS: $OS. Use scripts/install.ps1 on Windows." >&2
    exit 1
    ;;
esac

case "$ARCH" in
  arm64|aarch64) NORMALIZED_ARCH="arm64" ;;
  x86_64|amd64) NORMALIZED_ARCH="x64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

if [ "$TAG" = "latest" ]; then
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
else
  API_URL="https://api.github.com/repos/${REPO}/releases/tags/${TAG}"
fi

WORK_DIR="$(mktemp -d)"
cleanup() {
  if [ -z "$DOWNLOAD_DIR" ]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

RELEASE_JSON="${WORK_DIR}/release.json"
echo "Fetching release metadata from ${API_URL}"
curl -fsSL \
  -H "Accept: application/vnd.github+json" \
  -H "User-Agent: taskmanager-installer" \
  "$API_URL" \
  -o "$RELEASE_JSON"

ASSET_INFO="$("$JSON_TOOL" - "$RELEASE_JSON" "$PLATFORM" "$NORMALIZED_ARCH" "$LINUX_PACKAGE" <<'PY'
import json
import sys

release_path, platform, arch, linux_package = sys.argv[1:5]
with open(release_path, "r", encoding="utf-8") as handle:
    release = json.load(handle)

assets = release.get("assets") or []

def candidates():
    if platform == "macos":
        if arch == "arm64":
            yield lambda name: name.endswith("_aarch64.dmg")
            yield lambda name: "aarch64" in name and name.endswith(".app.tar.gz")
        else:
            yield lambda name: name.endswith("_x64.dmg")
            yield lambda name: "x64" in name and name.endswith(".app.tar.gz")
        yield lambda name: name.endswith(".dmg")
        return

    linux_order = {
        "auto": [".AppImage", ".deb", ".rpm"],
        "appimage": [".AppImage"],
        "deb": [".deb"],
        "rpm": [".rpm"],
    }[linux_package]
    for suffix in linux_order:
        if arch == "arm64":
            yield lambda name, suffix=suffix: name.endswith(suffix) and (
                "aarch64" in name.lower() or "arm64" in name.lower()
            )
        else:
            yield lambda name, suffix=suffix: name.endswith(suffix) and not (
                "aarch64" in name.lower() or "arm64" in name.lower()
            )

for matcher in candidates():
    for asset in assets:
        name = asset.get("name") or ""
        if matcher(name):
            print(json.dumps({
                "tag": release.get("tag_name") or "",
                "release_url": release.get("html_url") or "",
                "name": name,
                "url": asset.get("browser_download_url") or "",
            }))
            sys.exit(0)

print(f"No compatible installer asset found for {platform}/{arch}", file=sys.stderr)
print("Available assets:", file=sys.stderr)
for asset in assets:
    print(f"- {asset.get('name')}", file=sys.stderr)
sys.exit(1)
PY
)"

TAG_NAME="$("$JSON_TOOL" -c 'import json,sys; print(json.loads(sys.stdin.read())["tag"])' <<<"$ASSET_INFO")"
ASSET_NAME="$("$JSON_TOOL" -c 'import json,sys; print(json.loads(sys.stdin.read())["name"])' <<<"$ASSET_INFO")"
ASSET_URL="$("$JSON_TOOL" -c 'import json,sys; print(json.loads(sys.stdin.read())["url"])' <<<"$ASSET_INFO")"

if [ -n "$DOWNLOAD_DIR" ]; then
  mkdir -p "$DOWNLOAD_DIR"
  TARGET_DIR="$DOWNLOAD_DIR"
else
  TARGET_DIR="$WORK_DIR"
fi

INSTALLER_PATH="${TARGET_DIR}/${ASSET_NAME}"
echo "Selected ${ASSET_NAME} from ${TAG_NAME}"
if [ "$DRY_RUN" = "1" ]; then
  echo "Dry run: would download ${ASSET_URL}"
  exit 0
fi

echo "Downloading to ${INSTALLER_PATH}"
curl -fL \
  -H "User-Agent: taskmanager-installer" \
  "$ASSET_URL" \
  -o "$INSTALLER_PATH"

install_macos_dmg() {
  require_cmd hdiutil
  echo "Mounting ${INSTALLER_PATH}"
  MOUNT_OUTPUT="$(hdiutil attach "$INSTALLER_PATH" -nobrowse)"
  MOUNT_POINT="$(extract_mount_point "$MOUNT_OUTPUT" || true)"
  if [ -z "$MOUNT_POINT" ]; then
    echo "Could not find mounted volume for ${INSTALLER_PATH}" >&2
    exit 1
  fi

  detach() {
    hdiutil detach "$MOUNT_POINT" >/dev/null || true
    cleanup
  }
  trap detach EXIT

  APP_PATH="$(find "$MOUNT_POINT" -maxdepth 2 -name '*.app' -type d | head -n 1)"
  if [ -z "$APP_PATH" ]; then
    echo "No .app bundle found inside ${INSTALLER_PATH}" >&2
    exit 1
  fi

  DESTINATION="/Applications/$(basename "$APP_PATH")"
  echo "Installing ${APP_NAME} to ${DESTINATION}"
  if [ -w /Applications ]; then
    rm -rf "$DESTINATION"
    cp -R "$APP_PATH" /Applications/
  else
    require_cmd sudo
    sudo rm -rf "$DESTINATION"
    sudo cp -R "$APP_PATH" /Applications/
  fi
  echo "Installed ${APP_NAME}. You can open it from /Applications."
}

install_linux_asset() {
  case "$INSTALLER_PATH" in
    *.AppImage)
      TARGET_BIN="${HOME}/.local/bin/task-manager"
      mkdir -p "$(dirname "$TARGET_BIN")"
      cp "$INSTALLER_PATH" "$TARGET_BIN"
      chmod +x "$TARGET_BIN"
      echo "Installed AppImage to ${TARGET_BIN}"
      echo "Run it with: ${TARGET_BIN}"
      ;;
    *.deb)
      require_cmd sudo
      if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get install -y "$INSTALLER_PATH"
      elif command -v dpkg >/dev/null 2>&1; then
        sudo dpkg -i "$INSTALLER_PATH"
      else
        echo "No apt-get or dpkg found for installing .deb packages" >&2
        exit 1
      fi
      ;;
    *.rpm)
      require_cmd sudo
      if command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y "$INSTALLER_PATH"
      elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y "$INSTALLER_PATH"
      elif command -v rpm >/dev/null 2>&1; then
        sudo rpm -Uvh "$INSTALLER_PATH"
      else
        echo "No dnf, yum, or rpm found for installing .rpm packages" >&2
        exit 1
      fi
      ;;
    *)
      echo "Unsupported Linux installer: ${INSTALLER_PATH}" >&2
      exit 1
      ;;
  esac
}

case "$PLATFORM" in
  macos) install_macos_dmg ;;
  linux) install_linux_asset ;;
esac
