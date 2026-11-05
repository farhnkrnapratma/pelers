import { describe, expect, it } from 'vitest';
import { comparable, loadReference, peleReference } from '../support/agentflow-reference.js';

// Compares Pele's agentflow model with what Mermaid 12.1.0 itself built for the same text, taken
// from Mermaid's browser build: nodes, edges, flows, classes, the layout data with shapes, parents
// and palette slots, element positions, diagnostics and the semantic model.

// js-yaml's errors for an alias and for a lone colon in metadata. Pele's YAML reader has no aliases and
// takes both as text, so it accepts these diagrams where Mermaid does not.
const LENIENT_YAML = /alias|explicit mapping pair/;

const reference = loadReference();
const plain = (value: unknown): unknown => comparable(JSON.parse(JSON.stringify(value)));

describe('agentflow model against Mermaid', () => {
  it('has a reference to compare with', () => {
    expect(reference.length).toBeGreaterThan(200);
    expect(reference.filter((r) => r.model).length).toBeGreaterThan(150);
  });

  it('accepts and rejects the same inputs', () => {
    for (const ref of reference) {
      const mine = peleReference(ref.src);
      const where = JSON.stringify(ref.src);
      if (ref.error !== undefined && LENIENT_YAML.test(ref.error)) {
        expect(mine.model, `Mermaid: ${ref.error}; ${where}`).toBeDefined();
      } else {
        expect(mine.model !== undefined, `Mermaid: ${ref.error}; Pele: ${mine.error}; ${where}`).toBe(
          ref.model !== undefined
        );
      }
    }
  });

  it('builds the same model', () => {
    for (const ref of reference) {
      if (ref.model === undefined) continue;
      expect(plain(peleReference(ref.src).model), JSON.stringify(ref.src)).toEqual(plain(ref.model));
    }
  });
});
