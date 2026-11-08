import { resolve } from 'node:path';
import { tokenize } from '../../src/diagrams/block/lexer.js';
import { parseBlock, type BlockBuilder } from '../../src/diagrams/block/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/block/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

const oracle = createOracle(resolve('tests/compat/block/upstream/block.jison'));

function recorder() {
  let ids = 0;
  const rec = recordingYy(oracle.methods, {
    generateId: () => 'G' + ++ids,
    typeStr2Type: (text?: string) => `type(${text})`,
    edgeStrToEdgeData: (text: string) => `end(${text})`,
    edgeStrToEdgeStartData: (text: string) => `start(${text})`,
    edgeStrToThickness: (text: string) => `thickness(${text})`,
    edgeStrToPattern: (text: string) => `pattern(${text})`,
  });
  rec.yy.getLogger = () => ({ debug: () => {} });
  return rec;
}

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, recorder().yy).map(([name, text]) => [name, String(text)]);
}

export function peleTokens(src: string): Pair[] {
  const { types, texts } = tokenize(src);
  return types.map((t, i) => [TOKEN_NAMES[t], texts[i]] as Pair);
}

type Item = Record<string, unknown>;

// What Mermaid's grammar makes of a statement with more than one link: its left-recursive rule reads
// the id of the list built so far, so only the last link survives, and it starts at `undefined`.
// Pele keeps every link; this is the one place the two trees differ.
function collapse(chain: Item[]): Item[] {
  if (chain.length <= 3) return chain;
  const edge = chain[chain.length - 2];
  return [{}, { ...edge, id: `undefined-${String(edge.end)}`, start: undefined }, chain[chain.length - 1]];
}

function view(item: Item): Item {
  if (!Array.isArray(item.children) || item.type !== 'composite') return item;
  const out: Item = { ...item, children: mermaidView(item.children as (Item | Item[])[]) };
  if ('3' in out) {
    const header: Item[] = [];
    for (let k = 0; String(k) in out; k++) {
      header.push(out[String(k)] as Item);
      delete out[String(k)];
    }
    Object.assign(out, { ...collapse(header) });
  }
  return out;
}

export function mermaidView(document: (Item | Item[])[]): (Item | Item[])[] {
  const out = document.map((item) => (Array.isArray(item) ? collapse(item) : view(item)));
  const edge = out.findIndex((item) => !Array.isArray(item) && item.type === 'edge');
  if (edge > 0) out.push(...collapse(out.splice(edge - 1) as Item[]));
  return out;
}

const comparable = (calls: unknown[][]): unknown[][] =>
  calls.map((call) => (call[0] === 'setHierarchy' ? [call[0], mermaidView(call[1] as Item[])] : call));

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseBlock(src, yy as unknown as BlockBuilder);
    return { ok: true, calls: comparable(calls) };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
