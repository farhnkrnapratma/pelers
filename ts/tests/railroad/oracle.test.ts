import { describe, expect, it } from 'vitest';
import { mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';
import { NOTATIONS, corpusFor, grammar, longerAlternatives, pieces } from '../support/railroad-oracle.js';

for (const notation of NOTATIONS) {
  const { own, corpus } = corpusFor(notation);

  const compare = async (src: string): Promise<boolean> => {
    const expected = await referenceAst(notation.type, src);
    const actual = pelersAst(notation.parse, src);
    expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
    if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
    return expected.ok;
  };

  describe(`${notation.name} parser against the Mermaid grammar`, () => {
    it('uses the same token types in the same order, with the same longer alternatives', () => {
      expect(pelersTokens(notation.tokens)).toEqual(referenceTokens(notation.create, notation.key));
      expect(notation.tokens.map((t) => (t.longer ?? []).map((alt) => alt.name))).toEqual(longerAlternatives(notation));
    });

    // The tokenizer skips a type at characters its `first` leaves out, so `first` must leave out nothing a match can start with.
    it('lists every character a token type can start with', () => {
      const tails = [
        '',
        'a',
        'itle x',
        'ccTitle: x',
        'ccDescr {x}',
        '%{x}%%',
        '% c',
        '--\n---\n',
        '-beta',
        '*x*)',
        '*x*/',
        'x"',
        "x'",
        ' x ?',
        '*2',
        'x41',
        '\n',
        ' ',
      ];
      for (const type of notation.tokens) {
        if (type.first === undefined || typeof type.pattern === 'string') continue;
        for (let c = 0; c < 128; c++) {
          const ch = String.fromCharCode(c);
          if (type.first.includes(ch)) continue;
          for (const tail of tails) {
            type.pattern.lastIndex = 0;
            expect(type.pattern.test(ch + tail), `${type.name} at ${JSON.stringify(ch + tail)}`).toBe(false);
            if (type.scanner)
              expect(type.scanner()(ch + tail, 0), `${type.name} scanner at ${JSON.stringify(ch + tail)}`).toBe(-1);
          }
        }
      }
    });

    it('agrees on every spec and documentation input', async () => {
      expect(own.length).toBeGreaterThan(8);
      for (const src of corpus) await compare(src);
    }, 60_000);

    // A quarter each: mutated corpus inputs, random token pieces, generated grammars, and mutated generated grammars.
    it('agrees on mutated and generated inputs', async () => {
      const count = Number(process.env.FUZZ ?? 1500);
      const rnd = random(Number(process.env.SEED ?? 7));
      const mutated = mutator(corpus, notation.fragments, rnd);
      const accepted = [0, 0, 0, 0];
      for (let i = 0; i < count; i++) {
        const kind = i % 4;
        let src = kind === 0 ? mutated() : kind === 1 ? pieces(notation, rnd) : grammar(notation, rnd);
        if (kind === 3) src = mutator([src], notation.fragments, rnd)();
        if (src.length <= 2000 && (await compare(src))) accepted[kind]++;
      }
      if (process.env.FUZZ_STATS) console.log(`${notation.name}: accepted ${accepted.join(', ')} of ${count / 4} each`);
      // Both outcomes must be common, or the comparison says little.
      const total = accepted[0] + accepted[1] + accepted[2] + accepted[3];
      expect(total).toBeGreaterThan(count / 8);
      expect(total).toBeLessThan(count * 0.8);
    }, 3_600_000);
  });
}
