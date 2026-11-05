// Renders every tests-corpus source with the TypeScript oracle and stores the result as the reference
// the Rust port must reproduce byte for byte. Run `bun run build` in ts/ first.
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const corpus = join(root, 'ts/tests/corpus');
const out = join(root, 'tests/golden');
const { render } = await import(join(root, 'ts/dist/pele.js'));

rmSync(out, { recursive: true, force: true });
let svgs = 0;
let errors = 0;

for (const file of readdirSync(corpus).filter(name => name.endsWith('-docs.json')).sort()) {
    const type = file.replace(/-docs\.json$/, '');
    const sources = JSON.parse(readFileSync(join(corpus, file), 'utf8'));
    const dir = join(out, type);
    mkdirSync(dir, { recursive: true });
    sources.forEach((source, index) => {
        const name = String(index).padStart(3, '0');
        writeFileSync(join(dir, `${name}.mmd`), source);
        try {
            writeFileSync(join(dir, `${name}.svg`), render(source).svg);
            svgs += 1;
        } catch (error) {
            writeFileSync(join(dir, `${name}.error.json`), `${JSON.stringify({ code: error.code, message: error.message }, null, 2)}\n`);
            errors += 1;
        }
    });
}
console.log(`golden: ${svgs} svg, ${errors} error cases`);
