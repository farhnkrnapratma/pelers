import { build } from 'esbuild';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const entry = resolve(process.argv[2]);
const outdir = resolve('node_modules/.cache/pele-run');
mkdirSync(outdir, { recursive: true });
const outfile = resolve(outdir, `run-${process.pid}.mjs`);
await build({
  entryPoints: [entry],
  bundle: true,
  platform: 'node',
  format: 'esm',
  outfile,
  packages: 'external',
  logLevel: 'warning',
});
process.argv.splice(1, 2, outfile);
await import(pathToFileURL(outfile).href);
