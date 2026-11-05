import { isDigit, isLetter, isLineEnd, isSpace, isWord } from '../../util/chars.js';
import { wordAt } from '../common/caseless.js';
import { T } from './tokens.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

const enum S {
  INITIAL,
  title,
  acc_title,
  acc_descr,
  acc_descr_multiline,
  string,
  md_string,
  class_name,
  point_start,
  point_x,
  point_y,
}

// The token each state gives for the empty rest of the input, where it gives one.
const AT_END = [T.EOF, T.title_value, T.acc_title_value, T.acc_descr_value, T.acc_descr_multiline_value, T.STR];

const SINGLE = new Uint8Array(128);
for (const [chars, type] of [
  [':', T.COLON],
  ['+', T.PLUS],
  [',', T.COMMA],
  ['=', T.EQUALS],
  ['*', T.MULT],
  ['#', T.BRKT],
  ['_', T.UNDERSCORE],
  ['.', T.DOT],
  ['&', T.AMP],
  ['-', T.MINUS],
  [' \t\v\f', T.SPACE],
  [';', T.SEMI],
  ["!$%'`?\\/", T.PUNCTUATION],
] as const) {
  for (let k = 0; k < chars.length; k++) SINGLE[chars.charCodeAt(k)] = type;
}

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  // The last run scanned of each kind, so that a long run is not scanned again from each of its characters.
  let spaceFrom = 0;
  let spaceTo = 0;
  let dashFrom = 0;
  let dashTo = 0;
  let wsFrom = 0;
  let wsTo = 0;
  let gapFrom = 0;
  let gapTo = 0;

  const emit = (type: number, s: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(s, e));
    starts.push(s);
    p = e;
  };
  const push = (s: number): void => {
    stack.push(state);
    state = s;
  };
  const pop = (): void => {
    if (stack.length > 0) state = stack.pop()!;
  };
  const lineEnd = (q: number): number => {
    const e = src.indexOf('\n', q);
    return e === -1 ? n : e;
  };
  const spaces = (q: number): number => {
    while (src.charCodeAt(q) === 32) q++;
    return q;
  };
  const run = (q: number): number => {
    while (q < n && isSpace(src.charCodeAt(q))) q++;
    return q;
  };
  // Mermaid's `(1)|(0(.\d+)?)`, where the dot is any character within a line.
  const coordinate = (q: number): number => {
    const c = src.charCodeAt(q);
    if (c === 49) return q + 1;
    if (c !== 48) return -1;
    let e = q + 1;
    if (e < n && !isLineEnd(src.charCodeAt(e)) && isDigit(src.charCodeAt(e + 1))) {
      e += 2;
      while (isDigit(src.charCodeAt(e))) e++;
    }
    return e;
  };

  lexing: while (p < n) {
    const c = src.charCodeAt(p);
    const next = src.charCodeAt(p + 1);

    switch (state) {
      case S.INITIAL:
        break;

      case S.title:
      case S.acc_title:
      case S.acc_descr:
        emit(AT_END[state], p, lineEnd(p));
        pop();
        continue;

      case S.acc_descr_multiline:
        if (c === 125) {
          p++;
          pop();
        } else {
          const e = src.indexOf('}', p);
          emit(T.acc_descr_multiline_value, p, e === -1 ? n : e);
        }
        continue;

      case S.string:
        if (c === 34) {
          p++;
          pop();
        } else {
          const e = src.indexOf('"', p);
          emit(T.STR, p, e === -1 ? n : e);
        }
        continue;

      case S.md_string:
        if (c === 96 && next === 34) {
          p += 2;
          pop();
          continue;
        }
        if (c !== 96 && c !== 34) {
          let e = p + 1;
          for (let d = src.charCodeAt(e); e < n && d !== 96 && d !== 34; d = src.charCodeAt(++e));
          emit(T.MD_STR, p, e);
          continue;
        }
        break lexing;

      case S.class_name:
        if (isWord(c)) {
          let e = p + 1;
          while (isWord(src.charCodeAt(e))) e++;
          emit(T.class_name, p, e);
          pop();
          continue;
        }
        break lexing;

      case S.point_start: {
        const e = coordinate(p);
        if (e !== -1) {
          emit(T.point_x, p, e);
          push(S.point_x);
          continue;
        }
        const k = run(p);
        if (src.charCodeAt(k) === 93) {
          p = spaces(k + 1);
          pop();
          continue;
        }
        break lexing;
      }

      case S.point_x: {
        const k = run(p);
        if (src.charCodeAt(k) === 44) {
          p = run(k + 1);
          state = S.point_y;
          continue;
        }
        break lexing;
      }

      case S.point_y: {
        const e = coordinate(p);
        if (e !== -1) {
          emit(T.point_y, p, e);
          pop();
          continue;
        }
        break lexing;
      }
    }

    if (c === 37 && next === 37) {
      p = lineEnd(p + 2);
      continue;
    }
    if (c === 10 || c === 13) {
      let e = p + 1;
      for (let d = src.charCodeAt(e); d === 10 || d === 13; d = src.charCodeAt(++e));
      emit(T.NEWLINE, p, e);
      continue;
    }

    const lower = c | 32;
    if (lower === 116 && wordAt(src, p, 'title') && !isWord(src.charCodeAt(p + 5))) {
      emit(T.title, p, p + 5);
      push(S.title);
      continue;
    }
    if (lower === 97 && wordAt(src, p, 'acc')) {
      const descr = wordAt(src, p + 3, 'descr');
      if (descr || wordAt(src, p + 3, 'title')) {
        const k = run(p + 8);
        const d = src.charCodeAt(k);
        if (d === 58) {
          emit(descr ? T.acc_descr : T.acc_title, p, run(k + 1));
          push(descr ? S.acc_descr : S.acc_title);
          continue;
        }
        if (d === 123 && descr) {
          p = run(k + 1);
          push(S.acc_descr_multiline);
          continue;
        }
      }
    }

    // Axis, quadrant and chart keywords and the axis arrow take the spaces around them.
    let j = p;
    if (c === 32) {
      if (p < spaceFrom || p >= spaceTo) {
        spaceFrom = p;
        spaceTo = spaces(p);
      }
      j = spaceTo;
    }
    const d = src.charCodeAt(j);
    const dl = d | 32;
    let type = 0;
    let end = 0;
    if (dl === 120 && wordAt(src, j, 'x-axis')) {
      type = T.X_AXIS;
      end = j + 6;
    } else if (dl === 121 && wordAt(src, j, 'y-axis')) {
      type = T.Y_AXIS;
      end = j + 6;
    } else if (d === 45) {
      if (j < dashFrom || j >= dashTo) {
        dashFrom = j;
        dashTo = j;
        while (src.charCodeAt(dashTo) === 45) dashTo++;
      }
      if (dashTo - j >= 2 && src.charCodeAt(dashTo) === 62) {
        type = T.DELIMITER;
        end = dashTo + 1;
      }
    } else if (dl === 113 && wordAt(src, j, 'quadrant')) {
      const q = src.charCodeAt(j + 9);
      if (src.charCodeAt(j + 8) === 45 && q >= 49 && q <= 52) {
        type = T.QUADRANT_1 + q - 49;
        end = j + 10;
      } else if (wordAt(src, j + 8, 'chart')) {
        type = T.QUADRANT;
        end = j + 13;
      }
    }
    if (type !== 0) {
      emit(type, p, spaces(end));
      continue;
    }

    if (lower === 99 && wordAt(src, p, 'classdef') && !isWord(src.charCodeAt(p + 8))) {
      emit(T.CLASSDEF, p, p + 8);
      continue;
    }
    if (c === 34) {
      if (next === 96) {
        p += 2;
        push(S.md_string);
      } else {
        p++;
        push(S.string);
      }
      continue;
    }
    if (c === 58 && next === 58 && src.charCodeAt(p + 2) === 58) {
      p += 3;
      push(S.class_name);
      continue;
    }
    // A point's `: [`, with any whitespace before, between and after, line breaks included.
    if (c === 58 || isSpace(c)) {
      let k = p;
      if (c !== 58) {
        if (p < wsFrom || p >= wsTo) {
          wsFrom = p;
          wsTo = run(p);
        }
        k = wsTo;
      }
      if (src.charCodeAt(k) === 58) {
        if (k + 1 < gapFrom || k + 1 >= gapTo) {
          gapFrom = k + 1;
          gapTo = run(k + 1);
        }
        if (src.charCodeAt(gapTo) === 91) {
          emit(T.point_start, p, run(gapTo + 1));
          push(S.point_start);
          continue;
        }
      }
    }

    if (isLetter(c)) {
      let e = p + 1;
      while (isLetter(src.charCodeAt(e))) e++;
      emit(T.ALPHA, p, e);
      continue;
    }
    if (c >= 128) {
      let e = p + 1;
      while (src.charCodeAt(e) >= 128) e++;
      emit(T.UNICODE_TEXT, p, e);
      continue;
    }
    if (isDigit(c)) {
      let e = p + 1;
      while (isDigit(src.charCodeAt(e))) e++;
      emit(T.NUM, p, e);
      continue;
    }
    if (SINGLE[c] !== 0) {
      emit(SINGLE[c], p, p + 1);
      continue;
    }
    break;
  }

  if (p < n) {
    types.push(T.ERROR);
    texts.push('');
    starts.push(p);
    return { types, texts, starts };
  }
  // Jison tries the rules of the current state once more against the empty rest of the input.
  if (state < AT_END.length) {
    types.push(AT_END[state]);
    texts.push('');
    starts.push(n);
  }
  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
