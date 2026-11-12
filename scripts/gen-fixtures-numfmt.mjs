import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));

function mulberry32(seed) {
    return () => {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

const random = mulberry32(Number(process.env.PELERS_FUZZ_SEED ?? 20261109));
const values = [0, 0.5, 1.5, 2.5, 1.25, 1.005, 9.995, 99.999, 0.000123, 1e-7, 1e21, 1e20, 123456, 99.99, 5];
for (let i = 0; i < 6000 * SCALE; i++) {
    const magnitude = Math.floor(random() * 40) - 20;
    values.push(random() * 10 ** magnitude);
}
for (let i = 0; i < 2000 * SCALE; i++) values.push(Math.round(random() * 2000) / 8);
for (const value of [-0.5, -1.5, -0.000123, -1e21, -123456.789, -0, -5.5e-7, -99.999, -1.005]) values.push(value);
for (let i = 0; i < 2000 * SCALE; i++) values.push(-(random() * 10 ** (Math.floor(random() * 40) - 20)));

const cases = values.map((value) => {
    const digits = Math.floor(random() * 8);
    const precision = 1 + Math.floor(random() * 20);
    return {
        value: String(value),
        fixed: value.toFixed(digits),
        exponential: value.toExponential(digits),
        precision: value.toPrecision(precision),
        digits,
        significant: precision,
    };
});

writeFileSync(join(OUT, 'numfmt.json'), `${JSON.stringify({ cases })}\n`);
console.log(`numfmt: ${cases.length} cases`);
