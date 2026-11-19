#!/usr/bin/env node
'use strict';

// Builds one @clipstack/<os>-<arch> package directory from a compiled binary.
//   node make-platform-package.js <os> <arch> <path/to/binary> <version> <out-dir>
// Used by CI; also handy locally:
//   node npm/scripts/make-platform-package.js darwin arm64 target/release/clipstack 0.1.0 dist

const fs = require('node:fs');
const path = require('node:path');

const [osName, arch, binary, version, outDir] = process.argv.slice(2);
if (!osName || !arch || !binary || !version || !outDir) {
  console.error('usage: make-platform-package.js <os> <arch> <binary> <version> <out-dir>');
  process.exit(2);
}

const name = `@clipstack/${osName}-${arch}`;
const dir = path.join(outDir, `${osName}-${arch}`);
const exe = osName === 'win32' ? 'clipstack.exe' : 'clipstack';
fs.mkdirSync(path.join(dir, 'bin'), { recursive: true });
fs.copyFileSync(binary, path.join(dir, 'bin', exe));
if (osName !== 'win32') fs.chmodSync(path.join(dir, 'bin', exe), 0o755);

fs.writeFileSync(
  path.join(dir, 'package.json'),
  JSON.stringify(
    {
      name,
      version,
      description: `ClipStack binary for ${osName}-${arch}`,
      license: 'MIT',
      os: [osName],
      cpu: [arch],
      files: ['bin'],
    },
    null,
    2,
  ) + '\n',
);
console.log(`wrote ${dir}`);
