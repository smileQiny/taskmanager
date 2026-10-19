import { readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  DEV_CSP_NONCE,
  devCsp,
  devServerConnectExtras,
  frontendUrlsNotFetched,
  productionCsp,
  rustOnlyProviderHosts,
  serializeCsp,
} from "./cspPolicy.mjs";

function collectUrlHosts(root, extensions, ignore = () => false) {
  const hosts = new Set();
  const visit = (dir) => {
    for (const entry of readdirSync(dir)) {
      const path = join(dir, entry);
      if (statSync(path).isDirectory()) {
        if (entry === "node_modules" || entry === "target") continue;
        visit(path);
        continue;
      }
      if (ignore(path) || !extensions.includes(extname(path))) continue;
      const text = readFileSync(path, "utf8");
      for (const match of text.matchAll(/https?:\/\/([A-Za-z0-9.-]+)/g)) {
        hosts.add(match[1].toLowerCase());
      }
    }
  };
  visit(root);
  return [...hosts].sort();
}

function httpsHosts(sources) {
  return sources
    .map((source) => source.match(/^https:\/\/([^/:]+)/)?.[1])
    .filter(Boolean);
}

describe("content security policy", () => {
  const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const readme = readFileSync("README.md", "utf8");
  const viteConfig = readFileSync("vite.config.ts", "utf8");
  const productionHeader = serializeCsp(productionCsp);

  it("keeps the packaged webview on the production policy", () => {
    expect(config.app.security.csp).toEqual(productionCsp);
    expect(config.app.security.devCsp).toEqual(devCsp());
    expect(config.app.security.dangerousDisableAssetCspModification).toBeUndefined();
    expect(config.app.security.useHttpsScheme).not.toBe(true);
  });

  it("does not use wildcards, unsafe-eval, or script unsafe-inline", () => {
    for (const policy of [productionCsp, devCsp(), devCsp(devServerConnectExtras("10.0.0.8"))]) {
      for (const [directive, sources] of Object.entries(policy)) {
        for (const source of sources) {
          expect(source.includes("*"), `${directive} ${source}`).toBe(false);
          expect(source).not.toBe("'unsafe-eval'");
          expect(source).not.toBe("'wasm-unsafe-eval'");
          if (source === "'unsafe-inline'") {
            expect(directive).toBe("style-src-attr");
          }
        }
      }
    }
    expect(productionHeader).not.toContain("upgrade-insecure-requests");
    expect(productionCsp["frame-src"]).toEqual(["'none'"]);
    expect(productionCsp["font-src"]).toEqual(["'self'"]);
    expect(productionCsp["img-src"].some((source) => source.startsWith("https:"))).toBe(false);
  });

  it("allows Tauri IPC and the GitHub hosts the update check follows", () => {
    expect(productionCsp["connect-src"]).toEqual(expect.arrayContaining([
      "ipc:",
      "http://ipc.localhost",
      "https://github.com",
      "https://api.github.com",
      "https://release-assets.githubusercontent.com",
    ]));
    expect(productionCsp["script-src"]).toEqual(["'self'"]);
    expect(devCsp()["script-src"]).toContain(`'nonce-${DEV_CSP_NONCE}'`);
    expect(devCsp()["style-src"]).toContain(`'nonce-${DEV_CSP_NONCE}'`);
  });

  it("leaves calendar and loopback hosts to Rust instead of the webview", () => {
    const connectHosts = httpsHosts(productionCsp["connect-src"]);
    for (const host of rustOnlyProviderHosts) {
      expect(connectHosts, host).not.toContain(host);
    }
    expect(productionHeader).not.toContain("127.0.0.1");

    const rustHosts = collectUrlHosts("src-tauri/src", [".rs"]);
    expect(rustHosts).toEqual([...rustOnlyProviderHosts, "127.0.0.1"].sort());
  });

  it("covers every URL host referenced by the frontend", () => {
    const connectHosts = new Set(httpsHosts([
      ...productionCsp["connect-src"],
      ...devCsp()["connect-src"],
    ]));
    const frontendHosts = collectUrlHosts("src", [".ts", ".tsx", ".css"], (path) => (
      path.endsWith(".test.ts") || path.endsWith(".test.tsx")
    ));
    const indexHosts = collectUrlHosts(".", [".html"], (path) => path !== "index.html");
    for (const host of [...frontendHosts, ...indexHosts]) {
      const allowed = connectHosts.has(host) || frontendUrlsNotFetched.includes(host);
      expect(allowed, `${host} is not in connect-src or frontendUrlsNotFetched`).toBe(true);
    }
  });

  it("rejects dev-host values that could inject extra directives", () => {
    expect(devServerConnectExtras("")).toEqual([]);
    expect(devServerConnectExtras("127.0.0.1")).toEqual([
      "http://127.0.0.1:1420",
      "ws://127.0.0.1:1420",
      "http://127.0.0.1:1421",
      "ws://127.0.0.1:1421",
    ]);
    expect(devServerConnectExtras("evil.com; script-src *")).toEqual([]);
    expect(devServerConnectExtras("..")).toEqual([]);
  });

  it("keeps the Vite preview and the README on the same production policy", () => {
    expect(viteConfig).toContain("cspPolicy.mjs");
    expect(viteConfig).toContain("Content-Security-Policy");
    expect(viteConfig).toContain("isPreview");
    expect(viteConfig).toContain("cspNonce");
    expect(readme).toContain(productionHeader);
    expect(readme).toContain("style-src-attr");
    expect(readme).not.toContain("webview CSP is also still a follow-up");
  });
});
