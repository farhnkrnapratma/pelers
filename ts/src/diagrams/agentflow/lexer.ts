import { isUnicodeLetter } from '../../util/unicode.js';
import { T } from './tokens.js';
import { isSpace, isWord } from '../../util/chars.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
  // Four numbers per token, counted the way Jison counts them:
  // first line, first column, last line, last column. Lines start at 1, columns at 0.
  locs: number[];
}

const enum S {
  INITIAL,
  string,
  md_string,
  acc_title,
  acc_descr,
  acc_descr_multiline,
  dir,
  text,
  ellipseText,
  trapText,
  edgeText,
  click,
  callbackname,
  callbackargs,
  shapeData,
  shapeDataStr,
}

const NODE_CHAR = new Uint8Array(128);
for (const ch of 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!"#$%&\'*+.`?\\_/') {
  NODE_CHAR[ch.charCodeAt(0)] = 1;
}

const RE_ACC_TITLE = /accTitle\s*:\s*/y;
const RE_ACC_DESCR = /accDescr\s*:\s*/y;
const RE_ACC_DESCR_ML = /accDescr\s*\{\s*/y;
const RE_CALL = /call\s+/y;
const RE_CLICK = /click\s+/y;
const RE_NODIR = /(?:\r?\n)*\s*\n/y;
const RE_DIR = /\s*(?:LR|RL|TB|BT|TD|BR)\b/y;
const RE_DIR_SYM = /\s*[<>^]/y;
const RE_DIR_V = /\s*v\b/y;
const RE_DIRECTION = [
  /.*direction\s+TB[^\n]*/y,
  /.*direction\s+BT[^\n]*/y,
  /.*direction\s+RL[^\n]*/y,
  /.*direction\s+LR[^\n]*/y,
  /.*direction\s+TD[^\n]*/y,
];
const RE_LINE_END = /[\n\r\u2028\u2029]/g;
const RE_EMPTY_CALL = /\(\s*\)/y;
const RE_BR = /\n\s*/g;

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const locs: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  let line = 1;
  let col = 0;
  let firstGraph = true;
  let dirNext = src.indexOf('direction');
  let atNext = src.indexOf('@');
  let lineEnd = -1;
  let dirFailed = 0;
  let runEnd = 0;
  let runAt = -1;
  let wsFrom = -1;
  let wsTo = -1;
  let blankFrom = -1;
  let blankTo = -1;
  let noLink = -1;
  let noArrow = -1;

  // Moves over one match. Jison counts \r\n, \r and \n as one line break each, and after a
  // break its column stops at the first U+2028 or U+2029.
  const advance = (e: number): void => {
    let brk = -1;
    for (let k = p; k < e; k++) {
      const ch = src.charCodeAt(k);
      if (ch === 10) {
        line++;
        brk = k;
      } else if (ch === 13) {
        if (k + 1 < e && src.charCodeAt(k + 1) === 10) k++;
        line++;
        brk = k;
      }
    }
    if (brk < 0) {
      col += e - p;
    } else {
      let k = brk + 1;
      while (k < e) {
        const ch = src.charCodeAt(k);
        if (ch === 0x2028 || ch === 0x2029) break;
        k++;
      }
      col = k - brk - 1;
    }
    p = e;
  };

  const emit = (type: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(p, e));
    starts.push(p);
    locs.push(line, col);
    advance(e);
    locs.push(line, col);
  };
  // For a token that cannot hold a line break.
  const emitFlat = (type: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(p, e));
    starts.push(p);
    locs.push(line, col);
    col += e - p;
    p = e;
    locs.push(line, col);
  };
  // For a run that Jison matches one character at a time, where \r\n counts as two line breaks.
  const emitChars = (type: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(p, e));
    starts.push(p);
    locs.push(line, col);
    for (let k = p; k < e; k++) {
      const ch = src.charCodeAt(k);
      if (ch === 10 || ch === 13) {
        line++;
        col = 0;
      } else {
        col++;
      }
    }
    p = e;
    locs.push(line, col);
  };
  const emitText = (type: number, text: string, e: number): void => {
    types.push(type);
    texts.push(text);
    starts.push(p);
    locs.push(line, col);
    advance(e);
    locs.push(line, col);
  };
  const push = (s: number): void => {
    stack.push(state);
    state = s;
  };
  const pop = (): void => {
    if (stack.length > 0) state = stack.pop()!;
  };
  const at = (re: RegExp, q: number): number => {
    re.lastIndex = q;
    return re.test(src) ? re.lastIndex : -1;
  };
  const kw = (word: string): boolean => src.startsWith(word, p) && !isWord(src.charCodeAt(p + word.length));

  // End of the run of whitespace starting at q. The last run is kept, so asking again from inside it costs nothing.
  const wsEnd = (q: number): number => {
    if (q >= wsFrom && q <= wsTo) return wsTo;
    let j = q;
    while (j < n && isSpace(src.charCodeAt(j))) j++;
    wsFrom = q;
    wsTo = j;
    return j;
  };
  const blankEnd = (q: number): number => {
    if (q >= blankFrom && q <= blankTo) return blankTo;
    let j = q;
    while (j < n) {
      const ch = src.charCodeAt(j);
      if (ch !== 32 && ch !== 9) break;
      j++;
    }
    blankFrom = q;
    blankTo = j;
    return j;
  };

  // End of `\s*--+[x>]\s*` starting at q, or -1.
  const arrowEnd = (q: number): number => {
    const j = wsEnd(q);
    if (j === noArrow || src.charCodeAt(j) !== 45 || src.charCodeAt(j + 1) !== 45) return -1;
    let k = j + 2;
    while (src.charCodeAt(k) === 45) k++;
    const t = src.charCodeAt(k);
    if (t === 120 || t === 62) return wsEnd(k + 1);
    noArrow = j;
    return -1;
  };

  const link = (): boolean => {
    const j = wsEnd(p);
    if (j === noLink || src.charCodeAt(j) !== 45) return false;
    const d = src.charCodeAt(j + 1);
    let k = j + 2;
    if (d === 45) {
      while (src.charCodeAt(k) === 45) k++;
      const t = src.charCodeAt(k);
      if (t === 120 || t === 62) {
        emit(T.LINK, wsEnd(k + 1));
      } else {
        push(S.edgeText);
        emit(T.START_LINK, wsEnd(j + 2));
      }
      return true;
    }
    if (d === 46) {
      while (src.charCodeAt(k) === 46) k++;
      if (src.charCodeAt(k) === 45) {
        emit(T.LINK, wsEnd(k + 1));
        return true;
      }
    }
    noLink = j;
    return false;
  };

  // `%%` not followed by `{`, through the end of the line.
  const commentEnd = (q: number): number => {
    if (src.charCodeAt(q) !== 37 || src.charCodeAt(q + 1) !== 37 || src.charCodeAt(q + 2) === 123) return -1;
    const e = src.indexOf('\n', q + 2);
    return e === -1 ? n : e;
  };

  const quote = (): void => {
    if (src.charCodeAt(p + 1) === 96) {
      push(S.md_string);
      advance(p + 2);
    } else {
      push(S.string);
      advance(p + 1);
    }
  };

  const opener = (c: number): boolean => {
    const d = src.charCodeAt(p + 1);
    if (c === 40) {
      if (d === 45) {
        push(S.ellipseText);
        emitFlat(T.ELLIPSE_START, p + 2);
      } else if (d === 91) {
        push(S.text);
        emitFlat(T.STADIUMSTART, p + 2);
      } else if (d === 40 && src.charCodeAt(p + 2) === 40) {
        push(S.text);
        emitFlat(T.DOUBLECIRCLESTART, p + 3);
      } else {
        push(S.text);
        emitFlat(T.PS, p + 1);
      }
      return true;
    }
    if (c === 91) {
      if (d === 91) {
        push(S.text);
        emitFlat(T.SUBROUTINESTART, p + 2);
      } else if (d === 40) {
        push(S.text);
        emitFlat(T.CYLINDERSTART, p + 2);
      } else if (d === 47) {
        push(S.trapText);
        emitFlat(T.TRAPSTART, p + 2);
      } else if (d === 92) {
        push(S.trapText);
        emitFlat(T.INVTRAPSTART, p + 2);
      } else {
        push(S.text);
        emitFlat(T.SQS, p + 1);
      }
      return true;
    }
    if (c === 124) {
      push(S.text);
      emitFlat(T.PIPE, p + 1);
      return true;
    }
    if (c === 123) {
      push(S.text);
      emitFlat(T.DIAMOND_START, p + 1);
      return true;
    }
    return false;
  };

  const nodeString = (): boolean => {
    let q = p;
    while (q < n) {
      const ch = src.charCodeAt(q);
      if (ch < 128 && NODE_CHAR[ch] === 1) {
        q++;
      } else if (ch === 45 && q + 1 < n) {
        const d = src.charCodeAt(q + 1);
        if (d === 62 || d === 45 || d === 46) break;
        q++;
      } else {
        break;
      }
    }
    if (q === p) return false;
    emitFlat(T.NODE_STRING, q);
    return true;
  };

  const blanks = (from: number): number => {
    let e = from;
    while (e < n) {
      const ch = src.charCodeAt(e);
      if (ch !== 32 && ch !== 9) break;
      e++;
    }
    return e;
  };

  const initial = (c: number): boolean => {
    let e: number;
    switch (c) {
      case 97:
        if (src.startsWith('acc', p)) {
          if ((e = at(RE_ACC_TITLE, p)) >= 0) {
            push(S.acc_title);
            emit(T.acc_title, e);
            return true;
          }
          if ((e = at(RE_ACC_DESCR, p)) >= 0) {
            push(S.acc_descr);
            emit(T.acc_descr, e);
            return true;
          }
          if ((e = at(RE_ACC_DESCR_ML, p)) >= 0) {
            push(S.acc_descr_multiline);
            advance(e);
            return true;
          }
        }
        if (kw('agentflow-beta')) {
          if (firstGraph) {
            firstGraph = false;
            push(S.dir);
          }
          emitFlat(T.GRAPH, p + 14);
          return true;
        }
        break;
      case 64:
        if (src.charCodeAt(p + 1) === 123) {
          push(S.shapeData);
          emitText(T.SHAPE_DATA, '', p + 2);
          return true;
        }
        break;
      case 99:
        if ((e = at(RE_CALL, p)) >= 0) {
          push(S.callbackname);
          advance(e);
          return true;
        }
        if (kw('classDef')) {
          emitFlat(T.CLASSDEF, p + 8);
          return true;
        }
        if (kw('class')) {
          emitFlat(T.CLASS, p + 5);
          return true;
        }
        if ((e = at(RE_CLICK, p)) >= 0) {
          push(S.click);
          advance(e);
          return true;
        }
        if (kw('connector')) {
          emitFlat(T.connector, p + 9);
          return true;
        }
        break;
      case 34:
        quote();
        return true;
      case 115:
        if (kw('style')) {
          emitFlat(T.STYLE, p + 5);
          return true;
        }
        break;
      case 100:
        if (kw('default')) {
          emitFlat(T.DEFAULT, p + 7);
          return true;
        }
        break;
      case 108:
        if (kw('linkStyle')) {
          emitFlat(T.LINKSTYLE, p + 9);
          return true;
        }
        break;
      case 105:
        if (kw('interpolate')) {
          emitFlat(T.INTERPOLATE, p + 11);
          return true;
        }
        break;
      case 104:
        if (src.startsWith('href', p) && isSpace(src.charCodeAt(p + 4))) {
          emit(T.HREF, p + 5);
          return true;
        }
        break;
      case 102:
        if (kw('flow')) {
          emitFlat(T.flow, p + 4);
          return true;
        }
        break;
      case 103:
        if (kw('global')) {
          emitFlat(T.global, blanks(p + 6));
          return true;
        }
        break;
      case 101:
        if (kw('end')) {
          emitFlat(T.end, blanks(p + 3));
          return true;
        }
        break;
      case 95:
        if (kw('_self')) {
          emitFlat(T.LINK_TARGET, p + 5);
          return true;
        }
        if (kw('_blank')) {
          emitFlat(T.LINK_TARGET, p + 6);
          return true;
        }
        if (kw('_parent')) {
          emitFlat(T.LINK_TARGET, p + 7);
          return true;
        }
        if (kw('_top')) {
          emitFlat(T.LINK_TARGET, p + 4);
          return true;
        }
        break;
    }

    if (dirNext !== -1 && p >= dirFailed) {
      if (dirNext < p) dirNext = src.indexOf('direction', p);
      if (dirNext !== -1) {
        if (lineEnd < p) {
          RE_LINE_END.lastIndex = p;
          lineEnd = RE_LINE_END.test(src) ? RE_LINE_END.lastIndex - 1 : n;
        }
        if (dirNext < lineEnd) {
          for (let i = 0; i < 5; i++) {
            if ((e = at(RE_DIRECTION[i], p)) >= 0) {
              emit(T.direction_tb + i, e);
              return true;
            }
          }
          // No direction statement starts anywhere in the rest of this line either.
          dirFailed = lineEnd;
        }
      }
    }

    // A link id is a run of non-space characters up to its last `@` that is not followed by `{` or `"`.
    if (atNext !== -1 && c !== 34 && !isSpace(c)) {
      if (p >= runEnd) {
        if (atNext < p) atNext = src.indexOf('@', p);
        let q = p;
        while (q < n) {
          const ch = src.charCodeAt(q);
          if (ch === 34 || isSpace(ch)) break;
          q++;
        }
        runEnd = q;
        runAt = -1;
        if (atNext !== -1 && atNext < q) {
          for (let k = Math.min(q, n - 1) - 1; k >= atNext; k--) {
            if (src.charCodeAt(k) !== 64) continue;
            const after = src.charCodeAt(k + 1);
            if (after !== 123 && after !== 34) {
              runAt = k;
              break;
            }
          }
        }
      }
      if (runAt > p) {
        emitFlat(T.LINK_ID, runAt + 1);
        return true;
      }
    }

    if (c >= 48 && c <= 57) {
      let q = p + 1;
      while (q < n) {
        const ch = src.charCodeAt(q);
        if (ch < 48 || ch > 57) break;
        q++;
      }
      emitFlat(T.NUM, q);
      return true;
    }

    switch (c) {
      case 35:
        emitFlat(T.BRKT, p + 1);
        return true;
      case 58:
        if (src.charCodeAt(p + 1) === 58 && src.charCodeAt(p + 2) === 58) emitFlat(T.STYLE_SEPARATOR, p + 3);
        else emitFlat(T.COLON, p + 1);
        return true;
      case 38:
        emitFlat(T.AMP, p + 1);
        return true;
      case 59:
        emitFlat(T.SEMI, p + 1);
        return true;
      case 44:
        emitFlat(T.COMMA, p + 1);
        return true;
      case 42:
        emitFlat(T.MULT, p + 1);
        return true;
      case 60:
        emitFlat(T.TAGSTART, p + 1);
        return true;
      case 45:
        if (link() || nodeString()) return true;
        emitFlat(T.MINUS, p + 1);
        return true;
      case 40:
        return opener(c);
      case 91:
        if (src.charCodeAt(p + 1) === 124) {
          emitFlat(T.VERTEX_WITH_PROPS_START, p + 2);
          return true;
        }
        return opener(c);
      case 124:
      case 123:
        return opener(c);
      case 62:
        push(S.text);
        emitFlat(T.TAGEND, p + 1);
        return true;
      case 94:
        emitFlat(T.UP, p + 1);
        return true;
      case 92:
        if (src.charCodeAt(p + 1) === 124) {
          emitFlat(T.SEP, p + 2);
          return true;
        }
        return nodeString();
      case 118:
        if (!isWord(src.charCodeAt(p + 1))) {
          emitFlat(T.DOWN, p + 1);
          return true;
        }
        return nodeString();
      case 37:
        if ((e = commentEnd(p)) >= 0) {
          emit(T.COMMENT, e);
          return true;
        }
        return nodeString();
    }

    if (isSpace(c)) {
      if (link()) return true;
      if ((c === 32 || c === 9) && (e = commentEnd(blankEnd(p))) >= 0) {
        emit(T.COMMENT, e);
        return true;
      }
      e = p;
      while (true) {
        const ch = src.charCodeAt(e);
        if (ch === 10) e++;
        else if (ch === 13 && src.charCodeAt(e + 1) === 10) e += 2;
        else break;
      }
      if (e > p) emit(T.NEWLINE, e);
      else emit(T.SPACE, p + 1);
      return true;
    }

    if (c < 128) return nodeString();
    if (isUnicodeLetter(c)) {
      emitFlat(T.UNICODE_TEXT, p + 1);
      return true;
    }
    return false;
  };

  lexing: while (true) {
    if (p >= n) {
      let type = -1;
      switch (state) {
        case S.INITIAL:
          type = T.EOF;
          break;
        case S.acc_title:
          type = T.acc_title_value;
          break;
        case S.acc_descr:
          type = T.acc_descr_value;
          break;
        case S.acc_descr_multiline:
          type = T.acc_descr_multiline_value;
          break;
        case S.click:
          type = T.CLICK;
          break;
        case S.callbackname:
          type = T.CALLBACKNAME;
          break;
        case S.callbackargs:
          type = T.CALLBACKARGS;
          break;
      }
      if (type !== -1) emitFlat(type, n);
      break;
    }

    const c = src.charCodeAt(p);
    let ok = true;

    switch (state) {
      case S.INITIAL:
        ok = initial(c);
        break;

      case S.string:
        if (c === 34) {
          if (src.charCodeAt(p + 1) === 96) {
            push(S.md_string);
            advance(p + 2);
          } else {
            pop();
            advance(p + 1);
          }
        } else {
          const e = src.indexOf('"', p);
          emit(T.STR, e === -1 ? n : e);
        }
        break;

      case S.md_string:
        if (c === 96) {
          if (src.charCodeAt(p + 1) === 34) {
            pop();
            advance(p + 2);
          } else {
            ok = false;
          }
        } else if (c === 34) {
          quote();
        } else {
          let q = p + 1;
          while (q < n) {
            const ch = src.charCodeAt(q);
            if (ch === 96 || ch === 34) break;
            q++;
          }
          emit(T.MD_STR, q);
        }
        break;

      case S.acc_title:
      case S.acc_descr: {
        let e = src.indexOf('\n', p);
        if (e === -1) e = n;
        const type = state === S.acc_title ? T.acc_title_value : T.acc_descr_value;
        pop();
        emit(type, e);
        break;
      }

      case S.acc_descr_multiline:
        if (c === 125) {
          pop();
          advance(p + 1);
        } else {
          let e = src.indexOf('}', p);
          if (e === -1) e = n;
          emit(T.acc_descr_multiline_value, e);
        }
        break;

      case S.dir: {
        let e: number;
        if (c === 34) {
          quote();
        } else if ((e = at(RE_NODIR, p)) >= 0) {
          pop();
          emit(T.NODIR, e);
        } else if ((e = at(RE_DIR, p)) >= 0 || (e = at(RE_DIR_SYM, p)) >= 0 || (e = at(RE_DIR_V, p)) >= 0) {
          pop();
          emit(T.DIR, e);
        } else {
          ok = opener(c);
        }
        break;
      }

      case S.text: {
        const d = src.charCodeAt(p + 1);
        switch (c) {
          case 34:
            quote();
            break;
          case 40:
          case 91:
          case 123:
            opener(c);
            break;
          case 93:
            pop();
            if (d === 41) emitFlat(T.STADIUMEND, p + 2);
            else if (d === 93) emitFlat(T.SUBROUTINEEND, p + 2);
            else emitFlat(T.SQE, p + 1);
            break;
          case 41:
            pop();
            if (d === 93) emitFlat(T.CYLINDEREND, p + 2);
            else if (d === 41 && src.charCodeAt(p + 2) === 41) emitFlat(T.DOUBLECIRCLEEND, p + 3);
            else emitFlat(T.PE, p + 1);
            break;
          case 124:
            pop();
            emitFlat(T.PIPE, p + 1);
            break;
          case 125:
            pop();
            emitFlat(T.DIAMOND_STOP, p + 1);
            break;
          case 37: {
            // Jison drops the lone `%` from Mermaid's text pattern, so only a comment may start with one.
            const e = commentEnd(p);
            if (e >= 0) advance(e);
            else ok = false;
            break;
          }
          default: {
            let q = p + 1;
            while (q < n) {
              const ch = src.charCodeAt(q);
              if (
                ch === 91 ||
                ch === 93 ||
                ch === 40 ||
                ch === 41 ||
                ch === 123 ||
                ch === 125 ||
                ch === 124 ||
                ch === 34 ||
                ch === 37
              ) {
                break;
              }
              q++;
            }
            emit(T.TEXT, q);
          }
        }
        break;
      }

      case S.ellipseText: {
        const d = src.charCodeAt(p + 1);
        let e: number;
        if (c === 34) {
          quote();
        } else if (c === 37 && (e = commentEnd(p)) >= 0) {
          advance(e);
        } else if ((c === 45 || c === 47 || c === 41) && d === 41) {
          pop();
          emitFlat(T.ELLIPSE_END, p + 2);
        } else if (c === 40 || c === 91 || c === 123) {
          opener(c);
        } else if (c === 41 || c === 93 || c === 125) {
          ok = false;
        } else {
          let q = p + 1;
          while (q < n) {
            const ch = src.charCodeAt(q);
            if (ch === 34 || ch === 40 || ch === 41 || ch === 91 || ch === 93 || ch === 123 || ch === 125) break;
            if ((ch === 45 || ch === 47) && src.charCodeAt(q + 1) === 41) break;
            if (ch === 37 && src.charCodeAt(q + 1) === 37 && src.charCodeAt(q + 2) !== 123) break;
            q++;
          }
          emitChars(T.TEXT, q);
        }
        break;
      }

      case S.trapText: {
        const d = src.charCodeAt(p + 1);
        if (c === 34) {
          quote();
          break;
        }
        switch (c) {
          case 40:
            if (d === 45 || d === 91 || (d === 40 && src.charCodeAt(p + 2) === 40)) {
              opener(c);
            } else if (d === 93) {
              pop();
              emitFlat(T.TRAPEND, p + 2);
            } else {
              opener(c);
            }
            break;
          case 91:
          case 123:
            opener(c);
            break;
          case 92:
            if (d === 93) {
              pop();
              emitFlat(T.TRAPEND, p + 2);
            } else {
              emitFlat(T.TEXT, p + 1);
            }
            break;
          case 47:
            if (d === 93) {
              pop();
              emitFlat(T.INVTRAPEND, p + 2);
            } else {
              emitFlat(T.TEXT, p + 1);
            }
            break;
          case 93:
          case 41:
            if (d === 93) {
              pop();
              emitFlat(T.TRAPEND, p + 2);
            } else {
              ok = false;
            }
            break;
          case 125:
            ok = false;
            break;
          case 37: {
            const e = commentEnd(p);
            if (e >= 0) advance(e);
            else ok = false;
            break;
          }
          default: {
            if ((c === 63 || c === 61) && d === 93) {
              pop();
              emitFlat(T.TRAPEND, p + 2);
              break;
            }
            let q = p + 1;
            while (q < n) {
              const ch = src.charCodeAt(q);
              if (
                ch === 92 ||
                ch === 91 ||
                ch === 93 ||
                ch === 40 ||
                ch === 41 ||
                ch === 123 ||
                ch === 125 ||
                ch === 47 ||
                ch === 37
              ) {
                break;
              }
              q++;
            }
            emit(T.TEXT, q);
          }
        }
        break;
      }

      case S.edgeText: {
        if (c === 34) {
          quote();
          break;
        }
        const e = arrowEnd(p);
        if (e >= 0) {
          pop();
          emit(T.LINK, e);
          break;
        }
        if (c === 45 && src.charCodeAt(p + 1) === 45) {
          ok = false;
          break;
        }
        let q = p + 1;
        while (q < n) {
          const ch = src.charCodeAt(q);
          if (ch === 34) break;
          if (ch === 45) {
            if (src.charCodeAt(q + 1) === 45) break;
          } else if (isSpace(ch) && arrowEnd(q) >= 0) {
            break;
          }
          q++;
        }
        emitChars(T.EDGE_TEXT, q);
        break;
      }

      case S.click:
        if (c === 34) {
          quote();
        } else if (isSpace(c)) {
          pop();
          advance(p + 1);
        } else {
          let q = p + 1;
          while (q < n && !isSpace(src.charCodeAt(q))) q++;
          emitFlat(T.CLICK, q);
        }
        break;

      case S.callbackname:
        if (c === 40) {
          const e = at(RE_EMPTY_CALL, p);
          pop();
          if (e >= 0) {
            advance(e);
          } else {
            push(S.callbackargs);
            advance(p + 1);
          }
        } else {
          let e = src.indexOf('(', p);
          if (e === -1) e = n;
          emit(T.CALLBACKNAME, e);
        }
        break;

      case S.callbackargs:
        if (c === 41) {
          pop();
          advance(p + 1);
        } else {
          let e = src.indexOf(')', p);
          if (e === -1) e = n;
          emit(T.CALLBACKARGS, e);
        }
        break;

      case S.shapeData:
        if (c === 34) {
          push(S.shapeDataStr);
          emitFlat(T.SHAPE_DATA, p + 1);
        } else if (c === 125) {
          pop();
          advance(p + 1);
        } else if (c === 94) {
          ok = false;
        } else {
          let q = p + 1;
          while (q < n) {
            const ch = src.charCodeAt(q);
            if (ch === 125 || ch === 94 || ch === 34) break;
            q++;
          }
          emit(T.SHAPE_DATA, q);
        }
        break;

      case S.shapeDataStr:
        if (c === 34) {
          pop();
          emitFlat(T.SHAPE_DATA, p + 1);
        } else {
          let e = src.indexOf('"', p);
          if (e === -1) e = n;
          emitText(T.SHAPE_DATA, src.slice(p, e).replace(RE_BR, '<br/>'), e);
        }
        break;
    }

    if (!ok) {
      types.push(T.ERROR);
      texts.push('');
      starts.push(p);
      locs.push(line, col, line, col);
      return { types, texts, starts, locs };
    }
    continue lexing;
  }

  types.push(T.END);
  texts.push('');
  starts.push(n);
  locs.push(line, col, line, col);
  return { types, texts, starts, locs };
}
