import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (`${root}tests/fixtures`);
const { layoutLabel } = await import(`${root}ts/src/text/label.ts`);
const { metricsMeasurer } = await import(`${root}ts/src/text/measurer.ts`);
const { esc, labelSvg, num } = await import(`${root}ts/src/svg/builder.ts`);

function mulberry32(seed) {
    return () => {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const random = mulberry32(Number(process.env.PELERS_FUZZ_SEED ?? 20261105));
const pick = list => list[Math.floor(random() * list.length)];

const FRAGMENTS = [
    'Start', 'a', 'Hello world', 'The quick brown fox', 'iiiii', 'WWWWW', '0123456789', '  ', ' ', '\t',
    '<b>', '</b>', '<i>', '</i>', '<code>', '</code>', '<sub>', '</sub>', '<sup>', '</sup>', '<u>', '</u>', '<br>', '<br/>',
    '<BR />', '\\n', '\n', '\r\n', '**bold**', '*it*', '_em_', '__strong__', 'snake_case_word', '* not', 'a*b',
    '&amp;', '&lt;', '&#65;', '&#x42;', '&nbsp;', '&unknown;', '&#0;', '&', 'ﬂ°quot¶ß', 'ﬂ°°35¶ß', 'ﬂ°', '¶ß',
    'fa:fa-user', 'fab:fa-github', 'x fas:fa-cog y', 'fa:fa-', 'List<T>', '<span>x</span>', '<div class="a">', '</div>',
    '<unknown>', '<a href="x">', '< b>', '<b', '漢字かな', '日本語のテキスト、です。', '（括弧）', '한국어 텍스트', 'é', 'ñ', 'ü',
    '­', 'hy­phen­ated', '​', 'zero​width', '😀', 'á', '　', ' ', '﻿',
    '\u1100', '\u115f', '\u2e80', '\ua4cf', '\uac00', '\ud7a3', '\uf900', '\ufe30', '\uff00', '\uff60', '\uffe0', '\uffe6', '\u0300', '\u036f', '\u200f', '\ufe0f', '\u007f', '\u0080', '~',
    '-', '—', ',', '.', '!', '?', '(', ')', '[', ']', '{', '}', 'word-with-dash', 'averyveryveryverylongwordwithoutanyspaces',
];

const cases = [];
const sizes = [10, 12, 14, 16, 20, 24];
const widths = [30, 60, 120, 200, 4000];
const fixed = [
    'Start', 'Pelers v1.0.0-alpha2', 'A label that is long enough to wrap onto several lines when narrow',
    'line one<br/>line two', '**bold** and *italic*', 'a <b>bold</b> c', 'H<sub>2</sub>O and x<sup>2</sup>',
    '    indented\n    markdown', '', ' ', 'x', 'fa:fa-user Alice', 'Alice fa:fa-user', 'text &amp; more',
];
for (const raw of fixed) for (const markdown of [false, true]) cases.push({ raw, markdown, size: 16, max: 4000, base: 0 });
const narrow = [
    '漢字」漢字」', '「漢字」かな、です。', 'Supercalifragilisticexpialidocious\u200b', 'ab\u00ad<b>cd</b>', 'ab\u00ad<b>cd\u00ad</b>ef',
    'x<b/>y z', 'a<brx b c', 'one two three four five', '<b>aaa</b> bbb ccc ddd eee', 'aaa bbb ', 'abc\u200bdef\u200bghi',
    'long<br>line one two three<br>x', 'a <i>b c d e f</i> g h i', 'ａｂ（漢字）ｃｄ', 'word1 word2 word3\u00adword4 word5',
];
for (const raw of narrow) for (const markdown of [false, true]) for (const max of [20, 45, 80]) cases.push({ raw, markdown, size: 16, max, base: 0 });
for (const raw of ['a<br><br>b', '<br>a', 'a<br>', '漢字),漢,字.', 'abc\u200b)def ghi', 'ab\u00ad,cd efg']) for (const max of [-1, 0, 20, 45]) cases.push({ raw, markdown: false, size: 16, max, base: 0 });
for (let i = 0; i < 1500 * SCALE; i++) {
    let raw = '';
    for (let n = 1 + Math.floor(random() * 7); n > 0; n--) raw += pick(FRAGMENTS) + (random() < 0.5 ? ' ' : '');
    cases.push({
        raw,
        markdown: random() < 0.5,
        size: pick(sizes),
        max: pick(widths),
        base: pick([0, 0, 0, 1, 2, 4]),
    });
}

const text = String;
const out = cases.map(({ raw, markdown, size, max, base }) => {
    const label = layoutLabel(raw, markdown, metricsMeasurer, size, max, base);
    return {
        raw, markdown, size, max, base,
        lines: label.lines.map(line => line.map(span => [span.text, span.style, text(span.width)])),
        widths: label.widths.map(text),
        width: text(label.width),
        height: text(label.height),
        lineHeight: text(label.lineHeight),
        svg: labelSvg(label, 100, 50, ' class="t"'),
    };
});

const values = [0, -0, 0.004, 0.005, 0.125, 1.005, 1.006, 25.6, 75, -2.5, -0.125, 1e9, 1e10, 1e11, 1e12, 123456.789, NaN, Infinity, -Infinity];
for (let i = 0; i < 400 * SCALE; i++) values.push((random() - 0.5) * Math.pow(10, Math.floor(random() * 8)));
const numbers = values.map(value => [text(value), num(value)]);
const escapes = ['plain', 'a<b>&"c', '\u0001\u007fx￾', '&amp;', '😀 <>'].map(s => [s, esc(s)]);

writeFileSync(`${OUT}/text.json`, `${JSON.stringify({ labels: out, numbers, escapes })}\n`);
console.log(`fixtures: ${out.length} labels, ${numbers.length} numbers, ${escapes.length} escapes`);
