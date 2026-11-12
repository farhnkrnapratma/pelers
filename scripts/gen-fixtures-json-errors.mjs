import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const OUT = process.env.PELERS_FUZZ_OUT ?? join(root, 'tests/fixtures');
const COUNT = Number(process.env.PELERS_FUZZ_COUNT ?? 3000);

function mulberry32(seed) {
    let a = seed >>> 0;
    return () => {
        a = (a + 0x6d2b79f5) >>> 0;
        let t = a;
        t = Math.imul(t ^ (t >>> 15), t | 1);
        t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const random = mulberry32(Number(process.env.PELERS_FUZZ_SEED ?? 20261110));
const pick = list => list[Math.floor(random() * list.length)];
const between = (lo, hi) => lo + Math.floor(random() * (hi - lo + 1));

const TOKENS = ['{', '}', '[', ']', ',', ':', '"a"', '"b c"', '1', '-', '0', '01', '1.', '1.5', '1e', '1e+', '-x', 'true', 'false', 'null', 'tru', 'nul', 'nlink', 'ull', 'x', '#', ' ', '\n', '"', '\\', '"\\x"', '"\\u12"', '"\\u0041"', '\t', '\u0001', '"a\nb"', '2', ';', '@', '.', '+', '{"k":1}', '[1]', '"\\"'];
const VALID = ['{"a":1}', '{"a":[1,2,{"b":null}],"c":"d"}', '[1,"two",true,null]', '{"x":{"y":{"z":-1.5e3}}}', '{"":""}', '{"k": "\\u0041\\n"}', '{}', '[]'];

function soup() {
    let text = '';
    for (let k = between(1, 7); k > 0; k--) text += pick(TOKENS);
    return text;
}

function mutate(text) {
    if (text.length === 0) return soup();
    const at = between(0, text.length - 1);
    switch (between(0, 3)) {
        case 0: return text.slice(0, at) + text.slice(at + 1);
        case 1: return text.slice(0, at) + pick(TOKENS) + text.slice(at);
        case 2: return text.slice(0, at);
        default: return text.slice(0, at) + pick(TOKENS) + text.slice(at + 1);
    }
}

const seen = new Set();
const cases = [];
const add = text => {
    if (seen.has(text)) return;
    seen.add(text);
    try {
        JSON.parse(text);
        cases.push({ text, message: null });
    } catch (error) {
        cases.push({ text, message: error.message });
    }
};

for (const text of ['', ' ', '{', '[', '{"a"', '{"a":', '{"a":1', '{a:1}', '{"a" 1}', '[1,]', '[1 2]', '01', '1.', '1e', '-', 'nlink', 'x', '"abc', '"\\x"', '"\\u12"', '1 2', '{}}', ']', '{"a":1,}', '"a\nb"']) add(text);
for (let i = 0; i < COUNT; i++) {
    const roll = random();
    if (roll < 0.4) add(soup());
    else add(mutate(mutate(pick(VALID))));
}
writeFileSync(join(OUT, 'json-errors.json'), `${JSON.stringify({ cases })}\n`);
const messages = new Map();
for (const c of cases) if (c.message) messages.set(c.message, (messages.get(c.message) ?? 0) + 1);
console.log(`json-errors: ${cases.length} cases, ${cases.filter(c => c.message === null).length} valid`);
for (const [m, n] of [...messages].sort((a, b) => b[1] - a[1])) console.log(n, m);
