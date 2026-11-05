import { isSpace, isWord } from '../../util/chars.js';

export const enum T {
  END,
  EOF,
  SPACELINE,
  NL,
  ISHIKAWA,
  SPACELIST,
  TEXT,
}

export const TOKEN_NAMES = ['$end', 'EOF', 'SPACELINE', 'NL', 'ISHIKAWA', 'SPACELIST', 'TEXT'];

export interface Tokens {
  types: number[];
  starts: number[];
  ends: number[];
}

function keyword(src: string, at: number): number {
  const word = 'ishikawa-beta';
  let k = 0;
  while (k < 13) {
    const c = src.charCodeAt(at + k);
    const w = word.charCodeAt(k);
    if (c !== w && (c + 32 !== w || c < 65 || c > 90)) break;
    k++;
  }
  // `ishikawa-beta`, or failing that `ishikawa`, either one ending at a word boundary.
  if (k === 13 && !isWord(src.charCodeAt(at + 13))) return 13;
  return k >= 8 && !isWord(src.charCodeAt(at + 8)) ? 8 : 0;
}

// Produces the tokens of Mermaid's Jison lexer for Ishikawa diagrams, where the first rule that
// matches wins: white space up to a comment or up to the last line break in it is a blank line,
// a single line break before text is a new line, other white space is indentation, and the rest
// of a line is text.
export function tokenize(src: string): Tokens {
  const types: number[] = [];
  const starts: number[] = [];
  const ends: number[] = [];
  const n = src.length;
  let i = 0;
  while (i < n) {
    let j = i;
    let lastBreak = -1;
    while (j < n && isSpace(src.charCodeAt(j))) {
      if (src.charCodeAt(j) === 10) lastBreak = j;
      j++;
    }
    let type: number;
    let end: number;
    if (src.charCodeAt(j) === 37 && src.charCodeAt(j + 1) === 37) {
      type = T.SPACELINE;
      end = j + 2;
      while (end < n) {
        const c = src.charCodeAt(end);
        if (c === 10 || c === 13 || c === 0x2028 || c === 0x2029) break;
        end++;
      }
    } else if (j === i) {
      const length = keyword(src, i);
      if (length > 0) {
        type = T.ISHIKAWA;
        end = i + length;
      } else {
        type = T.TEXT;
        end = src.indexOf('\n', i);
        if (end === -1) end = n;
      }
    } else if (lastBreak > i) {
      type = T.SPACELINE;
      end = lastBreak + 1;
    } else if (lastBreak === i) {
      type = T.NL;
      end = i + 1;
    } else {
      type = T.SPACELIST;
      end = j;
    }
    types.push(type);
    starts.push(i);
    ends.push(end);
    i = end;
  }
  types.push(T.EOF, T.END);
  starts.push(n, n);
  ends.push(n, n);
  return { types, starts, ends };
}
