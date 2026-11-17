import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { render } from 'pele';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const corpus = JSON.parse(readFileSync(join(root, 'bench', 'corpus.json'), 'utf8'));
const RENAME_SENSITIVE = new Set(['info']);
const toPelers = (svg) => svg.replace(/pele(?!rs)/g, 'pelers').replace(/Pele(?!rs)/g, 'Pelers');

const pelersOptions = (type) => (type === 'cynefin' ? { idPrefix: 'pelers' } : undefined);

const cases = [];
for (const item of corpus) {
  if (RENAME_SENSITIVE.has(item.type)) continue;
  cases.push({ type: item.type, source: item.source, svg: toPelers(render(item.source, pelersOptions(item.type)).svg) });
}
writeFileSync(join(root, 'tests', 'fixtures', 'pelers-corpus.json'), `${JSON.stringify({ reference: 'pele@1.13.0', cases })}\n`);
console.log(`wrote ${cases.length} cases`);
