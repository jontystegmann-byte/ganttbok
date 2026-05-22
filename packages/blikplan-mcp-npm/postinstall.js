#!/usr/bin/env node
// postinstall.js
// Downloads the blikplan-mcp binary for the current platform from a
// GitHub release and places it at bin/blikplan-mcp (or bin/blikplan-mcp.exe
// on Windows). Based on the same pattern as the `esbuild` npm package.
//
// Environment variables:
//   BLIKPLAN_MCP_VERSION  — override the binary version to download
//                           (default: matches npm package version)
//   BLIKPLAN_MCP_SKIP_DOWNLOAD — set to "1" to skip download (CI / offline)

'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

if (process.env.BLIKPLAN_MCP_SKIP_DOWNLOAD === '1') {
  console.log('blikplan-mcp: skipping binary download (BLIKPLAN_MCP_SKIP_DOWNLOAD=1)');
  process.exit(0);
}

const pkg = require('./package.json');
const version = process.env.BLIKPLAN_MCP_VERSION || pkg.version;

// Map Node.js platform/arch to the Rust target triple used in release filenames.
function platformToTriple() {
  const p = process.platform;
  const a = process.arch;

  if (p === 'darwin' && a === 'arm64') return 'aarch64-apple-darwin';
  if (p === 'darwin' && a === 'x64')   return 'x86_64-apple-darwin';
  if (p === 'linux'  && a === 'x64')   return 'x86_64-unknown-linux-gnu';
  if (p === 'linux'  && a === 'arm64') return 'aarch64-unknown-linux-gnu';
  if (p === 'win32'  && a === 'x64')   return 'x86_64-pc-windows-msvc';

  throw new Error(
    `blikplan-mcp: unsupported platform ${p}/${a}.\n` +
    'Please open an issue at https://github.com/jontystegmann-byte/ganttbok'
  );
}

const triple = platformToTriple();
const isWindows = process.platform === 'win32';
const binaryName = isWindows ? 'blikplan-mcp.exe' : 'blikplan-mcp';
const assetName = isWindows
  ? `blikplan-mcp-${triple}.exe`
  : `blikplan-mcp-${triple}`;

const downloadUrl =
  `https://github.com/jontystegmann-byte/ganttbok/releases/download/` +
  `mcp-v${version}/${assetName}`;

const binDir = path.join(__dirname, 'bin');
const destPath = path.join(binDir, binaryName);

if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

console.log(`blikplan-mcp: downloading ${downloadUrl}`);

function download(url, dest, redirectCount = 0) {
  if (redirectCount > 5) {
    throw new Error('Too many redirects');
  }
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest + '.tmp');
    https.get(url, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        file.close(() => fs.unlinkSync(dest + '.tmp'));
        resolve(download(res.headers.location, dest, redirectCount + 1));
        return;
      }
      if (res.statusCode !== 200) {
        file.close(() => fs.unlinkSync(dest + '.tmp'));
        reject(new Error(`HTTP ${res.statusCode} from ${url}`));
        return;
      }
      res.pipe(file);
      file.on('finish', () => {
        file.close(() => {
          fs.renameSync(dest + '.tmp', dest);
          resolve();
        });
      });
    }).on('error', (err) => {
      file.close(() => fs.unlinkSync(dest + '.tmp'));
      reject(err);
    });
  });
}

download(downloadUrl, destPath)
  .then(() => {
    if (!isWindows) {
      fs.chmodSync(destPath, 0o755);
    }
    console.log(`blikplan-mcp: installed to ${destPath}`);
  })
  .catch((err) => {
    console.error(`blikplan-mcp: download failed — ${err.message}`);
    console.error(
      'You can set BLIKPLAN_MCP_SKIP_DOWNLOAD=1 to skip the download ' +
      'and provide the binary yourself.'
    );
    process.exit(1);
  });
