import { isSpace, isWord } from '../../util/chars.js';
import { accessibility, keywordAt, skipTrivia, type Tokens } from '../common/scan.js';

export const enum T {
  END,
  EOF,
  INVALID,
  NEWLINE,
  journey,
  title,
  acc_title,
  acc_title_value,
  acc_descr,
  acc_descr_value,
  acc_descr_multiline_value,
  section,
  taskName,
  taskData,
  colon,
}

export const TOKEN_NAMES = [
  '$end',
  'EOF',
  'INVALID',
  'NEWLINE',
  'journey',
  'title',
  'acc_title',
  'acc_title_value',
  'acc_descr',
  'acc_descr_value',
  'acc_descr_multiline_value',
  'section',
  'taskName',
  'taskData',
  ':',
];

// `#`, a line break, and `;` end every kind of text; a colon also ends a task name or a section.
function textEnd(src: string, from: number, colon: boolean): number {
  const n = src.length;
  let e = from;
  for (; e < n; e++) {
    const c = src.charCodeAt(e);
    if (c === 35 || c === 10 || c === 59 || (colon && c === 58)) break;
  }
  return e;
}

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const out: Tokens = { types, texts, starts };
  let p = 0;

  const emit = (type: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(p, e));
    starts.push(p);
    p = e;
  };

  for (;;) {
    p = skipTrivia(src, p);
    if (p >= n) break;
    const c = src.charCodeAt(p);
    let e = p + 1;
    if (c === 10) {
      while (src.charCodeAt(e) === 10) e++;
      emit(T.NEWLINE, e);
      continue;
    }
    if (keywordAt(src, p, 'journey') && !isWord(src.charCodeAt(p + 7))) {
      emit(T.journey, p + 7);
      continue;
    }
    if (keywordAt(src, p, 'title')) {
      // Unlike Mermaid, ignore bare titles.
      e = p + 5;
      while (src.charCodeAt(e) !== 10 && isSpace(src.charCodeAt(e))) e++;
      if (e === n || src.charCodeAt(e) === 10) {
        p = e;
        continue;
      }
      const sep = src.charCodeAt(p + 5);
      if (sep !== 13 && isSpace(sep)) {
        e = textEnd(src, p + 6, false);
        if (e > p + 6) {
          emit(T.title, e);
          continue;
        }
      }
    }
    e = accessibility(src, p, out, T.acc_title);
    if (e !== p) {
      p = e;
      continue;
    }
    // Like Mermaid, a bare section uses the next line as its name.
    if (keywordAt(src, p, 'section') && isSpace(src.charCodeAt(p + 7))) {
      e = textEnd(src, p + 8, true);
      if (e > p + 8) {
        emit(T.section, e);
        continue;
      }
    }
    if (c === 58) {
      e = textEnd(src, p + 1, false);
      emit(e > p + 1 ? T.taskData : T.colon, e > p + 1 ? e : p + 1);
    } else if (c === 59) {
      emit(T.INVALID, p + 1);
    } else {
      emit(T.taskName, textEnd(src, p + 1, true));
    }
  }

  // Jison marks its lexer done when a rule runs on empty input, so an accessibility value that
  // matches nothing at the very end of the text is followed by no EOF token.
  if (starts.length === 0 || starts[starts.length - 1] < n) emit(T.EOF, n);
  emit(T.END, n);
  return out;
}
