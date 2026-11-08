import { isDigit, isLetter, isSpace, isWord } from '../../util/chars.js';
import { wordAt } from '../common/caseless.js';
import { T } from './tokens.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

const enum S {
  INITIAL,
  axis_data,
  axis_band_data,
  data,
  data_inner,
  string,
  acc_title,
  acc_descr,
  acc_descr_multiline,
}

// The token each state gives for the empty rest of the input.
const AT_END = [
  T.EOF,
  T.EOF,
  T.EOF,
  T.EOF,
  T.EOF,
  T.STR,
  T.acc_title_value,
  T.acc_descr_value,
  T.acc_descr_multiline_value,
];

// Mermaid's grammar means to open a markdown string at `"` followed by a backtick, but its quoting turns
// the rule into one that matches this text and leaves the current state. Markdown strings are never read.
const BROKEN_RULE =
  '`)' +
  ' '.repeat(36) +
  '{ this.pushstate(md_string); }\n<md_string>(?:(?!`").)+' +
  ' '.repeat(18) +
  '{ return md_str; }\n<md_string>(?:`';

const SINGLE = new Uint8Array(128);
for (const [char, type] of [
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
  [';', T.SEMI],
] as const) {
  SINGLE[char.charCodeAt(0)] = type;
}

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;

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
  const run = (q: number): number => {
    while (q < n && isSpace(src.charCodeAt(q))) q++;
    return q;
  };
  const digits = (q: number): number => {
    while (isDigit(src.charCodeAt(q))) q++;
    return q;
  };
  const keyword = (word: string): boolean => wordAt(src, p, word) && !isWord(src.charCodeAt(p + word.length));

  while (p < n) {
    const c = src.charCodeAt(p);

    if (state === S.string) {
      if (c === 34) {
        p++;
        pop();
      } else {
        const e = src.indexOf('"', p);
        emit(T.STR, p, e === -1 ? n : e);
      }
      continue;
    }
    if (state === S.acc_title || state === S.acc_descr) {
      emit(AT_END[state], p, lineEnd(p));
      pop();
      continue;
    }
    if (state === S.acc_descr_multiline) {
      if (c === 125) {
        p++;
        pop();
      } else {
        const e = src.indexOf('}', p);
        emit(T.acc_descr_multiline_value, p, e === -1 ? n : e);
      }
      continue;
    }

    const next = src.charCodeAt(p + 1);
    if (c === 37 && next === 37) {
      p = lineEnd(p + 2);
      continue;
    }
    if (c === 10 || c === 13) {
      // Only a line break that directly follows an axis or a series ends it; one after spaces is skipped
      // with them, and the state carries on to the next line.
      if ((state === S.axis_data || state === S.data) && (c === 10 || next === 10)) {
        emit(T.NEWLINE, p, c === 10 ? p + 1 : p + 2);
        pop();
        continue;
      }
      let e = p + 1;
      for (let d = src.charCodeAt(e); d === 10 || d === 13; d = src.charCodeAt(++e));
      emit(T.NEWLINE, p, e);
      continue;
    }

    switch (c | 32) {
      case 116:
        if (keyword('title')) {
          emit(T.title, p, p + 5);
          continue;
        }
        break;
      case 97:
        if (wordAt(src, p, 'acc')) {
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
        break;
      case 120:
        if (keyword('xychart-beta')) {
          emit(T.XYCHART, p, p + 12);
          continue;
        }
        if (keyword('xychart')) {
          emit(T.XYCHART, p, p + 7);
          continue;
        }
        if (keyword('x-axis')) {
          emit(T.X_AXIS, p, p + 6);
          push(S.axis_data);
          continue;
        }
        break;
      case 121:
        if (keyword('y-axis')) {
          emit(T.Y_AXIS, p, p + 6);
          push(S.axis_data);
          continue;
        }
        break;
      case 118:
        if (wordAt(src, p, 'vertical')) {
          emit(T.CHART_ORIENTATION, p, p + 8);
          continue;
        }
        break;
      case 104:
        if (wordAt(src, p, 'horizontal')) {
          emit(T.CHART_ORIENTATION, p, p + 10);
          continue;
        }
        break;
      case 108:
        if (keyword('line')) {
          emit(T.LINE, p, p + 4);
          push(S.data);
          continue;
        }
        break;
      case 98:
        if (keyword('bar')) {
          emit(T.BAR, p, p + 3);
          push(S.data);
          continue;
        }
        break;
    }

    if (c === 91) {
      if (state === S.axis_data) push(S.axis_band_data);
      else if (state === S.data) push(S.data_inner);
      emit(T.SQUARE_BRACES_START, p, p + 1);
      continue;
    }
    if (c === 93) {
      if (state === S.data_inner || state === S.axis_band_data) pop();
      emit(T.SQUARE_BRACES_END, p, p + 1);
      continue;
    }
    if (state === S.axis_data && c === 45 && next === 45 && src.charCodeAt(p + 2) === 62) {
      emit(T.ARROW_DELIMITER, p, p + 3);
      continue;
    }
    if (state === S.axis_data || state === S.data_inner) {
      const q = c === 43 || c === 45 ? p + 1 : p;
      const d = src.charCodeAt(q);
      let e = -1;
      if (isDigit(d)) {
        e = digits(q + 1);
        if (src.charCodeAt(e) === 46 && isDigit(src.charCodeAt(e + 1))) e = digits(e + 2);
      } else if (d === 46 && isDigit(src.charCodeAt(q + 1))) {
        e = digits(q + 2);
      }
      if (e !== -1) {
        emit(T.NUMBER_WITH_DECIMAL, p, e);
        continue;
      }
    }
    if (c === 96 && wordAt(src, p, BROKEN_RULE)) {
      p += BROKEN_RULE.length;
      pop();
      continue;
    }
    if (c === 34) {
      p++;
      push(S.string);
      continue;
    }
    if (isLetter(c)) {
      let e = p + 1;
      while (isLetter(src.charCodeAt(e))) e++;
      emit(T.ALPHA, p, e);
      continue;
    }
    if (isDigit(c)) {
      emit(T.NUM, p, digits(p + 1));
      continue;
    }
    if (isSpace(c)) {
      p = run(p + 1);
      continue;
    }
    if (c < 128 && SINGLE[c] !== 0) {
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
  types.push(AT_END[state]);
  texts.push('');
  starts.push(n);
  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
