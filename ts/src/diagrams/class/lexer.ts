import { isUnicodeLetter } from '../../util/unicode.js';
import { T } from './tokens.js';
import { isSpace, isWord } from '../../util/chars.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

const enum S {
  INITIAL,
  string,
  bqstring,
  generic,
  callbackName,
  callbackArgs,
  accTitle,
  accDescr,
  accDescrMultiline,
  cls,
  classBody,
  namespace,
  namespaceBody,
}

// The token each state gives at the end of input, before the parser sees the end itself.
const AT_END = [
  T.EOF,
  T.STR,
  T.EOF,
  T.GENERICTYPE,
  T.CALLBACK_NAME,
  T.CALLBACK_ARGS,
  T.acc_title_value,
  T.acc_descr_value,
  T.acc_descr_multiline_value,
  T.EOF,
  T.EOF_IN_STRUCT,
  T.EOF,
  T.EOF_IN_STRUCT,
];

const RE_DIRECTION = [
  /.*direction\s+TB[^\n]*/y,
  /.*direction\s+BT[^\n]*/y,
  /.*direction\s+RL[^\n]*/y,
  /.*direction\s+LR[^\n]*/y,
];
const RE_LINE_END = /[\n\r\u2028\u2029]/g;

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  let dirNext = src.indexOf('direction');
  let lineEnd = -1;
  let dirFailed = 0;

  const emit = (type: number, s: number, e: number): true => {
    types.push(type);
    texts.push(src.slice(s, e));
    starts.push(s);
    p = e;
    return true;
  };
  const push = (s: number): void => {
    stack.push(state);
    state = s;
  };
  const pop = (): void => {
    if (stack.length > 0) state = stack.pop()!;
  };
  const kw = (word: string): boolean => src.startsWith(word, p) && !isWord(src.charCodeAt(p + word.length));
  const skipWs = (q: number): number => {
    while (q < n && isSpace(src.charCodeAt(q))) q++;
    return q;
  };
  const until = (ch: string): number => {
    const e = src.indexOf(ch, p);
    return e === -1 ? n : e;
  };

  // A run of whitespace is a NEWLINE up to its last line feed; the rest is skipped.
  const space = (leaves: boolean): void => {
    let q = p;
    let k = -1;
    for (let c = src.charCodeAt(q); q < n && isSpace(c); c = src.charCodeAt(++q)) if (c === 10) k = q;
    if (k < 0) {
      p = q;
      return;
    }
    if (leaves) pop();
    emit(T.NEWLINE, p, k + 1);
  };

  // The keywords that every state recognizes, even inside generics and backtick names.
  const keyword = (c: number): boolean => {
    switch (c) {
      case 99:
        if (kw('cssClass')) return emit(T.CSSCLASS, p, p + 8);
        if (kw('callback')) return emit(T.CALLBACK, p, p + 8);
        if (kw('click')) return emit(T.CLICK, p, p + 5);
        break;
      case 108:
        if (kw('link')) return emit(T.LINK, p, p + 4);
        break;
      case 110:
        if (kw('note for')) return emit(T.NOTE_FOR, p, p + 8);
        if (kw('note')) return emit(T.NOTE, p, p + 4);
        break;
      case 60:
        if (src.charCodeAt(p + 1) === 60) return emit(T.ANNOTATION_START, p, p + 2);
        break;
      case 62:
        if (src.charCodeAt(p + 1) === 62) return emit(T.ANNOTATION_END, p, p + 2);
        break;
      case 104:
        if (kw('href')) return emit(T.HREF, p, p + 4);
        break;
    }
    return false;
  };

  const common = (c: number): boolean => {
    if (keyword(c)) return true;
    const d = src.charCodeAt(p + 1);
    switch (c) {
      case 34:
        push(S.string);
        p++;
        return true;
      case 126:
        push(S.generic);
        p++;
        return true;
      case 96:
        push(S.bqstring);
        p++;
        return true;
      case 95:
        if (kw('_self')) return emit(T.LINK_TARGET, p, p + 5);
        if (kw('_blank')) return emit(T.LINK_TARGET, p, p + 6);
        if (kw('_parent')) return emit(T.LINK_TARGET, p, p + 7);
        if (kw('_top')) return emit(T.LINK_TARGET, p, p + 4);
        break;
      case 60:
        return d === 124 ? emit(T.EXTENSION, p, p + 2) : emit(T.DEPENDENCY, p, p + 1);
      case 124:
        return d === 62 && emit(T.EXTENSION, p, p + 2);
      case 62:
        return emit(T.DEPENDENCY, p, p + 1);
      case 42:
        return emit(T.COMPOSITION, p, p + 1);
      case 111:
        if (!isWord(d)) return emit(T.AGGREGATION, p, p + 1);
        break;
      case 40:
        return d === 41 && emit(T.LOLLIPOP, p, p + 2);
      case 45:
        return d === 45 ? emit(T.LINE, p, p + 2) : emit(T.MINUS, p, p + 1);
      case 46:
        return d === 46 ? emit(T.DOTTED_LINE, p, p + 2) : emit(T.DOT, p, p + 1);
      case 58: {
        let q = p + 1;
        while (q < n) {
          const ch = src.charCodeAt(q);
          if (ch === 58 || ch === 10 || ch === 59) break;
          q++;
        }
        if (q > p + 1) return emit(T.LABEL, p, q);
        return d === 58 && src.charCodeAt(p + 2) === 58 && emit(T.STYLE_SEPARATOR, p, p + 3);
      }
      case 43:
        return emit(T.PLUS, p, p + 1);
      case 37:
        return emit(T.PCT, p, p + 1);
      case 61:
        return emit(T.EQUALS, p, p + 1);
      case 91:
        return emit(T.SQS, p, p + 1);
      case 93:
        return emit(T.SQE, p, p + 1);
      case 33:
      case 35:
      case 36:
      case 38:
      case 39:
      case 44:
      case 63:
      case 92:
      case 47:
        return emit(T.PUNCTUATION, p, p + 1);
    }
    if (isWord(c)) {
      let q = p + 1;
      while (isWord(src.charCodeAt(q))) q++;
      return emit(T.ALPHA, p, q);
    }
    return isUnicodeLetter(c) && emit(T.UNICODE_TEXT, p, p + 1);
  };

  const direction = (): boolean => {
    if (dirNext === -1 || p < dirFailed) return false;
    if (dirNext < p) dirNext = src.indexOf('direction', p);
    if (dirNext === -1) return false;
    if (lineEnd < p) {
      RE_LINE_END.lastIndex = p;
      lineEnd = RE_LINE_END.test(src) ? RE_LINE_END.lastIndex - 1 : n;
    }
    if (dirNext >= lineEnd) return false;
    for (let i = 0; i < 4; i++) {
      const re = RE_DIRECTION[i];
      re.lastIndex = p;
      if (re.test(src)) return emit(T.direction_tb + i, p, re.lastIndex);
    }
    // No direction statement starts anywhere in the rest of this line either.
    dirFailed = lineEnd;
    return false;
  };

  const initial = (c: number): boolean => {
    if (direction()) return true;
    if (isSpace(c)) {
      space(false);
      return true;
    }
    switch (c) {
      case 37:
        if (src.charCodeAt(p + 1) === 37) {
          let e = until('\n');
          while (e < n && (src.charCodeAt(e) === 10 || src.charCodeAt(e) === 13)) e++;
          p = e;
          return true;
        }
        break;
      case 97: {
        const title = src.startsWith('accTitle', p);
        if (!title && !src.startsWith('accDescr', p)) break;
        const q = skipWs(p + 8);
        const d = src.charCodeAt(q);
        if (d === 58) {
          emit(title ? T.acc_title : T.acc_descr, p, skipWs(q + 1));
          push(title ? S.accTitle : S.accDescr);
          return true;
        }
        if (d === 123 && !title) {
          p = skipWs(q + 1);
          push(S.accDescrMultiline);
          return true;
        }
        break;
      }
      case 99:
        if (kw('classDiagram-v2')) return emit(T.CLASS_DIAGRAM, p, p + 15);
        if (kw('classDiagram')) return emit(T.CLASS_DIAGRAM, p, p + 12);
        if (src.startsWith('call', p) && isSpace(src.charCodeAt(p + 4))) {
          p = skipWs(p + 4);
          push(S.callbackName);
          return true;
        }
        if (kw('classDef')) return emit(T.CLASSDEF, p, p + 8);
        if (kw('class')) {
          push(S.cls);
          return emit(T.CLASS, p, p + 5);
        }
        break;
      case 91:
        if (src.startsWith('*]', p + 1)) return emit(T.EDGE_STATE, p, p + 3);
        break;
      case 115:
        if (kw('style')) return emit(T.STYLE, p, p + 5);
        break;
      case 110:
        if (kw('namespace')) {
          push(S.namespace);
          return emit(T.NAMESPACE, p, p + 9);
        }
        break;
      case 44:
        return emit(T.COMMA, p, p + 1);
      case 35:
        return emit(T.BRKT, p, p + 1);
    }
    return common(c) || (c === 58 && emit(T.COLON, p, p + 1));
  };

  while (true) {
    if (p >= n) {
      types.push(AT_END[state]);
      texts.push('');
      starts.push(n);
      break;
    }
    const c = src.charCodeAt(p);
    let ok = true;
    switch (state) {
      case S.INITIAL:
        ok = initial(c);
        break;

      case S.cls:
        if (isSpace(c)) {
          space(true);
        } else if (c === 125) {
          pop();
          pop();
          emit(T.STRUCT_STOP, p, p + 1);
        } else if (c === 123) {
          push(S.classBody);
          emit(T.STRUCT_START, p, p + 1);
        } else {
          ok = common(c);
        }
        break;

      case S.classBody:
        if (c === 34) {
          push(S.string);
          p++;
        } else if (c === 125) {
          pop();
          emit(T.STRUCT_STOP, p, p + 1);
        } else if (c === 91 && src.startsWith('*]', p + 1)) {
          emit(T.EDGE_STATE, p, p + 3);
        } else if (c === 123) {
          emit(T.OPEN_IN_STRUCT, p, p + 1);
        } else if (c === 10) {
          p++;
        } else {
          let q = p + 1;
          while (q < n) {
            const ch = src.charCodeAt(q);
            if (ch === 123 || ch === 125 || ch === 10) break;
            q++;
          }
          emit(T.MEMBER, p, q);
        }
        break;

      case S.namespace:
        if (isSpace(c)) {
          space(true);
        } else if (c === 123) {
          push(S.namespaceBody);
          emit(T.STRUCT_START, p, p + 1);
        } else if (c === 125) {
          // The brace is read again in the state the namespace was opened from.
          pop();
        } else if (c === 110 && kw('namespace')) {
          push(S.namespace);
          emit(T.NAMESPACE, p, p + 9);
        } else {
          ok = common(c);
        }
        break;

      case S.namespaceBody:
        if (isSpace(c)) {
          space(false);
        } else if (c === 125) {
          pop();
          emit(T.STRUCT_STOP, p, p + 1);
        } else if (c === 110 && kw('namespace')) {
          push(S.namespace);
          emit(T.NAMESPACE, p, p + 9);
        } else if (c === 91 && src.startsWith('*]', p + 1)) {
          emit(T.EDGE_STATE, p, p + 3);
        } else if (c === 99 && kw('class')) {
          push(S.cls);
          emit(T.CLASS, p, p + 5);
        } else {
          ok = common(c);
        }
        break;

      case S.string:
        if (c === 34) {
          pop();
          p++;
        } else {
          emit(T.STR, p, until('"'));
        }
        break;

      case S.generic:
        if (c === 34) {
          push(S.string);
          p++;
        } else if (keyword(c)) {
          break;
        } else if (c === 126) {
          pop();
          p++;
        } else {
          emit(T.GENERICTYPE, p, until('~'));
        }
        break;

      case S.bqstring:
        if (c === 34) {
          push(S.string);
          p++;
        } else if (keyword(c)) {
          break;
        } else if (c === 126) {
          push(S.generic);
          p++;
        } else if (c === 96) {
          pop();
          p++;
        } else {
          emit(T.BQUOTE_STR, p, until('`'));
        }
        break;

      case S.callbackName:
        if (c === 40) {
          const q = skipWs(p + 1);
          pop();
          if (src.charCodeAt(q) === 41) {
            p = q + 1;
          } else {
            push(S.callbackArgs);
            p++;
          }
        } else {
          emit(T.CALLBACK_NAME, p, until('('));
        }
        break;

      case S.callbackArgs:
        if (c === 41) {
          pop();
          p++;
        } else {
          emit(T.CALLBACK_ARGS, p, until(')'));
        }
        break;

      case S.accTitle:
      case S.accDescr:
        emit(state === S.accTitle ? T.acc_title_value : T.acc_descr_value, p, until('\n'));
        pop();
        break;

      case S.accDescrMultiline:
        if (c === 125) {
          pop();
          p++;
        } else {
          emit(T.acc_descr_multiline_value, p, until('}'));
        }
        break;
    }

    if (!ok) {
      types.push(T.ERROR);
      texts.push('');
      starts.push(p);
      return { types, texts, starts };
    }
  }

  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
