import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const shapes = await import(join(root, 'ts/src/svg/shapes.ts'));

const NAMES = ['rect', 'rounded', 'stadium', 'fr-rect', 'cyl', 'circle', 'dbl-circ', 'ellipse', 'diam', 'hex', 'odd', 'lean-r', 'lean-l', 'trap-b', 'trap-t', 'text', 'sm-circ', 'f-circ', 'fr-circ', 'fork', 'notch-rect', 'lin-rect', 'div-rect', 'win-pane', 'tri', 'flip-tri', 'sl-rect', 'notch-pent', 'hourglass', 'delay', 'h-cyl', 'cross-circ', 'st-rect', 'doc', 'flag', 'datastore', 'folder', 'bucket', 'console', 'browser', 'person', 'bang', 'cloud', 'brace', 'brace-r', 'braces', 'bolt', 'lin-cyl', 'curv-trap', 'docs', 'bow-rect', 'tag-rect', 'tag-doc', 'lin-doc', 'collapsedGroup', 'unknown-shape'];

let seed = 12345;
const random = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    return seed / 0x7fffffff;
};
const between = (low, high) => low + random() * (high - low);
const ATTR = ' fill="var(--_s)" stroke="var(--_b)"';
const LINE = ' stroke="var(--_b)"';

const cases = [];
for (const shape of NAMES) {
    for (let i = 0; i < 12 * SCALE; i++) {
        const tw = Math.round(between(0, 200) * 100) / 100;
        const th = Math.round(between(0, 80) * 100) / 100;
        const size = shapes.shapeSize(shape, tw, th);
        const w = size.w;
        const h = size.h;
        const insetT = [0, 1, 2, 3].map(() => (random() - 0.5) * w);
        const insets = [0, 1, 2, 3].map((side, k) => shapes.shapeInset(shape, w, h, side, insetT[k]));
        const spans = [false, true].map(sideways => shapes.shapeSpan(shape, w, h, sideways));
        cases.push({
            shape,
            tw,
            th,
            size,
            draw: shapes.drawShape(shape, w, h, ATTR, LINE),
            drawPlain: shapes.drawShape(shape, w, h, '', ''),
            insetT,
            insets,
            spans,
            hasLabel: shapes.shapeHasLabel(shape),
            filled: shapes.shapeFilled(shape),
        });
    }
}

const routes = [];
for (let i = 0; i < 200 * SCALE; i++) {
    const c = { x: between(0, 300), y: between(0, 300), w: between(10, 120), h: between(10, 80), isGroup: random() < 0.1 };
    const route = [between(0, 400), between(0, 400), 0, between(0, 400), between(0, 400), 0];
    const at = random() < 0.5 ? 0 : 3;
    const side = shapes.endSide(route, at, c);
    const view = { shape: NAMES[Math.floor(random() * NAMES.length)], w: c.w - between(0, 8), h: c.h - between(0, 8) };
    const inserted = route.slice();
    shapes.insetRoute(inserted, at, view, c);
    routes.push({ c, route, at, side, view, inserted });
}

writeFileSync(join(OUT, 'svg-shapes.json'), `${JSON.stringify({ cases, routes })}\n`);
console.log(`shapes: ${cases.length} shape cases, ${routes.length} route cases`);
