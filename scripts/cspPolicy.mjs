/**
 * Webview Content Security Policy for the Tauri app and the Vite preview.
 *
 * Production deliberately has no `unsafe-eval` and no script `unsafe-inline`.
 * Tauri appends a per-response style/script nonce and script hashes when it
 * serves the packaged HTML. Vite's production bundle is external files, so
 * `'self'` is enough for `npm run preview`.
 *
 * `style-src-attr 'unsafe-inline'` stays because CSP nonces do not apply to
 * style attributes. The cockpit transparency and pomodoro ring set those
 * attributes at runtime.
 *
 * Calendar OAuth and API calls are not webview requests. See
 * `rustOnlyProviderHosts`. Do not add those hosts unless the webview itself
 * starts loading them.
 */

export const DEV_CSP_NONCE = "taskmanager-dev";

export const productionCsp = {
  "default-src": ["'self'"],
  "script-src": ["'self'"],
  "style-src": ["'self'"],
  "style-src-attr": ["'unsafe-inline'"],
  "img-src": ["'self'", "asset:", "http://asset.localhost"],
  "font-src": ["'self'"],
  "connect-src": [
    "'self'",
    "ipc:",
    "http://ipc.localhost",
    "https://github.com",
    "https://api.github.com",
    "https://release-assets.githubusercontent.com",
  ],
  "frame-src": ["'none'"],
  "object-src": ["'none'"],
  "base-uri": ["'self'"],
  "form-action": ["'self'"],
  "frame-ancestors": ["'none'"],
};

/** Vite dev server origins. Ports match vite.config.ts (1420 app, 1421 HMR). */
export const devConnectExtras = [
  "http://localhost:1420",
  "http://127.0.0.1:1420",
  "ws://localhost:1420",
  "ws://127.0.0.1:1420",
  "http://localhost:1421",
  "http://127.0.0.1:1421",
  "ws://localhost:1421",
  "ws://127.0.0.1:1421",
];

/**
 * Hosts the Rust process and the system browser use for calendar sync.
 * They are intentionally absent from the webview policy.
 */
export const rustOnlyProviderHosts = [
  "accounts.feishu.cn",
  "accounts.google.com",
  "accounts.larksuite.com",
  "oauth2.googleapis.com",
  "open.feishu.cn",
  "open.larksuite.com",
  "qyapi.weixin.qq.com",
  "www.googleapis.com",
];

/** URL hosts that appear in frontend source but are not fetched by the webview. */
export const frontendUrlsNotFetched = [
  "127.0.0.1",
  "raw.githubusercontent.com",
];

const safeDevHost = /^[A-Za-z0-9.-]+$/;

export function devServerConnectExtras(host) {
  if (!host || !safeDevHost.test(host) || host.includes("..")) return [];
  return [1420, 1421].flatMap((port) => [
    `http://${host}:${port}`,
    `ws://${host}:${port}`,
  ]);
}

export function devCsp(extraConnectSources = []) {
  return {
    ...productionCsp,
    "script-src": ["'self'", `'nonce-${DEV_CSP_NONCE}'`],
    "style-src": ["'self'", `'nonce-${DEV_CSP_NONCE}'`],
    "connect-src": [
      ...productionCsp["connect-src"],
      ...devConnectExtras,
      ...extraConnectSources,
    ],
  };
}

export function serializeCsp(policy) {
  return Object.entries(policy)
    .map(([directive, sources]) => `${directive} ${sources.join(" ")}`)
    .join("; ");
}
