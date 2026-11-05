import { describe, expect, it } from 'vitest';
import { FlowDb } from '../../src/diagrams/flowchart/db.js';
import { parseFlowchart } from '../../src/diagrams/flowchart/parser.js';
import { PeleError } from '../../src/errors.js';

function parse(src: string): FlowDb {
  const db = new FlowDb();
  parseFlowchart(src, db);
  return db;
}

describe('flowchart parser', () => {
  it('merges a repeated subgraph id into one subgraph', () => {
    const db = parse('flowchart LR\nsubgraph S\n  x\nend\nsubgraph S\n  y\nend');
    expect(db.subgraphs).toHaveLength(1);
    expect(db.subgraphs[0].nodes).toEqual(['x', 'y']);
  });

  // Mermaid 12.1.0 throws a TypeError here; an unnamed subgraph is clearly meant to work.
  it('accepts a subgraph with no name', () => {
    const db = parse('flowchart TD\nsubgraph\n  a --> b\nend');
    expect(db.subgraphs).toHaveLength(1);
    expect(db.subgraphs[0].id).toBe('subGraph0');
    expect(db.subgraphs[0].nodes).toEqual(['b', 'a']);
  });

  it('reports a linkStyle index that does not exist', () => {
    expect(() => parse('graph TD\nA-->B\nlinkStyle 1 stroke-width:1px;')).toThrow(
      'The index 1 for linkStyle is out of bounds. Valid indices for linkStyle are between 0 and 0.'
    );
  });

  it('reports the position of a syntax error', () => {
    try {
      parse('graph TD\nA-->B\nB --> [oops]');
      expect.unreachable();
    } catch (error) {
      expect(error).toBeInstanceOf(PeleError);
      const e = error as PeleError;
      expect(e.line).toBe(3);
      expect(e.column).toBe(7);
      expect(e.message).toContain("got 'SQS'");
    }
  });

  // Mermaid skips this spec because its parser needs minutes for it.
  it('parses the huge diagram from the Mermaid specs quickly', () => {
    const body = ('A-->B;B-->A;'.repeat(415) + 'A-->B;').repeat(57) + 'A-->B;B-->A;'.repeat(275);
    const started = performance.now();
    const db = parse(`graph LR;${body}`);
    expect(db.edges[0].type).toBe('arrow_point');
    expect(db.edges.length).toBe(47917);
    expect(db.nodes.size).toBe(2);
    expect(performance.now() - started).toBeLessThan(2000);
  });

  it('honours an edge limit when one is set', () => {
    const db = new FlowDb({ maxEdges: 2 });
    expect(() => parseFlowchart('graph TD\nA-->B-->C-->D', db)).toThrow(/Edge limit exceeded/);
  });
});
