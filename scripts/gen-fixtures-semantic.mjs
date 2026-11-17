import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const OUT = process.env.PELERS_FUZZ_OUT ?? join(root, 'tests/fixtures');
const { parse, register } = await import(join(root, 'ts/src/core.ts'));
const { all } = await import(join(root, 'ts/src/diagrams/registry.ts'));
register(...all);

const sourceFiles = [
    join(root, 'tests/fixtures/render-agentflow.json'),
    join(root, 'tests/fixtures/render-mut-agentflow.json'),
];
const sources = [];
for (const file of sourceFiles) {
    for (const c of JSON.parse(readFileSync(file, 'utf8')).cases) sources.push(c.source);
}
for (const name of ['000', '001', '002', '003', '004', '005', '006', '007', '008', '009', '010', '011']) {
    try {
        sources.push(readFileSync(join(root, `tests/golden/agentflow/${name}.mmd`), 'utf8'));
    } catch {
        break;
    }
}

const cases = sources.map(source => {
    try {
        return { source, model: parse(source).semanticModel() };
    } catch (error) {
        return { source, error: { code: error.code, message: error.message, line: error.line ?? null, column: error.column ?? null } };
    }
});
writeFileSync(join(OUT, 'semantic-agentflow.json'), `${JSON.stringify({ cases })}\n`);
console.log(`semantic-agentflow: ${cases.length} cases, ${cases.filter(c => c.error).length} errors`);
