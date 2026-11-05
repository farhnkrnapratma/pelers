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
  md_string,
  acc_title,
  acc_descr,
  acc_descr_multiline,
  dir,
  text,
  ellipseText,
  trapText,
  edgeText,
  thickEdgeText,
  dottedEdgeText,
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
const RE_LINK = /\s*[xo<]?--+[-xo>]\s*/y;
const RE_START_LINK = /\s*[xo<]?--\s*/y;
const RE_THICK_LINK = /\s*[xo<]?==+[=xo>]\s*/y;
const RE_THICK_START = /\s*[xo<]?==\s*/y;
const RE_DOTTED_LINK = /\s*[xo<]?-?\.+-[xo>]?\s*/y;
const RE_DOTTED_START = /\s*[xo<]?-\.\s*/y;
const RE_INVIS_LINK = /\s*~~~+\s*/y;
const RE_NEWLINE = /(?:\r?\n)+/y;
const RE_SPACE = /[^\S\n\r]+/y;
const RE_EMPTY_CALL = /\(\s*\)/y;
const RE_BR = /\n\s*/g;

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  let firstGraph = true;
  let dirNext = src.indexOf('direction');
  let atNext = src.indexOf('@');
  let lineEnd = -1;
  let dirFailed = 0;
  let runEnd = 0;
  let runAt = -1;
  let wsFrom = -1;
  let wsTo = -1;

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
  const at = (re: RegExp, q: number): number => {
    re.lastIndex = q;
    return re.test(src) ? re.lastIndex : -1;
  };
  const kw = (word: string): boolean => src.startsWith(word, p) && !isWord(src.charCodeAt(p + word.length));

  // First character after any run of whitespace starting at q, skipping one optional x, o or <.
  const linkChar = (q: number): number => {
    let j = q;
    if (q >= wsFrom && q <= wsTo) {
      j = wsTo;
    } else if (isSpace(src.charCodeAt(q))) {
      while (j < n && isSpace(src.charCodeAt(j))) j++;
      wsFrom = q;
      wsTo = j;
    }
    const d = src.charCodeAt(j);
    return d === 120 || d === 111 || d === 60 ? src.charCodeAt(j + 1) : d;
  };
  const invisAhead = (q: number): boolean => {
    let j = q;
    if (q >= wsFrom && q <= wsTo) {
      j = wsTo;
    } else if (isSpace(src.charCodeAt(q))) {
      while (j < n && isSpace(src.charCodeAt(j))) j++;
      wsFrom = q;
      wsTo = j;
    }
    return src.charCodeAt(j) === 126;
  };

  const quote = (): void => {
    if (src.charCodeAt(p + 1) === 96) {
      push(S.md_string);
      p += 2;
    } else {
      push(S.string);
      p++;
    }
  };

  const invisLink = (): boolean => {
    if (!invisAhead(p)) return false;
    const e = at(RE_INVIS_LINK, p);
    if (e < 0) return false;
    emit(T.LINK, p, e);
    return true;
  };

  const opener = (c: number): boolean => {
    const d = src.charCodeAt(p + 1);
    if (c === 40) {
      if (d === 45) {
        push(S.ellipseText);
        emit(T.ELLIPSE_START, p, p + 2);
      } else if (d === 91) {
        push(S.text);
        emit(T.STADIUMSTART, p, p + 2);
      } else if (d === 40 && src.charCodeAt(p + 2) === 40) {
        push(S.text);
        emit(T.DOUBLECIRCLESTART, p, p + 3);
      } else {
        push(S.text);
        emit(T.PS, p, p + 1);
      }
      return true;
    }
    if (c === 91) {
      if (d === 91) {
        push(S.text);
        emit(T.SUBROUTINESTART, p, p + 2);
      } else if (d === 40) {
        push(S.text);
        emit(T.CYLINDERSTART, p, p + 2);
      } else if (d === 47) {
        push(S.trapText);
        emit(T.TRAPSTART, p, p + 2);
      } else if (d === 92) {
        push(S.trapText);
        emit(T.INVTRAPSTART, p, p + 2);
      } else {
        push(S.text);
        emit(T.SQS, p, p + 1);
      }
      return true;
    }
    if (c === 124) {
      push(S.text);
      emit(T.PIPE, p, p + 1);
      return true;
    }
    if (c === 123) {
      push(S.text);
      emit(T.DIAMOND_START, p, p + 1);
      return true;
    }
    return false;
  };

  const graph = (len: number): void => {
    if (firstGraph) {
      firstGraph = false;
      push(S.dir);
    }
    emit(T.GRAPH, p, p + len);
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
    emit(T.NODE_STRING, p, q);
    return true;
  };

  const link = (c: number): boolean => {
    const d = linkChar(p);
    let e: number;
    if (d === 45) {
      if ((e = at(RE_LINK, p)) >= 0) {
        emit(T.LINK, p, e);
        return true;
      }
      if ((e = at(RE_START_LINK, p)) >= 0) {
        push(S.edgeText);
        emit(T.START_LINK, p, e);
        return true;
      }
      if ((e = at(RE_DOTTED_LINK, p)) >= 0) {
        emit(T.LINK, p, e);
        return true;
      }
      if ((e = at(RE_DOTTED_START, p)) >= 0) {
        push(S.dottedEdgeText);
        emit(T.START_LINK, p, e);
        return true;
      }
    } else if (d === 61) {
      if ((e = at(RE_THICK_LINK, p)) >= 0) {
        emit(T.LINK, p, e);
        return true;
      }
      if ((e = at(RE_THICK_START, p)) >= 0) {
        push(S.thickEdgeText);
        emit(T.START_LINK, p, e);
        return true;
      }
    } else if (d === 46) {
      if ((e = at(RE_DOTTED_LINK, p)) >= 0) {
        emit(T.LINK, p, e);
        return true;
      }
    }
    return c !== 45 && c !== 61 && c !== 46 && invisLink();
  };

  const initial = (c: number): boolean => {
    let e: number;
    switch (c) {
      case 97:
        if (src.startsWith('acc', p)) {
          if ((e = at(RE_ACC_TITLE, p)) >= 0) {
            push(S.acc_title);
            emit(T.acc_title, p, e);
            return true;
          }
          if ((e = at(RE_ACC_DESCR, p)) >= 0) {
            push(S.acc_descr);
            emit(T.acc_descr, p, e);
            return true;
          }
          if ((e = at(RE_ACC_DESCR_ML, p)) >= 0) {
            push(S.acc_descr_multiline);
            p = e;
            return true;
          }
        }
        break;
      case 64:
        if (src.charCodeAt(p + 1) === 123) {
          push(S.shapeData);
          types.push(T.SHAPE_DATA);
          texts.push('');
          starts.push(p);
          p += 2;
          return true;
        }
        break;
      case 99:
        if ((e = at(RE_CALL, p)) >= 0) {
          push(S.callbackname);
          p = e;
          return true;
        }
        if (kw('classDef')) {
          emit(T.CLASSDEF, p, p + 8);
          return true;
        }
        if (kw('class')) {
          emit(T.CLASS, p, p + 5);
          return true;
        }
        if ((e = at(RE_CLICK, p)) >= 0) {
          push(S.click);
          p = e;
          return true;
        }
        break;
      case 34:
        quote();
        return true;
      case 115:
        if (kw('style')) {
          emit(T.STYLE, p, p + 5);
          return true;
        }
        if (kw('swimlane-beta')) {
          graph(13);
          return true;
        }
        if (kw('subgraph')) {
          emit(T.subgraph, p, p + 8);
          return true;
        }
        break;
      case 100:
        if (kw('default')) {
          emit(T.DEFAULT, p, p + 7);
          return true;
        }
        break;
      case 108:
        if (kw('linkStyle')) {
          emit(T.LINKSTYLE, p, p + 9);
          return true;
        }
        break;
      case 105:
        if (kw('interpolate')) {
          emit(T.INTERPOLATE, p, p + 11);
          return true;
        }
        break;
      case 104:
        if (src.startsWith('href', p) && isSpace(src.charCodeAt(p + 4))) {
          emit(T.HREF, p, p + 5);
          return true;
        }
        break;
      case 102:
        if (kw('flowchart-elk')) {
          graph(13);
          return true;
        }
        if (kw('flowchart')) {
          graph(9);
          return true;
        }
        break;
      case 103:
        if (kw('graph')) {
          graph(5);
          return true;
        }
        break;
      case 101:
        if (kw('end')) {
          e = p + 3;
          while (e < n && isSpace(src.charCodeAt(e))) e++;
          emit(T.end, p, e);
          return true;
        }
        break;
      case 95:
        if (kw('_self')) {
          emit(T.LINK_TARGET, p, p + 5);
          return true;
        }
        if (kw('_blank')) {
          emit(T.LINK_TARGET, p, p + 6);
          return true;
        }
        if (kw('_parent')) {
          emit(T.LINK_TARGET, p, p + 7);
          return true;
        }
        if (kw('_top')) {
          emit(T.LINK_TARGET, p, p + 4);
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
              emit(T.direction_tb + i, p, e);
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
        emit(T.LINK_ID, p, runAt + 1);
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
      emit(T.NUM, p, q);
      return true;
    }

    switch (c) {
      case 35:
        emit(T.BRKT, p, p + 1);
        return true;
      case 58:
        if (src.charCodeAt(p + 1) === 58 && src.charCodeAt(p + 2) === 58) emit(T.STYLE_SEPARATOR, p, p + 3);
        else emit(T.COLON, p, p + 1);
        return true;
      case 38:
        emit(T.AMP, p, p + 1);
        return true;
      case 59:
        emit(T.SEMI, p, p + 1);
        return true;
      case 44:
        emit(T.COMMA, p, p + 1);
        return true;
      case 42:
        emit(T.MULT, p, p + 1);
        return true;
      case 120:
      case 111:
        return link(c) || nodeString();
      case 60:
        if (link(c)) return true;
        emit(T.TAGSTART, p, p + 1);
        return true;
      case 45:
        if (link(c) || nodeString()) return true;
        emit(T.MINUS, p, p + 1);
        return true;
      case 61:
        return link(c);
      case 46:
        return link(c) || nodeString();
      case 126:
        return invisLink();
      case 40:
        return opener(c);
      case 91:
        if (src.charCodeAt(p + 1) === 124) {
          emit(T.VERTEX_WITH_PROPS_START, p, p + 2);
          return true;
        }
        return opener(c);
      case 124:
      case 123:
        return opener(c);
      case 62:
        push(S.text);
        emit(T.TAGEND, p, p + 1);
        return true;
      case 94:
        emit(T.UP, p, p + 1);
        return true;
      case 92:
        if (src.charCodeAt(p + 1) === 124) {
          emit(T.SEP, p, p + 2);
          return true;
        }
        return nodeString();
      case 118:
        if (!isWord(src.charCodeAt(p + 1))) {
          emit(T.DOWN, p, p + 1);
          return true;
        }
        return nodeString();
    }

    if (isSpace(c)) {
      if (link(c)) return true;
      if ((e = at(RE_NEWLINE, p)) >= 0) {
        emit(T.NEWLINE, p, e);
        return true;
      }
      if ((e = at(RE_SPACE, p)) >= 0) {
        emit(T.SPACE, p, e);
        return true;
      }
      return false;
    }

    if (c < 128) return nodeString();
    if (isUnicodeLetter(c)) {
      emit(T.UNICODE_TEXT, p, p + 1);
      return true;
    }
    return false;
  };

  const edgeText = (c: number, linkRe: RegExp, a: number, b: number): boolean => {
    if (c === 34) {
      quote();
      return true;
    }
    const d = linkChar(p);
    if (d === a || d === b) {
      const e = at(linkRe, p);
      if (e >= 0) {
        pop();
        emit(T.LINK, p, e);
        return true;
      }
    }
    if (c === a && (a !== 45 || src.charCodeAt(p + 1) === 45)) return false;
    let q = p + 1;
    while (q < n) {
      const ch = src.charCodeAt(q);
      if (ch === 34) break;
      if (ch === a) {
        if (a !== 45 || src.charCodeAt(q + 1) === 45) break;
      } else if (ch === 120 || ch === 111 || ch === 60 || ch === b || isSpace(ch)) {
        const dd = linkChar(q);
        if ((dd === a || dd === b) && at(linkRe, q) >= 0) break;
      }
      q++;
    }
    emit(T.EDGE_TEXT, p, q);
    return true;
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
      if (type !== -1) emit(type, n, n);
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
            p += 2;
          } else {
            pop();
            p++;
          }
        } else {
          const e = src.indexOf('"', p);
          emit(T.STR, p, e === -1 ? n : e);
        }
        break;

      case S.md_string:
        if (c === 96) {
          if (src.charCodeAt(p + 1) === 34) {
            pop();
            p += 2;
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
          emit(T.MD_STR, p, q);
        }
        break;

      case S.acc_title:
      case S.acc_descr: {
        let e = src.indexOf('\n', p);
        if (e === -1) e = n;
        const type = state === S.acc_title ? T.acc_title_value : T.acc_descr_value;
        pop();
        emit(type, p, e);
        break;
      }

      case S.acc_descr_multiline:
        if (c === 125) {
          pop();
          p++;
        } else {
          let e = src.indexOf('}', p);
          if (e === -1) e = n;
          emit(T.acc_descr_multiline_value, p, e);
        }
        break;

      case S.dir: {
        let e: number;
        if (c === 34) {
          quote();
        } else if ((e = at(RE_NODIR, p)) >= 0) {
          pop();
          emit(T.NODIR, p, e);
        } else if ((e = at(RE_DIR, p)) >= 0 || (e = at(RE_DIR_SYM, p)) >= 0 || (e = at(RE_DIR_V, p)) >= 0) {
          pop();
          emit(T.DIR, p, e);
        } else {
          ok = invisLink() || opener(c);
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
            if (d === 41) emit(T.STADIUMEND, p, p + 2);
            else if (d === 93) emit(T.SUBROUTINEEND, p, p + 2);
            else emit(T.SQE, p, p + 1);
            break;
          case 41:
            pop();
            if (d === 93) emit(T.CYLINDEREND, p, p + 2);
            else if (d === 41 && src.charCodeAt(p + 2) === 41) emit(T.DOUBLECIRCLEEND, p, p + 3);
            else emit(T.PE, p, p + 1);
            break;
          case 124:
            pop();
            emit(T.PIPE, p, p + 1);
            break;
          case 125:
            pop();
            emit(T.DIAMOND_STOP, p, p + 1);
            break;
          default: {
            if ((c === 126 || isSpace(c)) && invisLink()) break;
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
                ch === 34
              ) {
                break;
              }
              q++;
            }
            emit(T.TEXT, p, q);
          }
        }
        break;
      }

      case S.ellipseText: {
        const d = src.charCodeAt(p + 1);
        if (c === 34) {
          quote();
        } else if ((c === 126 || isSpace(c)) && invisLink()) {
          break;
        } else if ((c === 45 || c === 47 || c === 41) && d === 41) {
          pop();
          emit(T.ELLIPSE_END, p, p + 2);
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
            if ((ch === 126 || isSpace(ch)) && invisAhead(q) && at(RE_INVIS_LINK, q) >= 0) break;
            q++;
          }
          emit(T.TEXT, p, q);
        }
        break;
      }

      case S.trapText: {
        const d = src.charCodeAt(p + 1);
        if (c === 34) {
          quote();
          break;
        }
        if ((c === 126 || isSpace(c)) && invisLink()) break;
        switch (c) {
          case 40:
            if (d === 45 || d === 91 || (d === 40 && src.charCodeAt(p + 2) === 40)) {
              opener(c);
            } else if (d === 93) {
              pop();
              emit(T.TRAPEND, p, p + 2);
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
              emit(T.TRAPEND, p, p + 2);
            } else {
              emit(T.TEXT, p, p + 1);
            }
            break;
          case 47:
            if (d === 93) {
              pop();
              emit(T.INVTRAPEND, p, p + 2);
            } else {
              emit(T.TEXT, p, p + 1);
            }
            break;
          case 93:
          case 41:
            if (d === 93) {
              pop();
              emit(T.TRAPEND, p, p + 2);
            } else {
              ok = false;
            }
            break;
          case 125:
            ok = false;
            break;
          default: {
            if ((c === 63 || c === 61) && d === 93) {
              pop();
              emit(T.TRAPEND, p, p + 2);
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
                ch === 47
              ) {
                break;
              }
              q++;
            }
            emit(T.TEXT, p, q);
          }
        }
        break;
      }

      case S.edgeText:
        ok = edgeText(c, RE_LINK, 45, -1);
        break;
      case S.thickEdgeText:
        ok = edgeText(c, RE_THICK_LINK, 61, -1);
        break;
      case S.dottedEdgeText:
        ok = edgeText(c, RE_DOTTED_LINK, 46, 45);
        break;

      case S.click:
        if (c === 34) {
          quote();
        } else if (isSpace(c)) {
          pop();
          p++;
        } else {
          let q = p + 1;
          while (q < n && !isSpace(src.charCodeAt(q))) q++;
          emit(T.CLICK, p, q);
        }
        break;

      case S.callbackname:
        if (c === 40) {
          const e = at(RE_EMPTY_CALL, p);
          pop();
          if (e >= 0) {
            p = e;
          } else {
            push(S.callbackargs);
            p++;
          }
        } else {
          let e = src.indexOf('(', p);
          if (e === -1) e = n;
          emit(T.CALLBACKNAME, p, e);
        }
        break;

      case S.callbackargs:
        if (c === 41) {
          pop();
          p++;
        } else {
          let e = src.indexOf(')', p);
          if (e === -1) e = n;
          emit(T.CALLBACKARGS, p, e);
        }
        break;

      case S.shapeData:
        if (c === 34) {
          push(S.shapeDataStr);
          emit(T.SHAPE_DATA, p, p + 1);
        } else if (c === 125) {
          pop();
          p++;
        } else if (c === 94) {
          ok = false;
        } else {
          let q = p + 1;
          while (q < n) {
            const ch = src.charCodeAt(q);
            if (ch === 125 || ch === 94 || ch === 34) break;
            q++;
          }
          emit(T.SHAPE_DATA, p, q);
        }
        break;

      case S.shapeDataStr:
        if (c === 34) {
          pop();
          emit(T.SHAPE_DATA, p, p + 1);
        } else {
          let e = src.indexOf('"', p);
          if (e === -1) e = n;
          types.push(T.SHAPE_DATA);
          texts.push(src.slice(p, e).replace(RE_BR, '<br/>'));
          starts.push(p);
          p = e;
        }
        break;
    }

    if (!ok) {
      types.push(T.ERROR);
      texts.push('');
      starts.push(p);
      return { types, texts, starts };
    }
    continue lexing;
  }

  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
