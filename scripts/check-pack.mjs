import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

// Unpacks a packed release as an installed package and uses each entry point by its public name.
const tarball = resolve(process.argv[2] ?? '');
if (!existsSync(tarball)) throw new Error('Usage: node scripts/check-pack.mjs <tarball>');

const root = mkdtempSync(join(tmpdir(), 'pele-pack-'));
const installed = join(root, 'node_modules', 'pele');
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
  resolver.resolve(join('pele', name.replace('*', 'flowchart')));
}

writeFileSync(
  join(root, 'check.mjs'),
  `import { render, mount } from 'pele';
import { register, supports } from 'pele/core';
import flowchart from 'pele/diagrams/flowchart';
import { renderAsync, mountAsync } from 'pele/lazy';

const source = 'flowchart LR\\n  A --> B';
if (render(source).type !== 'flowchart') throw new Error('pele does not render');
register(flowchart);
if (!supports(source)) throw new Error('pele/core does not take a registered type');
if ((await renderAsync('pie\\n  "a": 1')).type !== 'pie') throw new Error('pele/lazy does not load a type');
if (typeof mount !== 'function' || typeof mountAsync !== 'function') throw new Error('mount is missing');
`
);
try {
  execFileSync(process.execPath, ['check.mjs'], { cwd: root, stdio: 'inherit' });
} finally {
  rmSync(root, { recursive: true, force: true });
}
console.log(`${pkg.name}@${pkg.version}: entry points work as packed`);
