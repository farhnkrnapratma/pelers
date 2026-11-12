import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? join(root, 'tests/fixtures');

function mulberry32(seed) {
    return function () {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const random = mulberry32(Number(process.env.PELERS_FUZZ_SEED ?? 1357));
const pick = list => list[Math.floor(random() * list.length)];
const between = (low, high) => low + Math.floor(random() * (high - low + 1));
const chance = odds => random() < odds;

const NUMBERS = () => {
    const digits = pick([1, 1, 2, 2, 2, 3, 4, 4, 4, 5]);
    if (chance(0.15)) return String(between(0, 99));
    if (chance(0.1)) return String(between(0, 3000));
    return String(between(0, 10 ** digits - 1)).padStart(digits, chance(0.5) ? '0' : ' ').replace(/^ +/, '');
};
const WORDS = ['Jan',  'January', 'feb', 'MAR', 'March', 'Apr', 'May', 'June', 'Jul', 'aug', 'sep', 'Sept', 'Oct', 'nov', 'Dec', 'Mon', 'Tuesday', 'wed', 'Thu', 'fri', 'SAT', 'sun', 'am', 'PM', 'a.m.', 'GMT', 'UTC', 'UT', 'Z', 'z', 'T', 'foo', 'vert', 'x', 'Ü', 'of', 'st', 'the'];
const SEPS = [' ', ' ', ' ', '-', '-', '/', '.', ':', ',', '  ', '+', '(', ')', '_', 'T', '\t', '', '\u0001', '\u0006', '\u001f', '\u000b', '\u0007'];
const SIGNS = ['+', '-'];

function token() {
    const roll = random();
    if (roll < 0.5) return NUMBERS();
    if (roll < 0.75) return pick(WORDS);
    if (roll < 0.85) return pick(SIGNS) + NUMBERS();
    return pick(['10:00', '12:30:45', '1:02:03.004', '00:00', '23:59:59', '0700', '07:00', '+0700', '-0500', 'GMT+7', 'UTC+07:00']);
}

function legacyString() {
    let text = '';
    for (let n = between(1, 6); n > 0; n--) {
        text += token();
        if (n > 1) text += pick(SEPS);
    }
    return text;
}

function isoLike() {
    const y = pick(['2024', '1999', '0050', '202', '12345', '-0044', '+002024', '2024']);
    const m = pick(['03', '3', '12', '13', '00', '1']);
    const d = pick(['15', '5', '31', '32', '0', '29']);
    const time = pick(['', 'T10:00', ' 10:00', 'T10:00:30.5', 'T24:00', 'T10:00Z', 'T10:00+07:00', 'Z', ' GMT', ' +0700']);
    return pick([`${y}-${m}-${d}${time}`, `${y}/${m}/${d}${time}`, `${y}${time}`, `${m}/${d}/${y}${time}`, `${d} ${pick(WORDS)} ${y}${time}`]);
}

const cases = [];
const seen = new Set();
const target = Number(process.env.PELERS_DATE_COUNT ?? 20000) * SCALE;
while (cases.length < target) {
    const source = chance(0.25) ? isoLike() : legacyString();
    if (seen.has(source)) continue;
    seen.add(source);
    cases.push({ source, time: new Date(source).getTime() });
}
writeFileSync(join(OUT, 'date-parse.json'), `${JSON.stringify({ cases: cases.map(c => ({ source: c.source, time: Number.isNaN(c.time) ? null : c.time })) })}\n`);
console.log(`date-parse: ${cases.length} cases, ${cases.filter(c => Number.isNaN(c.time)).length} invalid`);
