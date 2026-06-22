# TaskManager

Local-first desktop task manager built with Tauri 2, React 19, TypeScript, Rust, and SQLite.

## Current Features

- Local task CRUD with title, description, status, priority, schedule, all-day flag, and tags.
- Today workspace with overdue, today, and unscheduled task groups.
- Task workspace with search, status filter, priority filter, and sorting.
- Calendar workspace with month/week/day views, click-to-create slots, and task editing.
- Pomodoro workspace with local timer and completed session records.
- Settings workspace with theme, default calendar view, local data path, and sync provider configuration.
- Startup and manual signed updates from the latest GitHub Release.
- Remote sync provider boundary for Feishu, macOS Calendar, WeCom, and Google Calendar. Live provider sync still requires platform-specific authorization credentials.

## Development

```bash
npm install
npm run dev
```

Run the desktop app:

```bash
npm run tauri dev
```

Set the app version in every required file:

```bash
npm run version:set -- <next-version>
npm run version:check
```

## Verification

```bash
npm test
npm run build
cd src-tauri && cargo test && cargo check
```

Browser preview at `http://127.0.0.1:1420` uses localStorage fallback data so UI flows can be tested outside the Tauri WebView. The packaged desktop app uses Tauri IPC and SQLite.

## One-Line Install

Install the latest published GitHub Release directly from `smileQiny/taskmanager`.

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.ps1 | iex
```

Optional controls:

```bash
# Install a specific release tag.
curl -fsSL https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.sh | bash -s -- --version v0.2.1

# Keep the downloaded installer.
curl -fsSL https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.sh | bash -s -- --download-dir "$HOME/Downloads"

# Linux: choose a package type instead of auto-selecting AppImage, deb, then rpm.
curl -fsSL https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.sh | bash -s -- --linux-package deb
```

```powershell
# Install a specific release tag.
$env:TASKMANAGER_VERSION = "v0.2.1"; irm https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.ps1 | iex

# Keep the downloaded Windows installer.
$env:TASKMANAGER_DOWNLOAD_DIR = "$env:USERPROFILE\Downloads"; irm https://raw.githubusercontent.com/smileQiny/taskmanager/main/scripts/install.ps1 | iex
```

The macOS/Linux script installs the matching DMG or Linux package for the current machine. On Linux it prefers AppImage, then deb, then rpm. The Windows script downloads and runs the NSIS `.exe` installer.

## Signed App Updates

The app uses Tauri's signed updater for in-app updates. GitHub Releases remain the distribution channel: first-time installs use the scripts above, and running apps check `latest.json` from the latest Release.

Required GitHub Actions secret:

```text
TAURI_SIGNING_PRIVATE_KEY
```

Set this secret to the contents of your private key file, for example:

```bash
cat ~/.tauri/taskmanager-updater.key
```

If the key was generated with a password, also set:

```text
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

The public key is committed in `src-tauri/tauri.conf.json`. Do not commit the private key.

Release flow:

```bash
npm run version:set -- <next-version>
npm run version:check
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "Release v<next-version>"
git tag v<next-version>
git push origin main v<next-version>
```

Pushing a `v*` tag runs `.github/workflows/build.yml`, builds macOS, Windows, and Linux installers with `tauri-apps/tauri-action`, signs updater artifacts, uploads `latest.json`, and publishes everything to the GitHub Release. The app's update check only sees published releases, so keep the tag version aligned with `package.json`.
