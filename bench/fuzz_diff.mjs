import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const pelers = await import(join(root, 'index.native.js'));

const corpus = JSON.parse(readFileSync(join(root, 'bench', 'corpus.json'), 'utf8'));
const perSource = Number(process.env.PER_SOURCE ?? 30);
const seed0 = Number(process.env.SEED ?? 20260513);

const SPECIALS = ['"', "'", '[', ']', '(', ')', '{', '}', '|', ':', ';', ',', '-->', '---', '==>', '%%', '#', '\t', ' ', '\n', 'Ü', '中', '😀', '&', '<', '>', '\\', '$', '*', '+', '.', '?', '!', '~', '`', '%', '@'];

function rng(seed) {
    let s = seed >>> 0;
    return () => {
        s = (s + 0x6d2b79f5) >>> 0;
        let t = s;
        t = Math.imul(t ^ (t >>> 15), t | 1);
        t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

function tokens(source) {
    return source.split(/(\s+|[^\w\s])/).filter((t) => t.length > 0);
}

function mutate(source, random) {
    const lines = source.split('\n');
    const op = Math.floor(random() * 9);
    const at = (n) => Math.floor(random() * Math.max(1, n));
    switch (op) {
        case 0:
            if (lines.length > 2) lines.splice(at(lines.length - 1) + 1, 1);
            return lines.join('\n');
        case 1: {
            const i = at(lines.length);
            lines.splice(i, 0, lines[at(lines.length)]);
            return lines.join('\n');
        }
        case 2: {
            const i = at(lines.length);
            const j = at(lines.length);
            [lines[i], lines[j]] = [lines[j], lines[i]];
            return lines.join('\n');
        }
        case 3: {
            const toks = tokens(source);
            const t = toks[at(toks.length)];
            const pick = toks[at(toks.length)];
            toks[toks.indexOf(t)] = pick;
            return toks.join('');
        }
        case 4: {
            const pos = at(source.length + 1);
            return source.slice(0, pos) + SPECIALS[at(SPECIALS.length)] + source.slice(pos);
        }
        case 5: {
            const pos = at(source.length + 1);
            return source.slice(0, pos) + source.slice(pos + 1 + at(4));
        }
        case 6:
            return source.slice(0, at(source.length + 1));
        case 7: {
            const toks = tokens(source);
            toks.splice(at(toks.length), 0, SPECIALS[at(SPECIALS.length)]);
            return toks.join('');
        }
        default: {
            const i = at(lines.length);
            lines[i] = lines[i].replace(/\s+/g, ' ').trim();
            return lines.join('\n');
        }
    }
}

function renderPelers(source) {
    try {
        return { ok: true, svg: pelers.render(source) };
    } catch (error) {
        return { ok: false, message: String(error?.message ?? error).split('\n')[0] };
    }
}

const cases = [];
for (const [index, { type, source }] of corpus.entries()) {
    if (type === 'info') continue;
    const random = rng(seed0 + index);
    const texts = [source];
    for (let k = 0; k < perSource; k++) texts.push(mutate(source, random));
    for (const text of texts) cases.push({ index, type, source: text.toWellFormed() });
}

const workdir = mkdtempSync(join(root, 'bench', '.fuzz-'));
try {
    const casesPath = join(workdir, 'cases.json');
    const oraclePath = join(workdir, 'oracle.json');
    writeFileSync(casesPath, JSON.stringify(cases));
    execFileSync('bun', [join(root, 'bench', 'fuzz_oracle.mjs'), casesPath, oraclePath], {
        cwd: root,
        maxBuffer: 1 << 30,
        stdio: ['ignore', 'inherit', 'inherit'],
    });
    const oracle = JSON.parse(readFileSync(oraclePath, 'utf8'));

    const mismatches = [];
    let identical = 0;
    let bothFailed = 0;
    for (const [i, c] of cases.entries()) {
        const left = renderPelers(c.source);
        const right = oracle[i];
        if (!left.ok && !right.ok) {
            bothFailed++;
            continue;
        }
        if (left.ok && right.ok && Buffer.from(left.svg, 'utf8').equals(Buffer.from(right.svg, 'utf8'))) {
            identical++;
            continue;
        }
        mismatches.push({
            index: c.index,
            type: c.type,
            kind: left.ok === right.ok ? 'svg' : left.ok ? 'pelers-only-success' : 'oracle-only-success',
            source: c.source,
            pelersError: left.ok ? null : left.message,
            oracleError: right.ok ? null : right.message,
        });
    }

    const byType = {};
    for (const m of mismatches) byType[m.type] = (byType[m.type] ?? 0) + 1;
    const report = { oracle: "pele@1.13.0 npm under bun", seed: seed0, perSource, total: cases.length, identical, bothFailed, mismatches: mismatches.length, byType };
    writeFileSync(join(root, 'bench', 'fuzz-results.json'), JSON.stringify({ ...report, samples: mismatches.slice(0, 200) }, null, 2));
    console.log(JSON.stringify(report, null, 2));
} finally {
    rmSync(workdir, { recursive: true, force: true });
}
