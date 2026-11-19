#!/usr/bin/env node
'use strict';

// npx clipstack install     -> copy the binary somewhere stable, register autostart, launch
// npx clipstack uninstall   -> stop it, remove autostart entry and the installed binary
// npx clipstack start|stop|status
// npx clipstack <anything else> -> passed straight to the binary (pop, pop-oldest, clear, ...)

const { spawnSync, spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const PKG = require('../package.json');
const platform = require('../lib/platform');

function fail(msg) {
  console.error(`clipstack: ${msg}`);
  process.exit(1);
}

/** The binary shipped in the matching @clipstack/<os>-<arch> package. */
function bundledBinary() {
  const key = `${process.platform}-${process.arch}`;
  const pkgName = `@clipstack/${key}`;
  const exe = process.platform === 'win32' ? 'clipstack.exe' : 'clipstack';
  try {
    const pkgJson = require.resolve(`${pkgName}/package.json`);
    const bin = path.join(path.dirname(pkgJson), 'bin', exe);
    if (fs.existsSync(bin)) return bin;
  } catch (_) { /* not installed */ }
  return null;
}

function requireBinary() {
  const bin = bundledBinary();
  if (bin) return bin;
  fail(
    `no prebuilt binary for ${process.platform}-${process.arch}.\n` +
    `  Supported: ${Object.keys(PKG.optionalDependencies).map((n) => n.replace('@clipstack/', '')).join(', ')}\n` +
    `  Build from source instead: https://github.com/sharbuz/clipstack#build-from-source`,
  );
}

const cmd = process.argv[2];
const rest = process.argv.slice(3);

switch (cmd) {
  case 'install': {
    const src = requireBinary();
    const dest = platform.installedBinaryPath();
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    platform.stop();
    fs.copyFileSync(src, dest);
    if (process.platform !== 'win32') fs.chmodSync(dest, 0o755);
    platform.registerAutostart(dest);
    platform.start(dest);
    console.log(`clipstack installed to ${dest}`);
    console.log(`autostart: ${platform.autostartDescription()}`);
    if (process.platform === 'darwin') {
      console.log('macOS will ask for Accessibility access on first use — allow it in');
      console.log('System Settings → Privacy & Security → Accessibility.');
    }
    if (process.platform === 'linux') {
      console.log('Wayland note: global hotkeys are not available to apps on Wayland. Bind a');
      console.log(`desktop shortcut to "${dest} pop" to pop from the stack; the tray menu works regardless.`);
    }
    console.log(`config: ${platform.configPath()}`);
    break;
  }
  case 'uninstall': {
    platform.stop();
    platform.unregisterAutostart();
    const dest = platform.installedBinaryPath();
    if (fs.existsSync(dest)) fs.rmSync(dest);
    console.log('clipstack removed (config and history kept in ' + path.dirname(platform.configPath()) + ')');
    break;
  }
  case 'start': {
    const dest = fs.existsSync(platform.installedBinaryPath()) ? platform.installedBinaryPath() : requireBinary();
    platform.start(dest);
    console.log('clipstack started');
    break;
  }
  case 'stop': {
    platform.stop();
    console.log('clipstack stopped');
    break;
  }
  case 'status': {
    const dest = platform.installedBinaryPath();
    console.log(`installed: ${fs.existsSync(dest) ? dest : 'no'}`);
    console.log(`running:   ${platform.isRunning() ? 'yes' : 'no'}`);
    console.log(`autostart: ${platform.autostartRegistered() ? 'yes' : 'no'}`);
    console.log(`config:    ${platform.configPath()}`);
    break;
  }
  case undefined:
  case '--help':
  case '-h': {
    console.log(`clipstack ${PKG.version}

  npx clipstack install     install, register autostart and launch
  npx clipstack uninstall   stop and remove
  npx clipstack start|stop|status
  npx clipstack pop         pop the newest item onto the clipboard (for desktop shortcuts)
  npx clipstack pop-oldest  pop the oldest item
  npx clipstack clear       empty the stack
  npx clipstack config-path`);
    break;
  }
  default: {
    const bin = fs.existsSync(platform.installedBinaryPath()) ? platform.installedBinaryPath() : requireBinary();
    const r = spawnSync(bin, [cmd, ...rest], { stdio: 'inherit' });
    process.exit(r.status ?? 1);
  }
}
