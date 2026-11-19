'use strict';

// Per-OS install location, autostart registration and process control.

const { spawnSync, spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const HOME = os.homedir();
const IS_WIN = process.platform === 'win32';
const IS_MAC = process.platform === 'darwin';

const LAUNCH_AGENT = path.join(HOME, 'Library', 'LaunchAgents', 'com.clipstack.agent.plist');
const LAUNCH_LABEL = 'com.clipstack.agent';
const WIN_RUN_KEY = 'HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run';
const WIN_RUN_NAME = 'ClipStack';
const XDG_AUTOSTART = path.join(process.env.XDG_CONFIG_HOME || path.join(HOME, '.config'), 'autostart', 'clipstack.desktop');

function installedBinaryPath() {
  if (IS_WIN) return path.join(process.env.LOCALAPPDATA || path.join(HOME, 'AppData', 'Local'), 'ClipStack', 'clipstack.exe');
  return path.join(HOME, '.local', 'bin', 'clipstack');
}

function configPath() {
  if (IS_WIN) return path.join(process.env.APPDATA || path.join(HOME, 'AppData', 'Roaming'), 'clipstack', 'config.json');
  return path.join(HOME, '.config', 'clipstack', 'config.json');
}

function run(cmd, args) {
  return spawnSync(cmd, args, { encoding: 'utf8' });
}

function isRunning() {
  if (IS_WIN) {
    const r = run('tasklist', ['/FI', 'IMAGENAME eq clipstack.exe', '/NH']);
    return /clipstack\.exe/i.test(r.stdout || '');
  }
  const r = run('pgrep', ['-x', 'clipstack']);
  return r.status === 0;
}

function stop() {
  if (IS_WIN) run('taskkill', ['/IM', 'clipstack.exe', '/F']);
  else if (IS_MAC) {
    run('launchctl', ['bootout', `gui/${process.getuid()}/${LAUNCH_LABEL}`]);
    run('pkill', ['-x', 'clipstack']);
  } else run('pkill', ['-x', 'clipstack']);
}

function start(bin) {
  if (IS_MAC && fs.existsSync(LAUNCH_AGENT)) {
    // Through launchd so it is the same instance that autostart manages.
    const r = run('launchctl', ['bootstrap', `gui/${process.getuid()}`, LAUNCH_AGENT]);
    if (r.status === 0) return;
  }
  const child = spawn(bin, [], { detached: true, stdio: 'ignore', windowsHide: true });
  child.unref();
}

function registerAutostart(bin) {
  if (IS_MAC) {
    fs.mkdirSync(path.dirname(LAUNCH_AGENT), { recursive: true });
    fs.writeFileSync(LAUNCH_AGENT, `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key><array><string>${bin}</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><false/>
  <key>ProcessType</key><string>Interactive</string>
</dict></plist>
`);
  } else if (IS_WIN) {
    run('reg', ['add', WIN_RUN_KEY, '/v', WIN_RUN_NAME, '/t', 'REG_SZ', '/d', `"${bin}"`, '/f']);
  } else {
    fs.mkdirSync(path.dirname(XDG_AUTOSTART), { recursive: true });
    fs.writeFileSync(XDG_AUTOSTART, `[Desktop Entry]
Type=Application
Name=ClipStack
Comment=Clipboard stack in the tray
Exec=${bin}
X-GNOME-Autostart-enabled=true
NoDisplay=true
`);
  }
}

function unregisterAutostart() {
  if (IS_MAC) {
    if (fs.existsSync(LAUNCH_AGENT)) fs.rmSync(LAUNCH_AGENT);
  } else if (IS_WIN) {
    run('reg', ['delete', WIN_RUN_KEY, '/v', WIN_RUN_NAME, '/f']);
  } else if (fs.existsSync(XDG_AUTOSTART)) {
    fs.rmSync(XDG_AUTOSTART);
  }
}

function autostartRegistered() {
  if (IS_MAC) return fs.existsSync(LAUNCH_AGENT);
  if (IS_WIN) {
    const r = run('reg', ['query', WIN_RUN_KEY, '/v', WIN_RUN_NAME]);
    return r.status === 0;
  }
  return fs.existsSync(XDG_AUTOSTART);
}

function autostartDescription() {
  if (IS_MAC) return `LaunchAgent ${LAUNCH_AGENT}`;
  if (IS_WIN) return `registry ${WIN_RUN_KEY}\\${WIN_RUN_NAME}`;
  return XDG_AUTOSTART;
}

module.exports = {
  installedBinaryPath,
  configPath,
  isRunning,
  stop,
  start,
  registerAutostart,
  unregisterAutostart,
  autostartRegistered,
  autostartDescription,
};
