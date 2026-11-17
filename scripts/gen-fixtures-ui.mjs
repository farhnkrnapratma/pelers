import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const OUT = process.env.PELERS_FUZZ_OUT ?? join(root, 'tests/fixtures');
const zoom = await import(join(root, 'ts/src/zoom.ts'));
const mount = await import(join(root, 'ts/src/mount.ts'));
const windowModule = await import(join(root, 'ts/src/window.ts'));

function mulberry32(seed) {
    return function () {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const random = mulberry32(Number(process.env.PELERS_FUZZ_SEED ?? 20261110));
const between = (low, high) => low + random() * (high - low);
const pick = list => list[Math.floor(random() * list.length)];

const frames = [];
for (let i = 0; i < 2000 * Number(process.env.PELERS_FUZZ_SCALE ?? 1); i++) {
    const natural = { width: pick([0, 1, 10, 333.5, 1200]) + between(0, 400), height: between(0, 900) };
    const box = { width: between(-20, 900), height: between(-20, 700) };
    const scale = pick([0.25, 0.5, 1, 1.25, 1.5625, 3, between(0.1, 6)]);
    const x = between(-2000, 2000);
    const y = between(-2000, 2000);
    frames.push({ natural, box, scale, x, y, frame: zoom.frame(natural, box, scale, x, y) });
}

const messages = [];
const samples = ['', 'plain', 'a\nb', '  \n  ', 'Parse error on line 2:\n  x\n---^\nExpecting X', 'one\n\ntwo\nthree\n', 'Err\n\n', ' \t \n last line \n'];
for (let i = 0; i < 400 * Number(process.env.PELERS_FUZZ_SCALE ?? 1); i++) {
    const parts = [];
    for (let k = between(0, 5) | 0; k > 0; k--) parts.push(pick(['alpha', '  ', '', 'x y z', '\t', 'line', '---^']));
    samples.push(parts.join(pick(['\n', ' \n ', '\n\n'])));
}
for (const text of samples) messages.push({ text, summary: mount.summary(text) });
const errors = [new Error('boom\nmore'), 'string error', 42, null, undefined, new Error('')];
const messageOfs = errors.map(error => ({ error: typeof error === 'object' && error instanceof Error ? { message: error.message } : error === undefined ? '__undefined__' : error, text: mount.messageOf(error) }));

const rooms = [];
for (let i = 0; i < 500 * Number(process.env.PELERS_FUZZ_SCALE ?? 1); i++) {
    const clientWidth = pick([0, 10, 320, 1000.5, -3]) + between(-30, 30);
    const left = pick([0, 8, 10.5, NaN, -4]);
    const right = pick([0, 8, 3, NaN]);
    rooms.push({ clientWidth, left: Number.isNaN(left) ? null : left, right: Number.isNaN(right) ? null : right, room: windowModule.roomWithin(clientWidth, Number.isNaN(left) ? 0 : left, Number.isNaN(right) ? 0 : right) });
}

const constants = {
    STEP: zoom.STEP,
    PAN: zoom.PAN,
    GAP: zoom.GAP,
    BAR_STYLE: zoom.BAR_STYLE,
    BUTTON_STYLE: zoom.BUTTON_STYLE,
    ICONS: zoom.ICONS,
    LABELS: zoom.LABELS,
    OVER_STYLE: mount.OVER_STYLE,
    ALONE_STYLE: mount.ALONE_STYLE,
};

writeFileSync(join(OUT, 'ui.json'), `${JSON.stringify({ frames, messages, messageOfs, rooms, constants })}\n`);
console.log(`ui: ${frames.length} frames, ${messages.length} summaries, ${rooms.length} rooms`);
