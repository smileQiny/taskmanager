#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const version = process.argv[2];
const checkOnly = version === '--check';
const versionPattern = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

if (!checkOnly && (!version || !versionPattern.test(version))) {
  console.error('Usage: npm run version:set -- <semver>');
  console.error('Example: npm run version:set -- 0.2.1');
  process.exit(1);
}

const packagePath = join(root, 'package.json');
const packageLockPath = join(root, 'package-lock.json');
const tauriConfigPath = join(root, 'src-tauri', 'tauri.conf.json');
const cargoTomlPath = join(root, 'src-tauri', 'Cargo.toml');

const packageJson = readJson(packagePath);
const packageLock = readJson(packageLockPath);
const tauriConfig = readJson(tauriConfigPath);
const cargoToml = readFileSync(cargoTomlPath, 'utf8');

if (checkOnly) {
  const versions = {
    'package.json': packageJson.version,
    'package-lock.json': packageLock.version,
    'package-lock root package': packageLock.packages?.['']?.version,
    'tauri.conf.json': tauriConfig.version,
    'Cargo.toml': cargoToml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1],
  };
  const uniqueVersions = new Set(Object.values(versions));
  if (uniqueVersions.size !== 1) {
    console.error('Version files are not in sync:');
    for (const [file, value] of Object.entries(versions)) {
      console.error(`- ${file}: ${value ?? 'missing'}`);
    }
    process.exit(1);
  }
  console.log(`Version files are in sync at ${packageJson.version}`);
  process.exit(0);
}

packageJson.version = version;
packageLock.version = version;
if (packageLock.packages?.['']) {
  packageLock.packages[''].version = version;
}
tauriConfig.version = version;

const nextCargoToml = cargoToml.replace(
  /^(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
  `$1${version}$2`,
);

if (nextCargoToml === cargoToml) {
  console.error('Could not find [package] version in src-tauri/Cargo.toml');
  process.exit(1);
}

writeJson(packagePath, packageJson);
writeJson(packageLockPath, packageLock);
writeJson(tauriConfigPath, tauriConfig);
writeFileSync(cargoTomlPath, nextCargoToml);
console.log(`Set project version to ${version}`);

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}
