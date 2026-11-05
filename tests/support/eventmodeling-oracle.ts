import * as reference from '@mermaid-js/parser';
import { PeleError } from '../../src/errors.js';
import { patchTitle, patchTokens } from './token-patch.js';

patchTokens('EventModelingModule', (tokens) => patchTitle(tokens, String.raw`\w`, 'EM_TITLE'));

// Event modeling trees hold cross-references. Mermaid reads only their text (`$refText`) and
// never links them, and Langium's reference objects cannot be walked, so a reference is
// compared as its text.
export function plainTree(node: unknown): unknown {
  if (Array.isArray(node)) return node.map(plainTree);
  if (node === null || typeof node !== 'object') return node;
  const text = Object.getOwnPropertyDescriptor(node, '$refText');
  if (text !== undefined) return { $refText: text.value as string };
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(node)) {
    if (key.startsWith('$') && key !== '$type') continue;
    if (value !== undefined) out[key] = plainTree(value);
  }
  return out;
}

export async function referenceTree(src: string): Promise<{ ok: boolean; ast?: unknown; error?: string }> {
  try {
    const parse = reference.parse as unknown as (type: string, text: string) => Promise<unknown>;
    return { ok: true, ast: plainTree(await parse('eventmodeling', src)) };
  } catch (e) {
    return { ok: false, error: String((e as Error).message) };
  }
}

export function peleTree(parse: (src: string) => unknown, src: string): { ok: boolean; ast?: unknown; error?: string } {
  try {
    return { ok: true, ast: plainTree(parse(src)) };
  } catch (e) {
    if (!(e instanceof PeleError)) throw e;
    return { ok: false, error: e.message };
  }
}
