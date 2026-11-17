import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const pelers = require(join(root, '..', 'index.native.js')) as { render: (source: string) => string };

const generated = join(root, 'src/lib/generated');
mkdirSync(generated, { recursive: true });

const examples = JSON.parse(readFileSync(join(root, 'src/lib/content/examples.json'), 'utf8')) as Record<string, string>;
const exampleSvgs: Record<string, string> = {};
for (const [name, source] of Object.entries(examples)) {
    if (name.endsWith('Example')) {
        exampleSvgs[name] = pelers.render(source);
    }
}
writeFileSync(join(generated, 'examples.json'), JSON.stringify(exampleSvgs));

const markdownDirs = [join(root, 'src/lib/content'), join(root, 'src/lib/content/docs')];
const diagrams: Record<string, string> = {};
const fence = /```mermaid\n([\s\S]*?)```/g;
for (const dir of markdownDirs) {
    for (const file of readdirSync(dir).filter((name) => name.endsWith('.md'))) {
        const text = readFileSync(join(dir, file), 'utf8');
        for (const match of text.matchAll(fence)) {
            const source = match[1];
            if (!(source in diagrams)) {
                diagrams[source] = pelers.render(source);
            }
        }
    }
}
writeFileSync(join(generated, 'diagrams.json'), JSON.stringify(diagrams));
console.log(`rendered ${Object.keys(exampleSvgs).length} examples and ${Object.keys(diagrams).length} diagrams`);
