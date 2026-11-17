import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const ink = await import(join(root, 'ts/src/svg/ink.ts'));

const COLORS = ['#fff', '#000', '#1f1f1f', '#abc', '#abcd', '#aabbcc', '#aabbccdd', '#ggg', 'red', 'Red', 'blue', 'navy', 'grey', 'gray', 'white', 'transparent', 'currentColor', 'none', 'rgb(255 255 255)', 'rgb(0,0,0)', 'rgba(0,0,0,0.3)', 'rgb(10% 20% 30% / 0.5)', 'hsl(120 50% 50%)', 'hsla(0,100%,50%,0.2)', 'hsl(0.5turn 100% 50%)', 'rgb(none 0 0)', 'rgb(1,2)', 'rgb(1,2,3,4,5)', 'hsl(180deg, 20%, 80%)', 'var(--x)', 'calc(1px)', '#FFF', 'rgb(300,-5,0)', 'hsl(1rad 10% 90%)', 'hsl(0grad 100% 50%)', '#12', ''];
const OPACITIES = ['', 'opacity:0.5;', 'fill-opacity:0.2;', 'fill-opacity:50%;', 'opacity:x;', 'opacity:calc(1);', 'opacity:0.5;fill-opacity:0.5;', 'fill-opacity:1;'];
const TEXTS = ['', 'color:red', 'fill:red', 'stroke:blue', 'color:#fff'];

const shapes = [];
for (const color of COLORS) {
  for (const op of OPACITIES) {
    shapes.push(`fill:${color};${op}`);
    shapes.push(`;fill:${color};stroke:red;${op}`);
  }
}
shapes.push('', 'stroke:red', '"fill:#fff"', 'fill:#fff', 'fill:none', 'fill:transparent');

const cases = [];
for (const shape of shapes) {
  for (const text of TEXTS.slice(0, 3)) {
    cases.push({ shape, text, ink: ink.ink(shape, text) ?? null });
  }
  cases.push({ shape, paints: ink.paints(shape) });
}
const plains = COLORS.map(raw => ({ raw, out: ink.plainColor(raw) }));
writeFileSync(join(OUT, 'ink.json'), `${JSON.stringify({ cases, plains })}\n`);
console.log(`ink: ${cases.length} cases, ${plains.length} plain colors`);
