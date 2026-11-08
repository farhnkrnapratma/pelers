import { findToken, patchTitle, patchTokens } from './token-patch.js';

type Matched = { tokenType: { name: string } }[];

// Match Pele: allow longer names and preserve glued option values.
patchTokens('RadarModule', (tokens) => {
  const rest = /[-\w]*\w/y;
  for (const [name, option, source] of [
    ['GRATICULE', 'graticule', 'circle|polygon'],
    ['BOOLEAN', 'showLegend', 'true|false'],
  ]) {
    const token = findToken(tokens, name, source);
    const pattern = new RegExp(source, 'y');
    token.PATTERN = {
      source,
      exec(text: string, offset: number, matched: Matched): RegExpExecArray | null {
        pattern.lastIndex = offset;
        const m = pattern.exec(text);
        if (m === null || matched.at(-1)?.tokenType.name === option) return m;
        rest.lastIndex = offset + m[0].length;
        return rest.test(text) ? null : m;
      },
    };
  }
  patchTitle(tokens, String.raw`[-\w]*\w`);
});
