import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const { render } = await import(join(root, 'ts/dist/pele.js'));

function mulberry32(seed) {
    return () => {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

const random = mulberry32(20261105);
const pick = list => list[Math.floor(random() * list.length)];
const between = (low, high) => low + Math.floor(random() * (high - low + 1));
const chance = odds => random() < odds;

const WORDS = ['Dogs', 'Cats', 'Rats', 'Calcium', 'Iron', 'North', 'Q1 2024', 'a', 'B', 'Übergrößenträger', '日本語', 'emoji 😀', 'a&amp;b', '<b>bold</b>', 'x < y', 'long label that keeps going and going well past any reasonable width', '', ' ', 'with "quote"', "it's", 'back\\slash', 'tab\there', '#fff', '100%'];
const COLORS = [
    '#ff0000', 'red', 'navy', 'gold', 'Grey', 'rgb(10 20 30)', 'hsl(200, 50%, 60%)', 'rgba(0,0,0,.4)', '#0008', '#abc', '#abcd',
    'var(--pele-x)', 'var(--other)', 'url(x)', 'calc(1)', 'rebeccapurple', 'transparent', 'currentcolor', 5, true, '',
    '#707070', '#767676', '#777777', '#787878', '#7a7a7a', '#808080', '#6e6e6e', '#757575', '#7f7f7f',
    'hsl(0.5turn 50% 50%)', 'hsl(3.14rad 40% 40%)', 'hsl(200grad 60% 70%)', 'hsl(90deg, 100%, 25%)', 'hsl(120 100% 50% / 50%)',
    'hwb(0 0% 0%)', 'color-mix(in srgb, red, blue)', 'light-dark(red, blue)', 'rgb(10% 20% 30%)', 'rgb(none 5 7)',
    'rgb(300 0 0)', 'rgb(0 0 0 / 0.3)', 'rgba(255, 255, 255, 0.9)', 'rgb(1, 2, 3', 'rgb( 5 6 7 )', 'RGB(9 9 9)', 'hsla(10, 20%, 30%, 40%)',
    'lab(50 40 30)', 'oklch(0.7 0.1 200)', 'rgb(1e1 2 3)', 'rgb(.5 .5 .5)', 'rgb(+5 5 5)',
];

function label() {
    const text = pick(WORDS);
    if (text.includes('"') || text.includes('\\')) {
        const escaped = text.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
        return `"${escaped}"`;
    }
    return chance(0.8) ? `"${text}"` : `'${text.replace(/'/g, "\\'")}'`;
}

function value() {
    switch (between(0, 9)) {
        case 0: return '0';
        case 1: return String(between(1, 5));
        case 2: return String(between(1, 100000));
        case 3: return (random() * 100).toFixed(between(1, 4));
        case 4: return `0.${between(1, 999)}`;
        case 5: return '1000000';
        case 6: return String(between(1, 20));
        case 7: return chance(0.1) ? '-1' : chance(0.3) ? '-0' : '7';
        case 8: return `${between(1, 9)}.${between(0, 99)}`;
        default: return String(between(1, 400));
    }
}

function config() {
    const lines = [];
    const pie = [];
    if (chance(0.5)) pie.push(`textPosition: ${pick([0, 0.25, 0.5, 0.9, 1, 1.5, -1])}`);
    if (chance(0.4)) pie.push(`donutHole: ${pick([0, 0.2, 0.5, 0.9, 0.95, -1])}`);
    if (chance(0.5)) pie.push(`legendPosition: ${pick(['left', 'right', 'top', 'bottom', 'center', 'diagonal', 'null'])}`);
    if (chance(0.15)) pie.push('useMaxWidth: false');
    const theme = [];
    if (chance(0.4)) {
        for (let i = 1; i <= between(1, 9); i++) {
            const color = pick(COLORS);
            theme.push(`    pie${i}: ${typeof color === 'string' ? JSON.stringify(color) : color}`);
        }
    }
    if (pie.length === 0 && theme.length === 0) return '';
    lines.push('---', 'config:');
    if (pie.length) lines.push('  pie:', ...pie.map(entry => `    ${entry}`));
    if (theme.length) lines.push('  themeVariables:', ...theme);
    lines.push('---');
    return `${lines.join('\n')}\n`;
}

function source() {
    let text = config();
    if (chance(0.1)) text += '%% a comment\n';
    text += chance(0.5) ? 'pie' : 'pie showData';
    if (chance(0.3)) text += `${pick([' ', '  '])}title ${pick(['Pets', 'Key elements in Product X', 'A very long title that goes on and on and on and on and on', 'T&amp;C', 'x < y'])}`;
    text += '\n';
    if (chance(0.15)) text += `    title ${pick(['Later', 'Two'])}\n`;
    if (chance(0.15)) text += `    accTitle: ${pick(['Chart', '  spaced  '])}\n`;
    if (chance(0.15)) text += pick(['    accDescr: A chart\n', '    accDescr {\n      one\n        two\n    }\n']);
    const count = chance(0.05) ? 0 : between(1, chance(0.2) ? 40 : 9);
    const skew = chance(0.25);
    for (let i = 0; i < count; i++) {
        const gap = pick([' : ', ': ', ' :', ':']);
        const amount = skew && i > 0 ? pick(['0.01', '0.2', '1', '3']) : skew ? '5000' : value();
        text += `${pick(['    ', '', '\t'])}${label()}${gap}${amount}\n`;
        if (chance(0.05)) text += '\n';
        if (chance(0.05)) text += '    %% note\n';
    }
    if (chance(0.03)) text += pick(['"x" : abc\n', '"x" 5\n', 'x : 1\n', '"x" : 1.\n', '"x" : 1.5.5\n', '"x" : -\n', '"open : 1\n']);
    return text;
}

function options() {
    const result = {};
    if (chance(0.25)) result.maxWidth = pick([100, 200, 300, 400, 500, 640, 1000]);
    if (chance(0.15)) result.fontSize = pick([10, 12, 14, 20, 24]);
    if (chance(0.15)) result.padding = pick([0, 4, 16, 30]);
    return result;
}

function capture(text, opts) {
    try {
        const out = render(text, opts);
        return { svg: out.svg, width: String(out.width), height: String(out.height) };
    } catch (error) {
        return { error: { code: error.code, message: error.message, line: error.line ?? null, column: error.column ?? null } };
    }
}

const seen = new Set();
const cases = [];
while (cases.length < 700) {
    const text = source();
    const opts = options();
    const key = JSON.stringify([text, opts]);
    if (seen.has(key)) continue;
    seen.add(key);
    cases.push({ source: text, options: opts, ...capture(text, opts) });
}

writeFileSync(join(root, 'tests/fixtures/render-pie.json'), `${JSON.stringify({ cases })}\n`);
const failed = cases.filter(entry => entry.error).length;
console.log(`render-pie: ${cases.length} cases, ${failed} errors`);
