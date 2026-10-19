# TaskManager

Local-first desktop task manager built with Tauri 2, React 19, TypeScript, Rust, and SQLite.

## Current Features

- Local task CRUD with title, description, status, priority, schedule, all-day flag, and tags. `recurrence` is reserved on the task schema only; the UI does not edit or sync repeating events yet.
- Light and dark themes, loaded with the rest of the settings on startup, including the cockpit window. The default calendar view is applied when the calendar opens.
- Today workspace with overdue, today, and unscheduled task groups.
- Task workspace with search, status filter, priority filter, and sorting.
- Calendar workspace with month/week/day views, click-to-create slots, and task editing.
- Pomodoro workspace with local timer and completed session records.
- Settings workspace with theme, default calendar view, local data path, and sync provider configuration.
- Startup and manual signed updates from the latest GitHub Release.
- Remote sync for opted-in tasks with Feishu/Lark Calendar, macOS Calendar, Google Calendar, and WeCom (企业微信) schedules created by a corp app.

## Google Calendar

Google sync uses a desktop OAuth client, PKCE, and a loopback redirect on `127.0.0.1`. Tokens stay in the OS keychain when it is available, otherwise in an encrypted local table. They are not returned to the frontend and must not be committed.

1. In Google Cloud Console, enable the Google Calendar API.
2. Configure the OAuth consent screen and add the Google accounts that will test the app.
3. Create an OAuth client of type Desktop. Copy the Client ID. A client secret is optional for this PKCE flow.
4. The requested scope is `https://www.googleapis.com/auth/calendar.events`.
5. In Task Manager settings, paste the Client ID and calendar ID (`primary` or another calendar ID), then choose Connect Google. Do not put these credentials in the repository.

Opt in per task, then sync. If the same event changed both locally and in Google since the last sync, the task is marked as a conflict and neither side is overwritten until you keep the local task or the Google event.

