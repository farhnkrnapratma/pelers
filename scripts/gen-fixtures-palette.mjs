import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const { palette } = await import(join(root, 'ts/src/svg/palette.ts'));

function mulberry32(seed) {
    return () => {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const random = mulberry32(4242);
const between = (low, high) => low + Math.floor(random() * (high - low + 1));
const pick = list => list[Math.floor(random() * list.length)];
const hex = n => n.toString(16).padStart(2, '0');

const colors = [];
for (let v = 0; v < 256; v++) colors.push(`#${hex(v).repeat(3)}`);
for (let v = 100; v < 140; v++) colors.push(`rgb(${v} ${v} ${v})`, `rgb(${v}, ${v}, ${v})`, `rgb(${v}, ${v}, ${v}, 0.${between(1, 99)})`);
for (let i = 0; i < 400; i++) colors.push(`#${hex(between(0, 255))}${hex(between(0, 255))}${hex(between(0, 255))}${chance() ? hex(between(0, 255)) : ''}`);
for (let i = 0; i < 100; i++) colors.push(`#${between(0, 4095).toString(16).padStart(3, '0')}${chance() ? between(0, 15).toString(16) : ''}`);
for (let i = 0; i < 300; i++) {
    const unit = pick(['', 'deg', 'turn', 'rad', 'grad', 'DEG', 'Turn']);
    const h = unit === 'turn' ? random().toFixed(2) : unit === 'rad' ? (random() * 6.3).toFixed(2) : between(-400, 800);
    const s = between(0, 100);
    const l = between(0, 100);
    const alpha = pick(['', ' / 50%', ' / 0.3', ', 0.9', ' / none']);
    colors.push(pick([`hsl(${h}${unit} ${s}% ${l}%${alpha.startsWith(',') ? '' : alpha})`, `hsl(${h}${unit}, ${s}%, ${l}%${alpha.startsWith(',') ? alpha : ''})`, `hsla(${h}${unit}, ${s}%, ${l}%, 0.${between(1, 9)})`]));
}
for (let i = 0; i < 300; i++) {
    const r = between(0, 255);
    const g = between(0, 255);
    const b = between(0, 255);
    colors.push(pick([`rgb(${r} ${g} ${b})`, `rgb(${r}, ${g}, ${b})`, `rgba(${r}, ${g}, ${b}, 0.${between(0, 9)})`, `rgb(${r}% ${g % 100}% ${b % 100}%)`, `rgb(${r} ${g} ${b} / ${between(0, 100)}%)`, `rgb(${r}, ${g}%, ${b}%)`, `rgb(none ${g} ${b})`]));
}
for (let h = 0; h < 400; h += 2) for (const l of [40, 55, 70]) colors.push(`hsl(${h}grad 90% ${l}%)`);
for (let h = 0; h < 6.3; h += 0.05) for (const l of [40, 55, 70]) colors.push(`hsl(${h.toFixed(2)}rad 90% ${l}%)`);
for (let n = 190; n <= 202; n++) colors.push(`rgb(1${' '.repeat(n - 9)}2 3)`, `#fff${' '.repeat(5)}`.padEnd(n, ' ') + '');
colors.push(
    'var(--pele)', 'var(--pelex)', 'var(--pele-)', 'var(--pele-a-b_c)', 'var(  --pele-a)',
    'red', 'Navy', 'GREY', 'lightgrey', 'darkgrey', 'slategrey', 'rebeccapurple', 'transparent', 'currentColor', 'currentcolor', 'notacolor', '',
    ' red ', 'red;', 'red"', "red'", 'url(x)', 'var(--pele-x)', 'var( --pele-x)', 'VAR(--pele-x)', 'var(--other)', 'var(--pele-a) var(--other)',
    'color-mix(in srgb, red, blue)', 'color-mix(in srgb, var(--pele-a), blue)', 'light-dark(red, blue)', 'hwb(0 0% 0%)', 'lab(50 40 30)',
    'calc(1)', 'rgb(1 2 3', 'rgb(1 2 3))', ')(', '(', 'rgb (1 2 3)', 'rgb(1 2 3) ', 'x'.repeat(201), 'a'.repeat(200), '#fff'.padEnd(200, ' '),
    'rgb(1e1 2 3)', 'rgb(.5 .5 .5)', 'rgb(+5 5 5)', 'rgb(5. 5 5)', 'rgb(1/2/3)', 'rgb(1 2 3 4)', 'hsl(1 2 3)', 'hsl(1, 2%, 3%)', 'hsl(%, 1%, 1%)',
    '#12', '#12345', '#1234567', 'rgba(1,2,3)', 'rgb(1,2)', 'hsl(0turn 0% 0%)', 'hsl(1e1deg 50% 50%)', 'rgb(1 2 3 / 4 / 5)', ' red ', 'red ', '\u0000red'
);
function chance() {
    return random() < 0.5;
}

function dump(type, config) {
    const made = palette(type, config);
    return { type, config, style: made.style, inks: Array.from({ length: 8 }, (_, i) => made.inks[i] ?? null) };
}

const cases = [];
for (const color of colors) cases.push(dump('pie', { themeVariables: { pie1: color } }));
for (const type of ['pie', 'gitGraph', 'timeline', 'radar', 'treemap', 'mindmap', 'venn']) {
    const prefix = { pie: 'pie', gitGraph: 'git', venn: 'venn', mindmap: 'cScale' }[type] ?? 'cScale';
    for (let round = 0; round < 20; round++) {
        const vars = {};
        for (let i = 0; i <= 9; i++) if (random() < 0.5) vars[prefix + i] = pick(['red', '#000', '#fff', 'navy', '', 5, null, 'rgb(1 2 3)', 'bad;', 'gold']);
        cases.push(dump(type, { themeVariables: vars }));
    }
}
for (const list of ['red, blue', '#111, rgb(1, 2, 3), hsl(1, 2%, 3%)', '', ',', 'red,,blue', 'rgb(1, 2, 3', ' navy ', 'a(b, c), d'])
    cases.push(dump('xychart', { themeVariables: { xyChart: { plotColorPalette: list } } }));
for (const config of [{}, { themeVariables: 5 }, { themeVariables: [] }, { themeVariables: { xyChart: 3 } }, { themeVariables: { xyChart: { plotColorPalette: 7 } } }, { themeVariables: { pie1: 'red' } }])
    for (const type of ['pie', 'xychart', 'flowchart', 'venn']) cases.push(dump(type, config));

writeFileSync(join(root, 'tests/fixtures/palette.json'), `${JSON.stringify({ cases })}\n`);
console.log(`palette: ${cases.length} cases`);
