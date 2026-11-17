import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';
import { NOTATIONS, corpusFor, grammar } from '../support/railroad-oracle.js';

const options = { measurer: metricsMeasurer };

function check(src: string, extra: object = {}): boolean {
  let svg: string;
  try {
    svg = render(src, { ...options, ...extra }).svg;
  } catch (error) {
    if (error instanceof PelersError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

// Double quotes end a quoted string in every notation, so each payload is also tried with them escaped or removed.
const quoted = (p: string): string => p.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
const bare = (p: string): string => p.replace(/"/g, "'");

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `railroad-beta\nr = terminal("${quoted(p)}") ;`,
  (p) => `railroad-beta\nr = terminal('${p.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}') ;`,
  (p) => `railroad-beta\nr = nonterminal("${quoted(p)}") ;`,
  (p) => `railroad-beta\nr = special("${quoted(p)}") ;`,
  (p) =>
    `railroad-beta\nr = sequence(terminal("${quoted(p)}"), choice(nonterminal("${quoted(p)}"), optional(special("${quoted(p)}")))) ;`,
  (p) => `railroad-beta\nr = zeroOrMore(oneOrMore(terminal("${quoted(p)}"))) ;`,
  (p) => `railroad-beta\nr = terminal("${p}") ;`,
  (p) => `railroad-beta\n${p} = terminal("a") ;`,
  (p) => `railroad-beta\nr = ${p} ;`,
  (p) => `railroad-beta\nr = ${p}("a") ;`,
  (p) => `railroad-beta\ntitle ${p}\nr = terminal("a") ;`,
  (p) => `railroad-beta\ntitle "${p}"\nr = terminal("a") ;`,
  (p) => `railroad-beta title ${p}`,
  (p) => `railroad-beta\naccTitle: ${p}\naccDescr: ${p}\nr = terminal("a") ;`,
  (p) => `railroad-beta\naccDescr {\n ${p}\n }\nr = terminal("a") ;`,
  (p) => `railroad-beta\n/* ${p} */\nr = terminal("a") ; %% ${p}`,
  (p) => `railroad-beta ${p}\nr = terminal("a") ;`,
  (p) => `railroad-ebnf-beta\nr = "${quoted(p)}" ;`,
  (p) => `railroad-ebnf-beta\nr = "${p}" ;`,
  (p) => `railroad-ebnf-beta\nr = ? ${p.replace(/[?;]/g, '')} ? ;`,
  (p) => `railroad-ebnf-beta\nr = ? ${p} ? ;`,
  (p) => `railroad-ebnf-beta\nr = a - "${quoted(p)}" | [ "${quoted(p)}" ] | { ? ${p.replace(/[?;]/g, '')} ? }+ ;`,
  (p) => `railroad-ebnf-beta\nr = ${p} ;`,
  (p) => `railroad-ebnf-beta\n${p} ::= a ;`,
  (p) => `railroad-ebnf-beta\nr = a (* ${p} *) b /* ${p} */ ;`,
  (p) => `railroad-ebnf-beta\ntitle ${p}\naccTitle: ${p}\naccDescr { ${p} }\nr = a ;`,
  (p) => `railroad-abnf-beta\nr = "${bare(p)}" ;`,
  (p) => `railroad-abnf-beta\nr = 2*3"${bare(p)}" [ "${bare(p)}" ] / *( "${bare(p)}" ) ;`,
  (p) => `railroad-abnf-beta\nr = "${p}" ;`,
  (p) => `railroad-abnf-beta\nr = ${p} ;`,
  (p) => `railroad-abnf-beta\nr = %x${p} ;`,
  (p) => `railroad-abnf-beta\nr = ${p}*${p}a ;`,
  (p) => `railroad-abnf-beta\n${p} = a ;`,
  (p) => `railroad-abnf-beta\nr = a ; ${p}\n ;\n`,
  (p) => `railroad-abnf-beta\ntitle "${p}"\nr = a ;`,
  (p) => `railroad-peg-beta\nR <- "${quoted(p)}" ;`,
  (p) => `railroad-peg-beta\nR <- !"${quoted(p)}" &'x' ( "${quoted(p)}" )* . ;`,
  (p) => `railroad-peg-beta\nR <- "${p}" ;`,
  (p) => `railroad-peg-beta\nR <- ${p} ;`,
  (p) => `railroad-peg-beta\nR <- !${p} ;`,
  (p) => `railroad-peg-beta\n${p} <- a ;`,
  (p) => `railroad-peg-beta\nR <- a # ${p}\n ;`,
  (p) => `railroad-peg-beta\ntitle ${p}\naccDescr: ${p}\nR <- a ;`,
  (p) => `---\ntitle: "${quoted(p)}"\n---\nrailroad-ebnf-beta\nr = a ;`,
  (p) => `---\ntitle: ${p}\n---\nrailroad-ebnf-beta\nr = a ;`,
  (p) =>
    `---\nconfig:\n  railroad:\n    arcRadius: "${quoted(p)}"\n    padding: "${quoted(p)}"\n    fontSize: "${quoted(p)}"\n    fontFamily: "${quoted(p)}"\n    terminalFill: "${quoted(p)}"\n    lineColor: "${quoted(p)}"\n    strokeWidth: "${quoted(p)}"\n    showMarkers: "${quoted(p)}"\n    markerRadius: "${quoted(p)}"\n---\nrailroad-ebnf-beta\nr = a "b" ? c ? ;`,
  (p) =>
    `%%{init: {"railroad": {"arcRadius": "${quoted(p)}", "strokeWidth": "${quoted(p)}"}, "themeVariables": {"fontFamily": "${quoted(p)}"}}}%%\nrailroad-ebnf-beta\nr = a+ ;`,
];

describe('inert railroad output', () => {
  it('holds for every corpus input', () => {
    for (const notation of NOTATIONS) for (const src of corpusFor(notation).corpus) check(src);
  }, 60_000);

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(700);
  }, 60_000);

  it('escapes what it shows', () => {
    const { svg } = render(
      'railroad-beta\ntitle <b onclick=alert(1)>T</b>\nr = sequence(terminal("<img src=x onerror=alert(1)>"), nonterminal("\\"><script>alert(1)</script>"), special("</text></g></svg><script>a&b</script>")) ;',
      options
    );
    expect(svg).not.toContain('<img');
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('onclick');
    expect(svg).toContain('&lt;img src=x onerror=alert(1)&gt;');
    expect(svg).toContain('data-id="&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;"');
    expect(svg).toContain('a&amp;b');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      const src = 'railroad-ebnf-beta\naccTitle: t\naccDescr: d\nr = a "b" ? c ? - d* ;';
      expect(check(src, { idPrefix: payload, fontFamily: payload })).toBe(true);
      expect(
        check(src, {
          config: {
            railroad: {
              arcRadius: payload,
              padding: payload,
              fontSize: payload,
              strokeWidth: payload,
              markerRadius: payload,
              showMarkers: payload,
            },
          },
        })
      ).toBe(true);
      expect(check(src, { config: { railroad: payload } })).toBe(true);
    }
  }, 60_000);

  it('holds for mutated and generated inputs', () => {
    const count = Number(process.env.FUZZ ?? 600);
    for (const notation of NOTATIONS) {
      const rnd = random(3);
      const fragments = [...PAYLOADS, ...notation.fragments];
      const next = mutator(corpusFor(notation).corpus, fragments, rnd);
      for (let i = 0; i < count; i++) {
        const src = i % 3 === 0 ? next() : mutator([grammar(notation, rnd)], fragments, rnd)();
        if (src.length <= 4000) check(src);
      }
    }
  }, 600_000);

  it('does not pollute prototypes', () => {
    render(
      'railroad-ebnf-beta\n__proto__ = constructor | toString hasOwnProperty ;\nconstructor = "__proto__" ;\n__proto__ = prototype ;',
      options
    );
    render(
      'railroad-beta\nr = sequence(nonterminal("__proto__"), terminal("constructor"), special("__proto__")) ;',
      options
    );
    render('railroad-ebnf-beta\nr = a ;', {
      ...options,
      config: { railroad: { __proto__: { arcRadius: 99 }, constructor: 1 } as never },
    });
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.keys(Object.prototype)).toEqual([]);
  });
});
