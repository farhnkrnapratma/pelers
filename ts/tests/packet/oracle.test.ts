import { describe, expect, it } from 'vitest';
import { PACKET_TOKENS, parsePacket } from '../../src/diagrams/packet/parser.js';
import { tokenize } from '../../src/diagrams/common/tokens.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';
import {
  loadTestStrings,
  pelersLexer,
  pelersLongerAlts,
  referenceLexer,
  referenceLongerAlts,
} from '../support/langium-extra.js';

const FRAGMENTS = [
  'packet',
  'packet-beta',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  'accDescr {}',
  '}',
  ':',
  ' : ',
  '-',
  '+',
  '--',
  '---',
  '---\n',
  '\n---\n',
  '"',
  "'",
  '"a"',
  "'b'",
  '1',
  '0',
  '15',
  '16-31',
  '+8',
  '01',
  '1.',
  '.5',
  '1e3',
  '\n',
  '\r\n',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '\\',
  '\\"',
  'x',
  'é',
  ';',
  ',',
  '#35;',
  '0-7: "a"',
  '\n+4: "b"',
  '\n8: "c"',
];

const corpus = [
  ...loadCorpus('packet', /packet/),
  ...loadTestStrings('packet', /packet/),
  'packet',
  'packet\naccDescr {}\n0: "a"',
  'packet-beta',
  'packet-betax',
  'packet-beta x',
  'packetx',
  'packet%% c',
  'packet-beta%% c\n0: "a"',
  'packet\n0-15: "Source"\n16: \'x\'\n+8: "y\\"z"',
  'packet 0-15: "a"',
  'packet 0-15: "a" 16: "b"',
  'packet\n0-15: "a"\n\n\n',
  'packet\n0 - 15 : "a"',
  'packet\n0-15 "a"',
  'packet\n-15: "a"',
  'packet\n+ 8: "a"',
  'packet\n+8-9: "a"',
  'packet\n08: "a"',
  'packet\n8.: "a"',
  'packet\n0-1: "a"\n---\nfoo\n---\n2: "b"',
  'packet\n0---\n---\n1: "b"',
  'packet\n0---\n---: "b"',
  'packet\n0---: "b"',
  'packet\n0-\n1: "b"',
  'packet\ntitle T\n0: "a"',
  'packet title T\n0: "a"',
  'packet\ntitle T\naccTitle: A\naccDescr: D\n',
  'packet\ntitle T 0: "a"',
  'packet\n99999999999999999999-5: "a"',
  'packet\n0: "a" %% c\n1: "b"',
  '\n\n packet \n\n 0: "a"',
  'packet\n0: "a\n b"',
  'packet\n0: ""',
  'packet\n0:',
  'packet\n0',
  'packet\n: "a"',
  'packet\n+0: "a"',
  'packet\n10-5: "a"',
  'packet-beta\npacket',
  'packet\n1-2-3: "a"',
  'packet\naccDescr {\n a\n b\n}\n0: "a"',
  '---\ntitle: x\n---\npacket\n0: "a"',
  '%%{init: {"theme":"dark"}}%%\npacket title A %% c\n0-3: "a" %% d\n',
];

const referenceLex = referenceLexer('createPacketServices', 'Packet');
const pelersLex = pelersLexer(PACKET_TOKENS, (src) => tokenize(src, PACKET_TOKENS, 'packet'));

async function compare(src: string): Promise<void> {
  expect(pelersLex(src), `tokens of ${JSON.stringify(src)}`).toBe(referenceLex(src));
  const expected = await referenceAst('packet', src);
  const actual = pelersAst(parsePacket, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('packet parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(pelersTokens(PACKET_TOKENS)).toEqual(referenceTokens('createPacketServices', 'Packet'));
    expect(pelersLongerAlts(PACKET_TOKENS)).toEqual(referenceLongerAlts('createPacketServices', 'Packet'));
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(40);
    for (const src of corpus) await compare(src);
  });

  it('agrees on mutated inputs', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 2000) await compare(src);
    }
  }, 600_000);
});
