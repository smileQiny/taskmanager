import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import {
  DEV_CSP_NONCE,
  devCsp,
  devServerConnectExtras,
  productionCsp,
  serializeCsp,
} from "./scripts/cspPolicy.mjs";

const host = process.env.TAURI_DEV_HOST;
const packageJson = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as { version: string };
const productionCspHeader = serializeCsp(productionCsp);

// https://vitejs.dev/config/
export default defineConfig(async ({ command, isPreview }) => {
  const devServer = command === "serve" && !isPreview;
  const devCspHeader = serializeCsp(devCsp(devServerConnectExtras(host)));

  return {
    plugins: [react()],
    define: {
      __APP_VERSION__: JSON.stringify(packageJson.version),
    },
    // Dev-only placeholder. Production relies on Tauri's per-response nonces
    // and script hashes; a fixed nonce must not be baked into the packaged HTML.
    html: devServer ? { cspNonce: DEV_CSP_NONCE } : undefined,

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      headers: devServer
        ? { "Content-Security-Policy": devCspHeader }
        : undefined,
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    },
    preview: {
      port: 4173,
      strictPort: true,
      headers: {
        "Content-Security-Policy": productionCspHeader,
      },
    },
  };
});
