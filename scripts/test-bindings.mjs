import { existsSync, readFileSync } from 'node:fs';
import assert from 'node:assert/strict';

const root = new URL('../', import.meta.url);
const read = name => readFileSync(new URL(name, root), 'utf8');

const pkg = JSON.parse(read('package.json'));
assert.equal(pkg.exports['.'].types, './index.d.ts');

const dts = read('index.d.ts');
const native = read('src/bindings/node.rs');
for (const [name, jsName] of [['render', 'render'], ['detectType', 'detect_type'], ['supports', 'supports']]) {
    assert.match(dts, new RegExp(`function ${name}\\(`), `index.d.ts declares ${name}`);
    assert.match(native, new RegExp(`fn ${jsName}\\(`), `bindings/node.rs defines ${jsName}`);
}

if (!existsSync(new URL('index.native.js', root))) {
    console.log('index.native.js not built, skipping runtime checks');
} else {
    const { PeleError, detectType, render, supports } = await import(new URL('index.js', root));
    const result = render('flowchart LR\n  A[Start] --> B[End]');
    assert.match(result.svg, /^<svg/);
    assert.ok(result.width > 0 && result.height > 0);
    assert.equal(detectType('graph TD\n  A-->B'), 'Flowchart');
    assert.equal(supports('graph TD\n  A-->B'), true);
    assert.equal(supports('pie\n  "a": 1'), false);
    assert.throws(
        () => render('sequenceDiagram\n  A->>B: hi'),
        error => error instanceof PeleError && error.code === 'unsupported-diagram',
    );
}
console.log('bindings ok');
