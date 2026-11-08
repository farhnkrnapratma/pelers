import { isSpace, isWord } from '../../util/chars.js';
import { accessibility, keywordAt, lineEnd, skipTrivia, type Tokens } from '../common/scan.js';

export const enum T {
  END,
  EOF,
  INVALID,
  NEWLINE,
  timeline,
  timeline_lr,
  timeline_td,
  title,
  acc_title,
  acc_title_value,
  acc_descr,
  acc_descr_value,
  acc_descr_multiline_value,
  section,
  period,
  event,
}

export const TOKEN_NAMES = [
  '$end',
  'EOF',
  'INVALID',
  'NEWLINE',
  'timeline',
  'timeline_lr',
  'timeline_td',
  'title',
  'acc_title',
  'acc_title_value',
  'acc_descr',
  'acc_descr_value',
  'acc_descr_multiline_value',
  'section',
  'period',
  'event',
];

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
    if (keywordAt(src, p, 'timeline')) {
      e = p + 8;
      while (src.charCodeAt(e) === 32 || src.charCodeAt(e) === 9) e++;
      const lr = keywordAt(src, e, 'lr');
      if (e > p + 8 && (lr || keywordAt(src, e, 'td')) && !isWord(src.charCodeAt(e + 2))) {
        emit(lr ? T.timeline_lr : T.timeline_td, e + 2);
        continue;
      }
      if (!isWord(src.charCodeAt(p + 8))) {
        emit(T.timeline, p + 8);
        continue;
      }
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
        emit(T.title, lineEnd(src, p + 6));
        continue;
      }
    }
    e = accessibility(src, p, out, T.acc_title);
    if (e !== p) {
      p = e;
      continue;
    }
    // Like Mermaid, a bare section uses the next line as its name.
    if (keywordAt(src, p, 'section') && isSpace(src.charCodeAt(p + 7))) {
      e = p + 8;
      while (e < n && src.charCodeAt(e) !== 58 && src.charCodeAt(e) !== 10) e++;
      if (e > p + 8) {
        emit(T.section, e);
        continue;
      }
    }
    if (c !== 58) {
      e = p + 1;
      for (let k = src.charCodeAt(e); e < n && k !== 35 && k !== 58 && k !== 10; k = src.charCodeAt(++e));
      emit(T.period, e);
      continue;
    }
    // An event runs to the end of the line or to a colon that is followed by whitespace.
    if (isSpace(src.charCodeAt(p + 1))) {
      e = p + 2;
      for (; e < n; e++) {
        const k = src.charCodeAt(e);
        if (k === 10 || (k === 58 && isSpace(src.charCodeAt(e + 1)))) break;
      }
      if (e > p + 2) {
        emit(T.event, e);
        continue;
      }
    }
    emit(T.INVALID, p + 1);
  }

  // Jison marks its lexer done when a rule runs on empty input, so an accessibility value that
  // matches nothing at the very end of the text is followed by no EOF token.
  if (starts.length === 0 || starts[starts.length - 1] < n) emit(T.EOF, n);
  emit(T.END, n);
  return out;
}
