import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

// Unpacks a packed release as an installed package and uses each entry point by its public name.
const tarball = resolve(process.argv[2] ?? '');
if (!existsSync(tarball)) throw new Error('Usage: node scripts/check-pack.mjs <tarball>');

const root = mkdtempSync(join(tmpdir(), 'pelers-pack-'));
const installed = join(root, 'node_modules', 'pelers');
mkdirSync(installed, { recursive: true });
execFileSync('tar', ['-xzf', tarball, '-C', installed, '--strip-components=1']);

const pkg = JSON.parse(readFileSync(join(installed, 'package.json'), 'utf8'));
for (const [name, entry] of Object.entries(pkg.exports)) {
  for (const file of typeof entry === 'string' ? [entry] : Object.values(entry)) {
    const path = join(installed, file.replace('*', 'flowchart'));
    if (!existsSync(path)) throw new Error(`${name} points at ${file}, which is not in the package`);
  }
}

// Bundlers and build scripts find a package with require.resolve, which does not use `import`.
const resolver = createRequire(join(root, 'check.cjs'));
for (const name of Object.keys(pkg.exports)) {
  resolver.resolve(join('pelers', name.replace('*', 'flowchart')));
}

writeFileSync(
  join(root, 'check.mjs'),
  `import { render, mount } from 'pelers';
import { register, supports } from 'pelers/core';
import flowchart from 'pelers/diagrams/flowchart';
import { renderAsync, mountAsync } from 'pelers/lazy';

const source = 'flowchart LR\\n  A --> B';
if (render(source).type !== 'flowchart') throw new Error('pelers does not render');
register(flowchart);
if (!supports(source)) throw new Error('pelers/core does not take a registered type');
if ((await renderAsync('pie\\n  "a": 1')).type !== 'pie') throw new Error('pelers/lazy does not load a type');
if (typeof mount !== 'function' || typeof mountAsync !== 'function') throw new Error('mount is missing');
`
);
try {
  execFileSync(process.execPath, ['check.mjs'], { cwd: root, stdio: 'inherit' });
} finally {
  rmSync(root, { recursive: true, force: true });
}
console.log(`${pkg.name}@${pkg.version}: entry points work as packed`);