Apple notarization is still a follow-up. A custom URL scheme, multiple Google accounts, and importing unrelated Google events are also out of scope. The webview Content Security Policy does not list Google hosts; see [Content Security Policy](#content-security-policy).

## Feishu / Lark Calendar

Feishu sync uses the Open Platform OAuth 2.0 code flow with PKCE and a fixed loopback redirect. Feishu apps are confidential clients, so the desktop flow sends both the PKCE verifier and the App Secret. Tokens and the App Secret stay in the OS keychain when it is available, otherwise in an encrypted local table. They are not returned to the frontend and must not be committed.

Feishu calendar events do not expose an update timestamp. After the first successful sync, Task Manager stores a content fingerprint of the event. A later edit on only one side is copied across. If both the task and the Feishu event changed since that fingerprint, the task is marked as a conflict and neither side is overwritten until you keep the local task or the Feishu event.

1. Open [Feishu Open Platform](https://open.feishu.cn/app) for Feishu China, or [Lark Developer](https://open.larksuite.com/app) for Lark international. Create an enterprise custom app.
2. On Credentials & Basic Info, copy the App ID and App Secret.
3. On Permissions & Scopes, enable these user-identity scopes, then publish a version so they take effect:
   - `offline_access`
   - `calendar:calendar:readonly`
   - `calendar:calendar.event:read`
   - `calendar:calendar.event:create`
   - `calendar:calendar.event:update`
4. On Security Settings, add this exact redirect URL:

   ```text
   http://127.0.0.1:43821/callback
   ```

   If the page shows a switch for refreshing `user_access_token`, turn it on and publish again. The redirect port is fixed because the console must whitelist the full callback URL; `localhost` is not the same URL as `127.0.0.1`.
5. Make the app available to the Feishu or Lark account that will sync, and publish the app version.
6. In Task Manager settings, choose 飞书 or Lark, paste the App ID and App Secret, and set the calendar ID to `primary` or a specific calendar ID. Choose Connect Feishu. Do not put these credentials in the repository.

The requested scope string is `offline_access calendar:calendar:readonly calendar:calendar.event:read calendar:calendar.event:create calendar:calendar.event:update`. Authorization opens in the system browser and returns to the loopback redirect above. Opt in per task, then sync. The browser preview reports an explicit desktop-only error and does not mark the sync as successful.

All-day events use an exclusive UTC end date, the same way Google events do. Recurrence is not synced.

## WeCom Calendar

WeCom schedule APIs do not offer a user OAuth calendar scope. A self-built corp app calls `gettoken` with its CorpID and Secret, then creates and updates schedules on a calendar that same app owns. Task Manager stores the Secret and the corp access token in the OS keychain when it is available, otherwise in the encrypted local secret table. They are not returned to the frontend and must not be committed. There is no browser redirect URL.

WeCom schedule objects do not include an update timestamp. After the first successful sync, Task Manager stores a content fingerprint. A later edit on only one side is copied across. If both the task and the WeCom schedule changed since that fingerprint, the task is marked as a conflict and neither side is overwritten until you keep the local task or the WeCom schedule.

Limits, compared with Feishu or Google:

- Only schedules on calendars created by this app can be read or updated. Personal schedules the member created in WeCom, and schedules created by another app, are outside the API.
- The configured member userid is added as the schedule admin and attendee so the event shows on that member's calendar. The userid must be inside the app's visible range.
- New self-built apps must list this computer's public IP under the app's 企业可信IP. Calls from any other IP fail and are reported as errors.
- Titles are limited to 128 characters and descriptions to 1000 characters. A hidden `[taskmanager:<task id>]` marker is appended to the description so a retry can find the schedule instead of creating a duplicate. That marker is removed before the text is written back to the local task.
- All-day events use an exclusive end at local midnight, using the nearest whole-hour UTC offset because WeCom timezones are whole hours. Recurrence is not synced.
- The browser preview reports an explicit desktop-only error and does not mark the sync as successful.

Setup:

1. In the WeCom admin console, create a self-built app. Copy the CorpID, AgentId, and Secret.
2. Add the app's visible range so it includes the member who should see the schedules. Copy that member's userid.
3. Open 协作 → 日程 and add this app to 可调用接口的应用. Publish or save so the schedule permission is active.
4. In the app details, set 企业可信IP to the public IP of each desktop that will sync. A new app cannot call the API until this is set. Allow about a minute after saving the IP.
5. In Task Manager settings, paste the CorpID, AgentId, Secret, and member userid. Leave the calendar ID as `primary` to create a Task Manager calendar on connect, or paste a `cal_id` this same app already created. Choose 连接企业微信. Do not put these credentials in the repository.

Opt in per task, then sync. Disconnect deletes the saved access token and keeps the Secret on this machine until you clear the Secret field.

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

## Content Security Policy

The packaged webview and `npm run preview` use one production policy. The source of truth is `scripts/cspPolicy.mjs`. The same objects are copied into `src-tauri/tauri.conf.json` as `app.security.csp` and `app.security.devCsp`. `npm test` fails if the copies drift.

Production header:

```text
default-src 'self'; script-src 'self'; style-src 'self'; style-src-attr 'unsafe-inline'; img-src 'self' data: asset: http://asset.localhost; font-src 'self'; connect-src 'self' ipc: http://ipc.localhost https://github.com https://api.github.com https://release-assets.githubusercontent.com; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'
```

What that allows:

- **Scripts.** Same origin only. There is no `unsafe-inline` and no `unsafe-eval`. Tauri keeps its default asset rewriting, which adds a per-response nonce and script hashes when the desktop app serves HTML. Do not set `dangerousDisableAssetCspModification`.
- **Styles.** Same-origin stylesheets. `style-src-attr 'unsafe-inline'` remains because the cockpit transparency and the pomodoro ring set dynamic style attributes, and a CSP nonce cannot cover a style attribute. If a webview ignores `style-src-attr`, move `'unsafe-inline'` onto `style-src` and keep it off `script-src`.
- **Images.** `'self'`, `data:`, the Tauri `asset:` scheme, and `http://asset.localhost`. `data:` is there for the daisyUI loading spinner, which masks an inline SVG. No remote image hosts.
- **Fonts.** `'self'`. The UI uses system fonts and does not load a font CDN.
- **Connections.** Tauri IPC (`ipc:` and `http://ipc.localhost`, because `useHttpsScheme` is off) and the GitHub hosts the in-webview update check actually requests. `latest.json` redirects from `github.com` to `release-assets.githubusercontent.com`.
- **Frames.** `'none'`. OAuth is not loaded in an iframe.

`npm run dev` and `npm run tauri dev` use a looser dev policy. It adds the fixed nonce `taskmanager-dev` to `script-src` and `style-src` so Vite can run the React refresh preamble and CSS hot reload without `unsafe-inline` or `unsafe-eval`. It also allows the Vite HTTP and websocket origins on `localhost` and `127.0.0.1`, ports 1420 and 1421. That nonce is a development placeholder. It is applied only while the Vite dev server is running, and it is not written into the production bundle. `npm run preview` sends the production header.

### Calendar hosts and loopback

Google, Feishu/Lark, and WeCom calls are made by the Rust process with `reqwest`. Authorization pages open in the system browser. The Feishu callback `http://127.0.0.1:43821/callback` and the ephemeral Google loopback are TCP listeners in Rust. The webview never fetches them, so they are not CSP sources. Allowing `http://127.0.0.1:*` would let an injected script reach every local port and would not make OAuth work.

Rust-only hosts, from `rustOnlyProviderHosts` in `scripts/cspPolicy.mjs`:

- `accounts.google.com`, `oauth2.googleapis.com`, `www.googleapis.com`
- `accounts.feishu.cn`, `open.feishu.cn`
- `accounts.larksuite.com`, `open.larksuite.com`
- `qyapi.weixin.qq.com`

### Adding a host

When a new provider is loaded **by the webview** (a `fetch`, an image, or a frame), add the exact origin to `productionCsp` in `scripts/cspPolicy.mjs`, copy `productionCsp` and `devCsp()` into `src-tauri/tauri.conf.json`, and run `npm test`. Do not use `*`. If `TAURI_DEV_HOST` is set, `vite.config.ts` appends that hostname for ports 1420 and 1421; the value must be a plain hostname, not a CSP fragment. If GitHub changes the `latest.json` redirect host, add that host to `connect-src` the same way.

## Verification

```bash
npm test
npx tsc --noEmit
npm run build
npm run preview
cd src-tauri && cargo test && cargo check
```

`npm run preview` serves the production bundle with the production Content Security Policy header. `npm run dev` at `http://127.0.0.1:1420` uses the dev policy so hot reload works, and it uses localStorage fallback data so UI flows can be tested outside the Tauri webview. The packaged desktop app uses Tauri IPC and SQLite. Desktop OAuth still opens in the system browser.

`graphify-out/` is generated graph-analysis output. It is gitignored and should not be committed.

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

The macOS/Linux script installs the matching DMG or Linux package for the current machine. On macOS it also clears the quarantine flag after copying the app, which avoids the "app is damaged" Gatekeeper prompt that can appear after browser downloads of unsigned builds. On Linux it prefers AppImage, then deb, then rpm. The Windows script downloads and runs the NSIS `.exe` installer.

## Signed App Updates

The app uses Tauri's signed updater for in-app updates. GitHub Releases remain the distribution channel: first-time installs use the scripts above, and running apps check `latest.json` from the latest Release. Manual macOS DMG installs are still not Apple-notarized; use the one-line install script or configure Apple Developer ID signing and notarization for click-to-open DMG distribution.

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

The public key is committed in `src-tauri/tauri.conf.json`. Do not commit the private key. Apple notarization is still a follow-up, so downloaded macOS builds can need the install script's quarantine cleanup. Google OAuth itself runs in the system browser and the Rust process, not inside the webview, so those hosts are not part of the webview Content Security Policy.

Release flow:

```bash
npm run version:set -- <next-version>
npm run version:check
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "Release v<next-version>"
git tag v<next-version>
git push origin main v<next-version>
```

Pushing a `v*` tag runs `.github/workflows/build.yml`, builds macOS, Windows, and Linux installers with `tauri-apps/tauri-action`, signs updater artifacts, uploads `latest.json`, and publishes everything to the GitHub Release. Normal branch and PR builds merge `src-tauri/tauri.ci.conf.json` so they do not need updater signing secrets. The app's update check only sees published releases, so keep the tag version aligned with `package.json`.
