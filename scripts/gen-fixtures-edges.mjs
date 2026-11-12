import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const edges = await import(join(root, 'ts/src/svg/edges.ts'));

let seed = 987654;
const random = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    return seed / 0x7fffffff;
};
const between = (low, high) => low + random() * (high - low);
const pick = list => list[Math.floor(random() * list.length)];

const CURVES = [undefined, 'linear', 'step', 'stepBefore', 'stepAfter', 'basis'];
const TRIMS = [0, 1, 4, 7, 8, 9.5];
const KINDS = ['arrow_point', 'arrow_circle', 'arrow_cross', 'none', ''];

const routes = [];
for (let i = 0; i < 400 * SCALE; i++) {
    const points = 2 + Math.floor(random() * 4);
    const route = [];
    let x = Math.round(between(0, 300));
    let y = Math.round(between(0, 300));
    let axis = random() < 0.5 ? 0 : 1;
    route.push(x, y, axis);
    for (let k = 1; k < points; k++) {
        const roll = random();
        if (roll < 0.7) {
            if (axis === 0) y = Math.round(between(0, 300));
            else x = Math.round(between(0, 300));
        } else {
            x = Math.round(between(0, 300) * 10) / 10;
            y = Math.round(between(0, 300) * 10) / 10;
        }
        axis = 1 - axis;
        route.push(x, y, axis);
    }
    routes.push({ route, curve: pick(CURVES), startTrim: pick(TRIMS), endTrim: pick(TRIMS), startReach: random() < 0.3 ? between(0, 30) : undefined, endReach: random() < 0.3 ? between(0, 30) : undefined });
}
const pathCases = routes.map(c => {
    const args = [c.route, c.curve, c.startTrim, c.endTrim, c.startReach, c.endReach];
    const out = edges.routePath(...args.map(v => (v === undefined ? undefined : v)));
    return { ...c, out: { d: out.d, tokens: out.tokens ?? null, sx: out.sx, sy: out.sy, sdx: out.sdx, sdy: out.sdy, ex: out.ex, ey: out.ey, edx: out.edx, edy: out.edy } };
});

const markerCases = [];
for (let i = 0; i < 200 * SCALE; i++) {
    const kind = pick(KINDS);
    const x = between(0, 200), y = between(0, 200);
    const len = Math.hypot(between(-1, 1), between(-1, 1)) || 1;
    const dx = between(-1, 1) / len, dy = between(-1, 1) / len;
    const color = pick(['var(--_bg)', '#abc', 'red']);
    const stroke = pick(['none', 'var(--_bg)']);
    markerCases.push({ kind, x, y, dx, dy, color, stroke, out: edges.marker(kind, x, y, dx, dy, color, stroke), trim: edges.markerTrim(kind) });
}

const LINES = ['', 'stroke:red;stroke-width:2px', 'stroke:none;', 'stroke-width:3;stroke-opacity:0.5;', '"stroke-width:calc(1px + 1);"', 'opacity:0.4;stroke-width:1px;', 'stroke:transparent;', 'fill:#fff;stroke:#333;'];
const casingCases = [];
for (let i = 0; i < 200 * SCALE; i++) {
    const pc = pathCases[i % pathCases.length];
    const markers = edges.marker(pick(KINDS), pc.out.sx, pc.out.sy, pc.out.sdx, pc.out.sdy, 'var(--_bg)', 'none') + edges.marker(pick(KINDS), pc.out.ex, pc.out.ey, pc.out.edx, pc.out.edy, 'var(--_bg)', 'none');
    const line = pick(LINES);
    const thick = random() < 0.3;
    casingCases.push({ d: pc.out.d, markers, line, thick, out: edges.casing(pc.out.d, markers, line, thick) });
}

const FILL_SHAPES = ['fill:#ffeecc;stroke:#333;', 'fill:var(--_s);', 'fill:#222', 'stroke:red;fill:blue', 'rect', ''];
const HAND_DS = [
    'M10,10 A5,5 0 0 1 20,20', 'M10 10 A 0 5 0 0 1 20 20', 'M1e2 1E1 L 3e-1,2 A 4 4 0 1 0 5 5', 'M0 0 A5 5 0 0 0 0 0',
    'M0,0 A 3 3 0 1 1 6 0 A 3 3 0 1 0 0 0', 'M1 1 L2 2 Z', 'M1 1 C 2 3 4 5 6 7 S 8 9 10 11 Q 1 2 3 4 T 5 6 H 7 V 8 z', 'M1 -2.5e+1 L 1e-2 2E3',
    'M5 5 A 2 0 0 0 9 9', 'M5 5 A 0 2 0 1 0 9 9', 'M-5.5 -6 L-1.25e-3,+4 A 10 4 30 0 1 12 3', 'M0 0 A 1e1 1e1 0 1 1 1e1 0 A 3 2 1 0 0 4 4 L 1 1',
];
const fillCases = [];
for (let i = 0; i < 300 * SCALE; i++) {
    const pc = pathCases[i % pathCases.length];
    const handD = i % 4 === 0 ? pick(HAND_DS) : undefined;
    const fills = [];
    const count = Math.floor(between(0, 4));
    for (let f = 0; f < count; f++) {
        const l = between(-50, 250), t = between(-50, 250);
        fills.push({ box: [l, t, l + between(10, 300), t + between(10, 300)], shape: pick(FILL_SHAPES) });
    }
    const tips = [];
    const pairs = Math.floor(between(0, 3));
    for (let k = 0; k < 2 * pairs; k++) tips.push(between(-50, 350));
    const line = pick(LINES);
    const d = handD ?? pc.out.d;
    const tokens = handD === undefined ? pc.out.tokens : undefined;
    fillCases.push({
        d,
        tokens: tokens ?? null,
        tips,
        fills,
        line,
        overFills: edges.overFills(d, tips, fills, line, tokens ?? undefined) ?? null,
    });
}

const hollowCases = [];
for (let i = 0; i < 80 * SCALE; i++) {
    const markers = ' fill="var(--_bg)" x fill="var(--_bg)"'.slice(0, pick([0, 10, 37]) + 1) + ' fill="var(--_bg)"';
    const fill = { box: [0, 0, 1, 1], shape: pick(FILL_SHAPES) };
    hollowCases.push({ markers, fill, out: edges.hollowOn(markers, fill) });
}

const loopCases = [];
for (let i = 0; i < 150 * SCALE; i++) {
    const c = { x: between(0, 300), y: between(0, 300) };
    const w = between(20, 200), h = between(20, 120);
    const k = Math.floor(between(0, 4));
    const sideways = random() < 0.5;
    const base = between(10, 40), trim = pick(TRIMS);
    const e = { labelW: between(0, 60), labelH: between(0, 20), labelX: 0, labelY: 0 };
    const out = edges.loopPath(c, w, h, k, sideways, base, trim, e);
    loopCases.push({ c, w, h, k, sideways, base, trim, labelW: e.labelW, labelH: e.labelH, out: { d: out.d, sx: out.sx, sy: out.sy, sdx: out.sdx, sdy: out.sdy, ex: out.ex, ey: out.ey, edx: out.edx, edy: out.edy }, labelX: e.labelX, labelY: e.labelY });
}

writeFileSync(join(OUT, 'svg-edges.json'), `${JSON.stringify({ pathCases, markerCases, casingCases, fillCases, hollowCases, loopCases })}\n`);
console.log(`edges: ${pathCases.length} paths, ${markerCases.length} markers, ${casingCases.length} casings, ${fillCases.length} fills, ${loopCases.length} loops`);
