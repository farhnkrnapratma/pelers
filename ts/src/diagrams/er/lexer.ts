import { T } from './tokens.js';
import { isDigit, isSpace, isWord } from '../../util/chars.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

const enum S {
  INITIAL,
  block,
  block_bq,
  acc_title,
  acc_descr,
  acc_descr_multiline,
  style,
}

const TEXT = 1;
const STYLE_TEXT = 2;
const ATTR_START = 4;
const ATTR_REST = 8;

const CLASS = new Uint8Array(128);
for (let c = 0; c < 128; c++) {
  const word = (c >= 48 && c <= 57) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c === 95;
  const letter = word && (c < 48 || c > 57);
  if (word) CLASS[c] |= TEXT | STYLE_TEXT | ATTR_REST;
  if (letter) CLASS[c] |= ATTR_START;
}
for (const ch of '-*') CLASS[ch.charCodeAt(0)] |= TEXT | STYLE_TEXT | ATTR_REST;
CLASS[46] |= TEXT | ATTR_REST;
CLASS[42] |= ATTR_START;
for (const ch of '[](),') CLASS[ch.charCodeAt(0)] |= ATTR_REST;

const RE_ACC_TITLE = /accTitle\s*:\s*/iy;
const RE_ACC_DESCR = /accDescr\s*:\s*/iy;
const RE_ACC_DESCR_ML = /accDescr\s*\{\s*/iy;
const RE_DIRECTION = [
  /.*direction\s+TB[^\n]*/iy,
  /.*direction\s+BT[^\n]*/iy,
  /.*direction\s+RL[^\n]*/iy,
  /.*direction\s+LR[^\n]*/iy,
];
const RE_DIR_WORD = /direction/gi;
const RE_LINE_END = /[\n\r\u2028\u2029]/g;

// Mermaid's attribute pattern is case-insensitive, which lets the micro sign into its \u00c0-\uffff range.
function isAttr(c: number, kind: number): boolean {
  return c < 128 ? (CLASS[c] & kind) !== 0 : c >= 0xc0 || c === 0xb5;
}

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  let dirNext = -2;
  let dirFailed = 0;
  let lineEnd = -1;
  let tildeNext = -2;
  let tildeLine = -1;
  let tildeLast = -1;
  let wsNext = -1;

  const emit = (type: number, s: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(s, e));
    starts.push(s);
    p = e;
  };
  const at = (re: RegExp, q: number): number => {
    re.lastIndex = q;
    return re.test(src) ? re.lastIndex : -1;
  };
  // Matches a lower-case keyword at p, ignoring case.
  const kw = (word: string): boolean => {
    for (let i = 0; i < word.length; i++) {
      const k = word.charCodeAt(i);
      const c = src.charCodeAt(p + i);
      if (c !== k && (k < 97 || k > 122 || (c | 32) !== k)) return false;
    }
    return true;
  };
  const kwb = (word: string): boolean => kw(word) && !isWord(src.charCodeAt(p + word.length));
  const lower = (q: number): number => {
    const c = src.charCodeAt(q);
    return c >= 65 && c <= 90 ? c + 32 : c;
  };
  const nextLineEnd = (q: number): number => {
    RE_LINE_END.lastIndex = q;
    return RE_LINE_END.test(src) ? RE_LINE_END.lastIndex - 1 : n;
  };
  const digits = (q: number): number => {
    while (isDigit(src.charCodeAt(q))) q++;
    return q;
  };

  // A direction statement swallows its whole line, from wherever the lexer stands on it.
  const direction = (): boolean => {
    if (dirNext === -1) return false;
    if (dirNext < p) {
      RE_DIR_WORD.lastIndex = p;
      dirNext = RE_DIR_WORD.test(src) ? RE_DIR_WORD.lastIndex - 9 : -1;
      if (dirNext === -1) return false;
    }
    if (p < dirFailed) return false;
    if (lineEnd < p) lineEnd = nextLineEnd(p);
    if (dirNext >= lineEnd) return false;
    for (let i = 0; i < 4; i++) {
      const e = at(RE_DIRECTION[i], p);
      if (e >= 0) {
        emit(T.direction_tb + i, p, e);
        return true;
      }
    }
    dirFailed = lineEnd;
    return false;
  };

  const quoted = (): void => {
    const e = src.indexOf('"', p + 1);
    if (e === -1) {
      emit(T.CHAR, p, p + 1);
      return;
    }
    let name = e > p + 1;
    for (let q = p + 1; name && q < e; q++) {
      const c = src.charCodeAt(q);
      if (c === 37 || c === 92 || c === 13 || c === 10 || c === 11 || c === 8) name = false;
    }
    emit(name ? T.ENTITY_NAME : T.WORD, p, e + 1);
  };

  const number = (c: number): void => {
    const d = src.charCodeAt(p + 1);
    if (d === 43 && (c === 49 || c === 48)) {
      emit(c === 49 ? T.ONE_OR_MORE : T.ZERO_OR_MORE, p, p + 2);
      return;
    }
    const q = digits(p + 1);
    if (src.charCodeAt(q) === 46 && isDigit(src.charCodeAt(q + 1))) {
      emit(T.DECIMAL_NUM, p, digits(q + 2));
      return;
    }
    if (c === 49) {
      if (isSpace(d)) {
        let r = p + 2;
        while (isSpace(src.charCodeAt(r))) r++;
        const f = src.charCodeAt(r);
        if (isWord(f) || f === 34 || f === 39) {
          emit(T.ONLY_ONE, p, p + 1);
          return;
        }
      } else if (d === 45 || d === 46) {
        const f = src.charCodeAt(p + 2);
        if (f === 45 || f === 46) {
          emit(T.ONLY_ONE, p, p + 1);
          return;
        }
      }
      if (!isWord(d)) {
        emit(T.ENTITY_ONE, p, p + 1);
        return;
      }
    }
    emit(T.NUM, p, q);
  };

  const keyword = (lc: number): boolean => {
    const d = src.charCodeAt(p + 1);
    switch (lc) {
      case 101:
        if (kwb('erdiagram')) emit(T.ER_DIAGRAM, p, p + 9);
        else if (kwb('end')) {
          let e = p + 3;
          while (isSpace(src.charCodeAt(e))) e++;
          emit(T.END_KW, p, e);
        } else return false;
        return true;
      case 115:
        if (kwb('style')) {
          state = S.style;
          emit(T.STYLE, p, p + 5);
        } else if (kwb('subgraph')) emit(T.SUBGRAPH, p, p + 8);
        else return false;
        return true;
      case 99:
        if (kwb('classdef')) {
          state = S.style;
          emit(T.CLASSDEF, p, p + 8);
        } else if (kwb('class')) emit(T.CLASS, p, p + 5);
        else return false;
        return true;
      case 111:
        if (kwb('one or zero')) emit(T.ZERO_OR_ONE, p, p + 11);
        else if (kwb('one or more') || kwb('one or many')) emit(T.ONE_OR_MORE, p, p + 11);
        else if (kwb('one')) emit(T.ONLY_ONE, p, p + 3);
        else if (kwb('only one')) emit(T.ONLY_ONE, p, p + 8);
        else if (d === 124) emit(T.ZERO_OR_ONE, p, p + 2);
        else if (d === 123) emit(T.ZERO_OR_MORE, p, p + 2);
        else if (kwb('optionally to')) emit(T.NON_IDENTIFYING, p, p + 13);
        else return false;
        return true;
      case 122:
        if (kwb('zero or one')) emit(T.ZERO_OR_ONE, p, p + 11);
        else if (kwb('zero or more') || kwb('zero or many')) emit(T.ZERO_OR_MORE, p, p + 12);
        else return false;
        return true;
      case 109:
        if (kw('many(0)')) emit(T.ZERO_OR_MORE, p, p + 7);
        else if (kw('many(1)')) emit(T.ONE_OR_MORE, p, p + 7);
        else if (kwb('many')) emit(T.ZERO_OR_MORE, p, p + 4);
        else return false;
        return true;
      case 116:
        if (!kwb('to')) return false;
        emit(T.IDENTIFYING, p, p + 2);
        return true;
      case 117:
        if (d !== 46 && d !== 45 && d !== 124) return false;
        emit(T.MD_PARENT, p, p + 1);
        return true;
      case 124:
        if (lower(p + 1) === 111 && !isWord(src.charCodeAt(p + 2))) emit(T.ZERO_OR_ONE, p, p + 2);
        else if (d === 124) emit(T.ONLY_ONE, p, p + 2);
        else if (d === 123) emit(T.ONE_OR_MORE, p, p + 2);
        else return false;
        return true;
      case 125:
        if (lower(p + 1) === 111 && !isWord(src.charCodeAt(p + 2))) emit(T.ZERO_OR_MORE, p, p + 2);
        else if (d === 124) emit(T.ONE_OR_MORE, p, p + 2);
        else return false;
        return true;
      case 46:
        if (d !== 46 && d !== 45) return false;
        emit(T.NON_IDENTIFYING, p, p + 2);
        return true;
      case 45:
        if (d === 45) emit(T.IDENTIFYING, p, p + 2);
        else if (d === 46) emit(T.NON_IDENTIFYING, p, p + 2);
        else return false;
        return true;
    }
    return false;
  };

  const initial = (c: number): void => {
    let e: number;
    if ((c === 97 || c === 65) && kw('acc')) {
      if ((e = at(RE_ACC_TITLE, p)) >= 0) {
        state = S.acc_title;
        emit(T.acc_title, p, e);
        return;
      }
      if ((e = at(RE_ACC_DESCR, p)) >= 0) {
        state = S.acc_descr;
        emit(T.acc_descr, p, e);
        return;
      }
      if ((e = at(RE_ACC_DESCR_ML, p)) >= 0) {
        state = S.acc_descr_multiline;
        p = e;
        return;
      }
    }
    if (direction()) return;

    switch (c) {
      case 32:
      case 9:
      case 13:
        e = p + 1;
        for (let d = src.charCodeAt(e); d === 32 || d === 9 || d === 13; d = src.charCodeAt(e)) e++;
        p = e;
        return;
      case 10:
        e = p + 1;
        while (src.charCodeAt(e) === 10) e++;
        emit(T.NEWLINE, p, e);
        return;
      case 34:
        quoted();
        return;
      case 123:
        state = S.block;
        emit(T.BLOCK_START, p, p + 1);
        return;
      case 35:
        emit(T.BRKT, p, p + 1);
        return;
      case 44:
        emit(T.COMMA, p, p + 1);
        return;
      case 58:
        if (src.charCodeAt(p + 1) === 58 && src.charCodeAt(p + 2) === 58) emit(T.STYLE_SEPARATOR, p, p + 3);
        else emit(T.COLON, p, p + 1);
        return;
      case 91:
        emit(T.SQS, p, p + 1);
        return;
      case 93:
        emit(T.SQE, p, p + 1);
        return;
    }
    if (isDigit(c)) {
      number(c);
      return;
    }
    if (keyword(c >= 65 && c <= 90 ? c + 32 : c)) return;
    if (c >= 128 || (CLASS[c] & TEXT) !== 0) {
      e = p + 1;
      for (let d = src.charCodeAt(e); d >= 128 || (CLASS[d] & TEXT) !== 0; d = src.charCodeAt(e)) e++;
      emit(T.UNICODE_TEXT, p, e);
      return;
    }
    emit(T.CHAR, p, p + 1);
  };

  // Mermaid's generic type rule: a run of non-space characters holding a `~`, through the last `~` on the line.
  const generic = (): boolean => {
    if (tildeNext === -1) return false;
    if (tildeNext < p) {
      tildeNext = src.indexOf('~', p);
      if (tildeNext === -1) return false;
    }
    if (wsNext < p) {
      let q = p;
      while (q < n && !isSpace(src.charCodeAt(q))) q++;
      wsNext = q;
    }
    if (tildeNext >= wsNext) return false;
    if (tildeLine < p) {
      tildeLine = nextLineEnd(p);
      tildeLast = src.lastIndexOf('~', tildeLine - 1);
    }
    if (tildeLast <= tildeNext) return false;
    let e = tildeLast + 1;
    while (e < n && !isSpace(src.charCodeAt(e))) e++;
    emit(T.ATTRIBUTE_WORD, p, e);
    return true;
  };

  const block = (c: number): void => {
    if (isSpace(c)) {
      p++;
      while (isSpace(src.charCodeAt(p))) p++;
      return;
    }
    const lc = c | 32;
    if (
      (lc === 112 || lc === 102 || lc === 117) &&
      (src.charCodeAt(p + 1) | 32) === 107 &&
      !isWord(src.charCodeAt(p + 2))
    ) {
      emit(T.ATTRIBUTE_KEY, p, p + 2);
      return;
    }
    if (generic()) return;
    if (isAttr(c, ATTR_START)) {
      let e = p + 1;
      while (e < n && isAttr(src.charCodeAt(e), ATTR_REST)) e++;
      emit(T.ATTRIBUTE_WORD, p, e);
      return;
    }
    if (c === 96) {
      state = S.block_bq;
      p++;
      return;
    }
    if (c === 34) {
      const e = src.indexOf('"', p + 1);
      if (e !== -1) {
        emit(T.COMMENT, p, e + 1);
        return;
      }
    } else if (c === 125) {
      state = S.INITIAL;
      emit(T.BLOCK_STOP, p, p + 1);
      return;
    }
    emit(T.CHAR, p, p + 1);
  };

  while (p < n) {
    const c = src.charCodeAt(p);
    let e: number;
    switch (state) {
      case S.INITIAL:
        initial(c);
        break;

      case S.block:
        block(c);
        break;

      case S.block_bq:
        if (c === 96) {
          state = S.block;
          p++;
        } else {
          e = src.indexOf('`', p);
          emit(T.ATTRIBUTE_WORD, p, e === -1 ? n : e);
        }
        break;

      case S.acc_title:
      case S.acc_descr:
        e = src.indexOf('\n', p);
        emit(state === S.acc_title ? T.acc_title_value : T.acc_descr_value, p, e === -1 ? n : e);
        state = S.INITIAL;
        break;

      case S.acc_descr_multiline:
        if (c === 125) {
          state = S.INITIAL;
          p++;
        } else {
          e = src.indexOf('}', p);
          emit(T.acc_descr_multiline_value, p, e === -1 ? n : e);
        }
        break;

      case S.style:
        if (c === 10) {
          e = p + 1;
          while (src.charCodeAt(e) === 10) e++;
          state = S.INITIAL;
          emit(T.NEWLINE, p, e);
        } else if (isSpace(c)) {
          p++;
          while (isSpace(src.charCodeAt(p))) p++;
        } else if (c === 58) emit(T.COLON, p, p + 1);
        else if (c === 44) emit(T.COMMA, p, p + 1);
        else if (c === 35) emit(T.BRKT, p, p + 1);
        else if (c === 59) emit(T.SEMI, p, p + 1);
        else if (c >= 128 || (CLASS[c] & STYLE_TEXT) !== 0) {
          e = p + 1;
          for (let d = src.charCodeAt(e); d >= 128 || (CLASS[d] & STYLE_TEXT) !== 0; d = src.charCodeAt(e)) e++;
          emit(T.STYLE_TEXT, p, e);
        } else {
          types.push(T.ERROR);
          texts.push('');
          starts.push(p);
          return { types, texts, starts };
        }
        break;
    }
  }

  // At the end of input Jison's INITIAL state yields EOF, the title and description states yield
  // an empty value, and the block and style states yield nothing.
  const last =
    state === S.INITIAL
      ? T.EOF
      : state === S.acc_title
        ? T.acc_title_value
        : state === S.acc_descr
          ? T.acc_descr_value
          : state === S.acc_descr_multiline
            ? T.acc_descr_multiline_value
            : -1;
  if (last !== -1) {
    types.push(last);
    texts.push('');
    starts.push(n);
  }
  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
