# TaskManager

Local-first desktop task manager built with Tauri 2, React 19, TypeScript, Rust, and SQLite.

## Current Features

- Local task CRUD with title, description, status, priority, schedule, all-day flag, and tags.
- Today workspace with overdue, today, and unscheduled task groups.
- Task workspace with search, status filter, priority filter, and sorting.
- Calendar workspace with month/week/day views, click-to-create slots, and task editing.
- Pomodoro workspace with local timer and completed session records.
- Settings workspace with theme, default calendar view, local data path, and sync provider configuration.
- Startup and manual update checks against the latest GitHub Release installer.
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

## GitHub Installer Updates

The app does not use Tauri's signed updater yet. It checks `smileQiny/taskmanager` on GitHub for the latest published Release, compares the release tag with the local app version, and opens the matching installer or Release page for the user to install.

Release flow:

```bash
npm run version:set -- <next-version>
npm run version:check
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "Release v<next-version>"
git tag v<next-version>
git push origin main v<next-version>
```

Pushing a `v*` tag runs `.github/workflows/build.yml`, builds macOS, Windows, and Linux installers with `tauri-apps/tauri-action`, and publishes them to the GitHub Release. The app's update check only sees published releases, so keep the tag version aligned with `package.json`.
