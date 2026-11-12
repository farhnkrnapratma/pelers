import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const { tokenize } = await import(join(root, 'ts/src/diagrams/flowchart/lexer.ts'));
const { T, TOKEN_NAMES } = await import(join(root, 'ts/src/diagrams/flowchart/tokens.ts'));

const inputs = [];
const goldenDir = join(root, 'tests/golden/flowchart');
for (const name of readdirSync(goldenDir).filter(n => n.endsWith('.mmd')).sort()) {
    inputs.push(readFileSync(join(goldenDir, name), 'utf8'));
}
const extra = [
    'flowchart LR\n  A[Hello] --> B((circle))\n',
    'graph TD\n  A-->|label text|B\n  B-.->C\n  C==>D\n',
    'flowchart TB\n  subgraph one\n    x --- y\n  end\n',
    'flowchart\n  A@{ shape: rect }\n  B@-->C\n  x@-->y@\n',
    'flowchart LR\n  n1@{ label: "a" }\n',
    'flowchart LR\n  accTitle: My title\n  accDescr: Some description\n',
    'flowchart LR\n  accDescr {\n  multi line\n  }\n',
    'flowchart LR\n  classDef big fill:#f9f,stroke:#333\n  class A big\n  style A fill:#f00\n',
    'flowchart LR\n  click A "https://example.com" _blank\n  click B call fn() "tip"\n',
    'flowchart LR\n  A["`**md** text`"] --> B(["stadium"])\n',
    'flowchart LR\n  A[[sub]] --> B[(cyl)] --> C{{hex}} --> D>tag]\n',
    'flowchart LR\n  A[/trap/] --> B[\\inv\\] --> C((("dbl")))\n',
    'flowchart LR\n  Ünïcode 🙂 --> 漢字\n  α -- ünï --> β\n',
    'flowchart TD\n  a --> b & c\n  A ~~~ B\n  X --o Y\n  P --x Q\n  M <--> N\n',
    'flowchart LR\n  A %% comment\n  B\n',
    'direction TB\nflowchart LR\n  A --> B\n',
    '\n\nflowchart LR\n  A\n  \n  B\n',
    'flowchart LR\n  A(("tail")) --> B\n  C -.- D\n',
    'flowchart LR\n  node1@{ shape: "dia" } --> n2\n  x:::cls --> y\n',
    '',
    'graph LR\n  A -- text --> B\n  A -. dotted .-> B\n  A == thick ==> B\n',
    'flowchart LR\n  bad [unterminated\n',
    'flowchart LR\n  A{ diamond\n  B{{ hex }}\n',
    'flowchart LR\n  A["quoted with \\"escape\\""] --> B\n',
    'flowchart LR\n  click A callback\n  click A "ok" "tooltip"\n',
    'flowchart LR\n  A@{ icon: "fa:x", form: "rounded" }\n  D@{ shape: "cyl" }\n',
    'flowchart LR\n  A["a<br/>b"] --> B\n',
];
inputs.push(...extra);

const out = inputs.map(source => {
    const t = tokenize(source);
    return { source, types: Array.from(t.types), texts: t.texts, starts: t.starts };
});
writeFileSync(join(OUT, 'flow-lexer.json'), `${JSON.stringify(out)}\n`);
console.log(`flow lexer: ${out.length} inputs, ${out.reduce((a, b) => a + b.types.length, 0)} tokens`);
