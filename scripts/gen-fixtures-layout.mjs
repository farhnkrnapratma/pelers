import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const { layered, lnode, ledge } = await import(join(root, 'ts/src/layout/layered.ts'));

const KIND_NODE = 0;

function mulberry32(seed) {
    return () => {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

const cases = [];
for (let seed = 1; seed <= 600; seed++) {
    const random = mulberry32(seed * 7919);
    const between = (low, high) => low + Math.floor(random() * (high - low + 1));
    const chance = p => random() < p;
    const count = seed % 10 === 0 ? between(30, 60) : between(1, 14);
    const nodes = [];
    for (let i = 0; i < count; i++) {
        const w = between(10, 120);
        const h = between(10, 60);
        const node = { w, h, kind: KIND_NODE, pin: 0, hint: null, key: chance(0.3) ? between(0, 9) : null, span: chance(0.25) ? between(0, w) : 0 };
        nodes.push(node);
    }
    const edges = [];
    const edgeCount = count === 1 ? 0 : between(0, count * 2);
    for (let i = 0; i < edgeCount; i++) {
        const tail = between(0, count - 1);
        let head = between(0, count - 1);
        if (head === tail) head = (tail + 1) % count;
        const labelled = chance(0.3);
        edges.push({
            tail,
            head,
            minlen: chance(0.2) ? 2 : 1,
            labelW: labelled ? between(10, 60) : 0,
            labelH: labelled ? between(10, 20) : 0,
            tailDx: chance(0.1) ? between(-10, 10) : 0,
            headDx: chance(0.1) ? between(-10, 10) : 0,
            tailFixed: chance(0.05),
            headFixed: chance(0.05),
        });
    }
    const tight = chance(0.3);
    const options = {
        nodeSep: tight ? 20 : 40,
        edgeSep: 16,
        rankSep: tight ? 36 : 48,
        portSep: 20,
        tracks: chance(0.2),
        straight: chance(0.1),
        apart: chance(0.2),
        headRoom: [0, 8, 10][between(0, 2)],
        orderOnly: false,
    };
    cases.push({ nodes, edges, options });
}

const toNaN = v => (v === null ? NaN : v);
const results = [];
for (const c of cases) {
    const lnodes = c.nodes.map(n => {
        const ln = lnode(n.w, n.h, KIND_NODE);
        ln.pin = n.pin;
        ln.hint = toNaN(n.hint);
        ln.key = toNaN(n.key);
        ln.span = n.span;
        return ln;
    });
    const ledges = c.edges.map(e => {
        const le = ledge(e.tail, e.head, e.minlen, e.labelW, e.labelH);
        le.tailDx = e.tailDx;
        le.headDx = e.headDx;
        le.tailFixed = e.tailFixed;
        le.headFixed = e.headFixed;
        return le;
    });
    const result = layered(lnodes, ledges, { ...c.options });
    results.push({
        input: c,
        output: {
            width: result.width,
            height: result.height,
            nodes: lnodes.map(n => ({ x: n.x, y: n.y, rank: n.rank })),
            edges: ledges.map(e => ({
                reversed: e.reversed,
                tailDx: e.tailDx,
                headDx: e.headDx,
                points: e.points,
                labelX: e.labelX,
                labelY: e.labelY,
            })),
        },
    });
}

writeFileSync(join(root, 'tests/fixtures/layout-layered.json'), `${JSON.stringify(results)}\n`);
console.log(`layout: ${results.length} layered cases`);

const { compoundLayout } = await import(join(root, 'ts/src/layout/compound.ts'));

const DIRS = [undefined, 'TB', 'BT', 'LR', 'RL'];
const compoundCases = [];
for (let seed = 1; seed <= 400; seed++) {
    const random = mulberry32(seed * 104729);
    const between = (low, high) => low + Math.floor(random() * (high - low + 1));
    const chance = p => random() < p;
    const groupCount = seed % 5 === 0 ? 0 : between(1, 5);
    const leafCount = between(2, 14);
    const nodes = [];
    for (let g = 0; g < groupCount; g++) {
        const parent = g > 0 && chance(0.4) ? between(0, g - 1) : -1;
        nodes.push({
            w: 0,
            h: 0,
            parent,
            isGroup: true,
            dir: DIRS[between(0, DIRS.length - 1)],
            padX: 10,
            padTop: 30,
            padBottom: 10,
            minW: chance(0.3) ? between(40, 160) : 0,
            seq: 0,
            span: chance(0.3) ? between(0, 120) : 0,
        });
    }
    for (let l = 0; l < leafCount; l++) {
        const parent = groupCount > 0 && chance(0.6) ? between(0, groupCount - 1) : -1;
        const w = between(20, 110);
        const h = between(16, 60);
        nodes.push({ w, h, parent, isGroup: false, dir: undefined, padX: 0, padTop: 0, padBottom: 0, minW: 0, seq: 0, span: chance(0.2) ? between(0, w) : 0 });
    }
    nodes.forEach((node, i) => {
        node.seq = i;
    });
    const edges = [];
    const edgeCount = between(1, nodes.length * 2);
    for (let i = 0; i < edgeCount; i++) {
        const src = between(0, nodes.length - 1);
        let dst = between(0, nodes.length - 1);
        if (dst === src) dst = (src + 1) % nodes.length;
        const labelled = chance(0.25);
        edges.push({ src, dst, minlen: 1, labelW: labelled ? between(10, 50) : 0, labelH: labelled ? between(10, 18) : 0 });
    }
    const rootDir = ['TB', 'BT', 'LR', 'RL'][between(0, 3)];
    const tight = chance(0.3);
    const options = { nodeSep: tight ? 20 : 40, edgeSep: 16, rankSep: tight ? 36 : 48, portSep: 20, tracks: chance(0.2), straight: chance(0.1), apart: false, headRoom: [0, 8][between(0, 1)], orderOnly: false };
    compoundCases.push({ nodes, edges, rootDir, options });
}

const compoundResults = [];
for (const c of compoundCases) {
    const cnodes = c.nodes.map(n => ({ ...n }));
    const cedges = c.edges.map(e => ({ src: e.src, dst: e.dst, minlen: e.minlen, labelW: e.labelW, labelH: e.labelH, route: [], labelX: 0, labelY: 0 }));
    const result = compoundLayout(cnodes, cedges, c.rootDir, { ...c.options });
    compoundResults.push({
        input: c,
        output: {
            width: result.width,
            height: result.height,
            nodes: cnodes.map(n => ({ x: n.x, y: n.y, w: n.w, h: n.h, parent: n.parent })),
            edges: cedges.map(e => ({ src: e.src, dst: e.dst, route: e.route, labelX: e.labelX, labelY: e.labelY })),
        },
    });
}
writeFileSync(join(root, 'tests/fixtures/layout-compound.json'), `${JSON.stringify(compoundResults)}\n`);
console.log(`layout: ${compoundResults.length} compound cases`);

const { wrapComponents } = await import(join(root, 'ts/src/layout/components.ts'));
const componentResults = [];
for (let seed = 1; seed <= 400; seed++) {
    const random = mulberry32(seed * 15485863);
    const between = (low, high) => low + Math.floor(random() * (high - low + 1));
    const chance = p => random() < p;
    const count = between(1, 12);
    const nodes = [];
    for (let i = 0; i < count; i++) {
        const parent = i > 0 && chance(0.2) ? between(0, i - 1) : -1;
        nodes.push({ w: between(10, 120), h: between(10, 60), parent, isGroup: false, dir: undefined, padX: 0, padTop: 0, padBottom: 0, minW: 0, seq: i, span: 0, x: between(0, 400), y: between(0, 300) });
    }
    const edges = [];
    for (let i = 0; i < between(0, count * 2); i++) {
        const src = between(0, count - 1);
        const dst = between(0, count - 1);
        if (src === dst) continue;
        const route = [];
        const points = between(2, 4);
        for (let k = 0; k < points; k++) route.push(between(0, 500), between(0, 400), chance(0.5) ? 0 : 1);
        const labelled = chance(0.3);
        edges.push({ src, dst, minlen: 1, labelW: labelled ? between(5, 40) : 0, labelH: labelled ? between(5, 20) : 0, route, labelX: between(0, 400), labelY: between(0, 300) });
    }
    const width = between(100, 600);
    const gap = between(0, 40);
    const cnodes = nodes.map(n => ({ ...n }));
    const cedges = edges.map(e => ({ ...e, route: [...e.route] }));
    const result = wrapComponents(cnodes, cedges, width, gap);
    componentResults.push({
        input: { nodes, edges, width, gap },
        output: result
            ? { width: result.width, height: result.height, nodes: cnodes.map(n => ({ x: n.x, y: n.y })), edges: cedges.map(e => ({ route: e.route, labelX: e.labelX, labelY: e.labelY })) }
            : null,
    });
}
writeFileSync(join(root, 'tests/fixtures/layout-components.json'), `${JSON.stringify(componentResults)}\n`);
console.log(`layout: ${componentResults.length} component cases`);

const { laneLayout } = await import(join(root, 'ts/src/layout/lanes.ts'));
const laneResults = [];
for (let seed = 1; seed <= 300; seed++) {
    const random = mulberry32(seed * 32452843);
    const between = (low, high) => low + Math.floor(random() * (high - low + 1));
    const chance = p => random() < p;
    const laneCount = between(1, 4);
    const leafCount = between(2, 12);
    const nodes = [];
    for (let g = 0; g < laneCount; g++) {
        nodes.push({ w: 0, h: 0, parent: -1, isGroup: true, dir: undefined, padX: 10, padTop: 30, padBottom: 10, minW: chance(0.3) ? between(40, 160) : 0, seq: g, span: 0 });
    }
    for (let l = 0; l < leafCount; l++) {
        const parent = chance(0.85) ? between(0, laneCount - 1) : -1;
        const w = between(20, 110);
        const h = between(16, 60);
        nodes.push({ w, h, parent, isGroup: false, dir: undefined, padX: 0, padTop: 0, padBottom: 0, minW: 0, seq: laneCount + l, span: 0 });
    }
    const edges = [];
    const edgeCount = between(1, nodes.length * 2);
    for (let i = 0; i < edgeCount; i++) {
        const src = laneCount + between(0, leafCount - 1);
        let dst = laneCount + between(0, leafCount - 1);
        if (dst === src) dst = laneCount + ((dst - laneCount + 1) % leafCount);
        const labelled = chance(0.25);
        edges.push({ src, dst, minlen: 1, labelW: labelled ? between(10, 50) : 0, labelH: labelled ? between(10, 18) : 0 });
    }
    const dir = ['TB', 'BT', 'LR', 'RL'][between(0, 3)];
    const options = { nodeSep: 40, edgeSep: 16, rankSep: 48, portSep: 20, tracks: false, straight: chance(0.1), apart: false, headRoom: 0, orderOnly: false };
    laneResults.push({ input: { nodes, edges, dir, options } });
}
for (const c of laneResults) {
    const cnodes = c.input.nodes.map(n => ({ ...n }));
    const cedges = c.input.edges.map(e => ({ src: e.src, dst: e.dst, minlen: e.minlen, labelW: e.labelW, labelH: e.labelH, route: [], labelX: 0, labelY: 0 }));
    const result = laneLayout(cnodes, cedges, c.input.dir, { ...c.input.options });
    c.output = {
        width: result.width,
        height: result.height,
        nodes: cnodes.map(n => ({ x: n.x, y: n.y, w: n.w, h: n.h, padTop: n.padTop })),
        edges: cedges.map(e => ({ route: e.route, labelX: e.labelX, labelY: e.labelY })),
    };
}
writeFileSync(join(root, 'tests/fixtures/layout-lanes.json'), `${JSON.stringify(laneResults)}\n`);
console.log(`layout: ${laneResults.length} lane cases`);
