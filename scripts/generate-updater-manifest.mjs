#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';

const installerRules = [
  {
    platformKeys: ['darwin-aarch64', 'darwin-aarch64-app'],
    match: (name) => name.endsWith('_aarch64.app.tar.gz'),
  },
  {
    platformKeys: ['darwin-x86_64', 'darwin-x86_64-app'],
    match: (name) => name.endsWith('_x64.app.tar.gz'),
  },
  {
    platformKeys: ['windows-x86_64', 'windows-x86_64-nsis'],
    match: (name) => name.endsWith('_x64-setup.exe'),
  },
  {
    platformKeys: ['linux-x86_64', 'linux-x86_64-appimage'],
    match: (name) => name.endsWith('_amd64.AppImage'),
  },
  {
    platformKeys: ['linux-x86_64-deb'],
    match: (name) => name.endsWith('_amd64.deb'),
  },
  {
    platformKeys: ['linux-x86_64-rpm'],
    match: (name) => name.endsWith('.x86_64.rpm'),
  },
];

export async function buildUpdaterManifest({
  assets,
  notes,
  pubDate,
  readSignature,
  version,
}) {
  const platforms = {};

  for (const rule of installerRules) {
    const installer = assets.find((asset) => rule.match(asset.name));
    if (!installer) continue;
    const signatureAsset = assets.find((asset) => asset.name === `${installer.name}.sig`);
    if (!signatureAsset) {
      throw new Error(`Missing signature asset for ${installer.name}`);
    }
    const entry = {
      signature: await readSignature(signatureAsset.name, signatureAsset),
      url: installer.browser_download_url,
    };
    for (const key of rule.platformKeys) {
      platforms[key] = entry;
    }
  }

  return {
    version,
    notes,
    pub_date: pubDate,
    platforms,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const assetsJson = args.assetsJson ?? args.assets;
  if (!assetsJson || !args.out || !args.version) {
    console.error('Usage: node scripts/generate-updater-manifest.mjs --assets <assets.json> --version <semver> --notes <text> --pub-date <iso> --out <latest.json>');
    process.exit(1);
  }

  const assets = JSON.parse(readFileSync(assetsJson, 'utf8'));
  const manifest = await buildUpdaterManifest({
    assets,
    notes: args.notes ?? '',
    pubDate: args.pubDate ?? new Date().toISOString(),
    readSignature: async (_name, asset) => {
      if (!asset.local_path) {
        throw new Error(`Signature asset ${asset.name} is missing local_path`);
      }
      return readFileSync(asset.local_path, 'utf8').trim();
    },
    version: args.version.replace(/^v/i, ''),
  });

  writeFileSync(args.out, `${JSON.stringify(manifest, null, 2)}\n`);
}

function parseArgs(argv) {
  const args = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith('--')) continue;
    args[toCamelCase(arg.slice(2))] = argv[index + 1];
    index += 1;
  }
  return args;
}

function toCamelCase(value) {
  return value.replace(/-([a-z])/g, (_match, letter) => letter.toUpperCase());
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
