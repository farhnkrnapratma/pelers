import { T } from './tokens.js';
import { isSpace, isWord } from '../../util/chars.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
  // Set when lexing stopped on the grammar's own error rather than on unrecognized text.
  error: string | undefined;
}

const enum S {
  INITIAL,
  struct,
  STATE,
  STATE_STRING,
  STATE_ID,
  SCALE,
  acc_title,
  acc_descr,
  acc_descr_multiline,
  CLASSDEF,
  CLASSDEFID,
  CLASS,
  CLASS_STYLE,
  STYLE,
  STYLEDEF_STYLES,
  NOTE,
  NOTE_ID,
  NOTE_TEXT,
  FLOATING_NOTE,
  FLOATING_NOTE_ID,
}

const RE_DIRECTION = /direction/gi;
const RE_STEREOTYPE = /<<(?:fork|join|choice)>>|\[\[(?:fork|join|choice)\]\]/gi;
const RE_LINE_END = /[\n\r\u2028\u2029]/g;
const RE_PERCENT = /%%/g;
const RE_BRACE = /\{/g;

// Finds the next match of a pattern at or after a position, remembering the answer so that
// asking at each token costs one scan of the text in total.
function finder(re: RegExp, src: string): (q: number) => number {
  let from = 0;
  let next = -2;
  return (q) => {
    if (next === -2 || q < from || (next !== -1 && q > next)) {
      re.lastIndex = q;
      const m = re.exec(src);
      next = m ? m.index : -1;
      from = q;
    }
    return next;
  };
}

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  let error: string | undefined;

  const nextDirection = finder(RE_DIRECTION, src);
  const nextStereotype = finder(RE_STEREOTYPE, src);
  const nextLineEnd = finder(RE_LINE_END, src);
  const nextPercent = finder(RE_PERCENT, src);
  const nextBrace = finder(RE_BRACE, src);

  // The last `direction <dir>` of each kind on the line being lexed, and where its keyword ends.
  let dirFrom = -1;
  let dirTo = -1;
  const dirAt = [-1, -1, -1, -1];
  const dirEnd = [0, 0, 0, 0];
  // The same for <<fork>>, <<join>>, <<choice>> and their [[ ]] spellings.
  let stereoFrom = -1;
  let stereoTo = -1;
  const stereoAt = [-1, -1, -1, -1, -1, -1];

  const emit = (type: number, s: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(s, e));
    starts.push(s);
    p = e;
  };
  const emitText = (type: number, text: string, s: number, e: number): void => {
    types.push(type);
    texts.push(text);
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
  // Case-insensitive match of a lowercase ASCII word, as the grammar's /i flag gives.
  const word = (w: string, q: number): boolean => {
    for (let i = 0; i < w.length; i++) {
      const a = w.charCodeAt(i);
      const c = src.charCodeAt(q + i);
      if (c !== a && !(a >= 97 && a <= 122 && c === a - 32)) return false;
    }
    return true;
  };
  const wsEnd = (q: number): number => {
    let j = q;
    while (j < n && isSpace(src.charCodeAt(j))) j++;
    return j;
  };
  const wordEnd = (q: number): number => {
    let j = q;
    while (j < n && isWord(src.charCodeAt(j))) j++;
    return j;
  };
  const eol = (q: number): number => {
    const k = src.indexOf('\n', q);
    return k === -1 ? n : k;
  };
  const lineEnd = (q: number): number => {
    const k = nextLineEnd(q);
    return k === -1 ? n : k;
  };

  // Rules `.*direction\s+TB[^\n]*` and its three siblings: the first kind, in grammar order,
  // found between q and the end of its line.
  const direction = (q: number): number => {
    if (q < dirFrom || q > dirTo) {
      const first = nextDirection(q);
      const end = first === -1 ? n : lineEnd(q);
      dirFrom = q;
      dirTo = end;
      dirAt[0] = dirAt[1] = dirAt[2] = dirAt[3] = -1;
      for (let d = first; d !== -1 && d < end; d = nextDirection(d + 1)) {
        const w = wsEnd(d + 9);
        if (w === d + 9) continue;
        const kind = word('tb', w) ? 0 : word('bt', w) ? 1 : word('rl', w) ? 2 : word('lr', w) ? 3 : -1;
        if (kind === -1) continue;
        dirAt[kind] = d;
        dirEnd[kind] = w + 2;
      }
    }
    for (let kind = 0; kind < 4; kind++) if (dirAt[kind] >= q) return kind;
    return -1;
  };

  const stereotype = (q: number): number => {
    if (q < stereoFrom || q > stereoTo) {
      const first = nextStereotype(q);
      const end = first === -1 ? n : lineEnd(q);
      stereoFrom = q;
      stereoTo = end;
      stereoAt.fill(-1);
      for (let d = first; d !== -1 && d < end; d = nextStereotype(d + 1)) {
        const letter = src.charCodeAt(d + 2) | 32;
        stereoAt[(src.charCodeAt(d) === 60 ? 0 : 3) + (letter === 102 ? 0 : letter === 106 ? 1 : 2)] = d;
      }
    }
    for (let kind = 0; kind < 6; kind++) if (stereoAt[kind] >= q) return kind;
    return -1;
  };

  // The grammar's processId(): an id ends at an inline `%%`, and a text that starts with one is dropped.
  const emitId = (s: number, e: number): boolean => {
    const at = nextPercent(s);
    if (at === -1 || at + 2 > e) {
      emit(T.ID, s, e);
      return true;
    }
    if (at === s) {
      p = e;
      return false;
    }
    emit(T.ID, s, at);
    return true;
  };

  const finish = (type: number): Tokens => {
    types.push(type);
    texts.push('');
    starts.push(p);
    return { types, texts, starts, error };
  };

  for (;;) {
    if (p >= n) {
      // Jison tries the current state's rules once more on the empty remainder.
      switch (state) {
        case S.INITIAL:
          emit(T.NL, n, n);
          break;
        case S.acc_title:
          emit(T.acc_title_value, n, n);
          break;
        case S.acc_descr:
          emit(T.acc_descr_value, n, n);
          break;
        case S.acc_descr_multiline:
          emit(T.acc_descr_multiline_value, n, n);
          break;
        case S.CLASSDEFID:
          emit(T.CLASSDEF_STYLEOPTS, n, n);
          break;
        case S.CLASS_STYLE:
          emit(T.STYLECLASS, n, n);
          break;
        case S.STYLEDEF_STYLES:
          emit(T.STYLEDEF_STYLEOPTS, n, n);
          break;
        case S.STATE_STRING:
          emit(T.STATE_DESCR, n, n);
          break;
        case S.FLOATING_NOTE:
          emit(T.NOTE_TEXT, n, n);
          break;
        case S.STATE_ID:
        case S.FLOATING_NOTE_ID:
          emit(T.ID, n, n);
          break;
      }
      return finish(T.END);
    }

    const c = src.charCodeAt(p);
    switch (state) {
      case S.INITIAL:
      case S.struct: {
        const root = state === S.INITIAL;
        const lc = c | 32;
        if (root) {
          if (lc === 99 && word('click', p) && !isWord(src.charCodeAt(p + 5))) {
            emit(T.CLICK, p, p + 5);
            continue;
          }
          if (lc === 104 && word('href', p) && !isWord(src.charCodeAt(p + 4))) {
            emit(T.HREF, p, p + 4);
            continue;
          }
          if (c === 34) {
            const close = src.indexOf('"', p + 1);
            if (close !== -1) {
              emit(T.STRING, p, close + 1);
              continue;
            }
          }
          if (lc === 100 && word('default', p) && !isWord(src.charCodeAt(p + 7))) {
            emit(T.DEFAULT, p, p + 7);
            continue;
          }
          const kind = direction(p);
          if (kind !== -1) {
            emit(T.direction_tb + kind, p, eol(dirEnd[kind]));
            continue;
          }
          if (c === 10) {
            let j = p + 1;
            while (src.charCodeAt(j) === 10) j++;
            emit(T.NL, p, j);
            continue;
          }
          if (isSpace(c)) {
            p = wsEnd(p);
            continue;
          }
        } else if (c !== 10 && isSpace(c)) {
          let j = p + 1;
          while (j < n && src.charCodeAt(j) !== 10 && isSpace(src.charCodeAt(j))) j++;
          p = j;
          continue;
        }
        if (c === 35 || (c === 37 && src.charCodeAt(p + 1) === 37 && src.charCodeAt(p + 2) !== 123)) {
          p = eol(p);
          continue;
        }
        if (lc === 115) {
          if (root && word('scale', p)) {
            const e = wsEnd(p + 5);
            if (e > p + 5) {
              emit(T.scale, p, e);
              push(S.SCALE);
              continue;
            }
          }
          if (word('style', p)) {
            const e = wsEnd(p + 5);
            if (e > p + 5) {
              emit(T.style, p, e);
              push(S.STYLE);
              continue;
            }
          }
          if (word('state', p)) {
            let e = wsEnd(p + 5);
            if (e > p + 5) {
              p = e;
              push(S.STATE);
              continue;
            }
            if (root && word('diagram', p + 5)) {
              const from = word('-v2', p + 12) ? p + 15 : p + 12;
              e = wsEnd(from);
              if (e > from) {
                emit(T.SD, p, e);
                continue;
              }
            }
          }
        } else if (lc === 99) {
          if (word('class', p)) {
            const def = word('def', p + 5);
            const from = def ? p + 8 : p + 5;
            const e = wsEnd(from);
            if (e > from) {
              emit(def ? T.classDef : T.class, p, e);
              push(def ? S.CLASSDEF : S.CLASS);
              continue;
            }
          }
        } else if (lc === 97 && root) {
          if (word('acctitle', p) || word('accdescr', p)) {
            const title = (src.charCodeAt(p + 3) | 32) === 116;
            const q = wsEnd(p + 8);
            const d = src.charCodeAt(q);
            if (d === 58) {
              emit(title ? T.acc_title : T.acc_descr, p, wsEnd(q + 1));
              push(title ? S.acc_title : S.acc_descr);
              continue;
            }
            if (d === 123 && !title) {
              p = wsEnd(q + 1);
              push(S.acc_descr_multiline);
              continue;
            }
          }
        }
        if (root) {
          if (c === 123) {
            emit(T.STRUCT_START, p, p + 1);
            push(S.struct);
            continue;
          }
        } else {
          const kind = direction(p);
          if (kind !== -1) {
            emit(T.direction_tb + kind, p, eol(dirEnd[kind]));
            continue;
          }
          if (c === 125) {
            emit(T.STRUCT_STOP, p, p + 1);
            pop();
            continue;
          }
          if (c === 10) {
            p++;
            continue;
          }
        }
        if (lc === 110 && word('note', p)) {
          const e = wsEnd(p + 4);
          if (e > p + 4) {
            emit(T.note, p, e);
            push(S.NOTE);
            continue;
          }
        }
        if (root && lc === 104 && word('hide empty description', p) && !isWord(src.charCodeAt(p + 22))) {
          emit(T.HIDE_EMPTY, p, p + 22);
          continue;
        }
        if (c === 91 && src.charCodeAt(p + 1) === 42 && src.charCodeAt(p + 2) === 93) {
          emit(T.EDGE_STATE, p, p + 3);
          continue;
        }
        let j = p;
        while (j < n) {
          const d = src.charCodeAt(j);
          if (d === 58 || d === 45 || d === 123 || isSpace(d)) break;
          j++;
        }
        if (j > p) {
          emitId(p, j);
          continue;
        }
        if (c === 58) {
          j = p + 1;
          while (j < n) {
            const d = src.charCodeAt(j);
            if (d === 10 || d === 59) break;
            if (d === 58) {
              const after = src.charCodeAt(j + 1);
              if (j + 1 >= n || after === 58 || after === 10 || after === 59) break;
              j++;
            }
            j++;
          }
          if (j > p + 1) {
            emitText(T.DESCR, src.slice(p, j).trim(), p, j);
            continue;
          }
          if (src.charCodeAt(p + 1) === 58 && src.charCodeAt(p + 2) === 58) {
            emit(T.STYLE_SEPARATOR, p, p + 3);
            continue;
          }
        } else if (c === 45 && src.charCodeAt(p + 1) === 45) {
          if (src.charCodeAt(p + 2) === 62) {
            emit(T.ARROW, p, p + 3);
            continue;
          }
          if (!root) {
            emit(T.CONCURRENT, p, p + 2);
            continue;
          }
        }
        if (!root) return finish(T.ERROR);
        emit(T.INVALID, p, p + 1);
        continue;
      }

      case S.STATE: {
        if (c !== 10 && isSpace(c)) {
          let j = p + 1;
          while (j < n && src.charCodeAt(j) !== 10 && isSpace(src.charCodeAt(j))) j++;
          p = j;
          continue;
        }
        if (c === 35 || (c === 37 && src.charCodeAt(p + 1) === 37 && src.charCodeAt(p + 2) !== 123)) {
          p = eol(p);
          continue;
        }
        const kind = stereotype(p);
        if (kind !== -1) {
          const at = stereoAt[kind];
          const choice = kind === 2 || kind === 5;
          emitText(
            choice ? T.CHOICE : kind % 3 === 0 ? T.FORK : T.JOIN,
            src.slice(p, at).trim(),
            p,
            at + (choice ? 10 : 8)
          );
          pop();
          continue;
        }
        if (c === 34) {
          p++;
          push(S.STATE_STRING);
          continue;
        }
        const q = c === 10 ? wsEnd(p) : p;
        if (word('as', q)) {
          const e = wsEnd(q + 2);
          if (e > q + 2) {
            emit(T.AS, p, e);
            push(S.STATE_ID);
            continue;
          }
        }
        if (c === 10) {
          p++;
          pop();
          continue;
        }
        if (c === 123) {
          emit(T.STRUCT_START, p, p + 1);
          pop();
          push(S.struct);
          continue;
        }
        if (isWord(c)) {
          const first = wordEnd(p);
          const second = wsEnd(first);
          if (second > first && isWord(src.charCodeAt(second))) {
            const brace = nextBrace(second + 1);
            if (brace !== -1 && brace < lineEnd(second)) {
              error = `Error: State name must be a single word. Found: "${src.slice(p, brace + 1).trim()}"`;
              return finish(T.ERROR);
            }
          }
        }
        let j = p + 1;
        while (j < n) {
          const d = src.charCodeAt(j);
          if (d === 123 || isSpace(d)) break;
          j++;
        }
        emit(T.COMPOSIT_STATE, p, j);
        continue;
      }

      case S.STATE_STRING: {
        if (c === 34) {
          p++;
          pop();
          continue;
        }
        const close = src.indexOf('"', p);
        emit(T.STATE_DESCR, p, close === -1 ? n : close);
        continue;
      }

      case S.STATE_ID: {
        let j = p;
        while (j < n) {
          const d = src.charCodeAt(j);
          if (d === 10 || d === 123) break;
          j++;
        }
        if (emitId(p, j)) pop();
        continue;
      }

      case S.SCALE: {
        if (c >= 48 && c <= 57) {
          let j = p + 1;
          while (j < n && src.charCodeAt(j) >= 48 && src.charCodeAt(j) <= 57) j++;
          emit(T.WIDTH, p, j);
          continue;
        }
        const q = wsEnd(p);
        if (q > p && word('width', q) && !isWord(src.charCodeAt(q + 5))) {
          p = q + 5;
          pop();
          continue;
        }
        return finish(T.ERROR);
      }

      case S.acc_title:
        emit(T.acc_title_value, p, eol(p));
        pop();
        continue;

      case S.acc_descr:
        emit(T.acc_descr_value, p, eol(p));
        pop();
        continue;

      case S.acc_descr_multiline: {
        if (c === 125) {
          p++;
          pop();
          continue;
        }
        const close = src.indexOf('}', p);
        emit(T.acc_descr_multiline_value, p, close === -1 ? n : close);
        continue;
      }

      case S.CLASSDEF: {
        const isDefault = word('default', p) && wsEnd(p + 7) > p + 7;
        const j = isDefault ? p + 7 : wordEnd(p);
        const e = wsEnd(j);
        if (j === p || e === j) return finish(T.ERROR);
        emit(isDefault ? T.DEFAULT_CLASSDEF_ID : T.CLASSDEF_ID, p, e);
        pop();
        push(S.CLASSDEFID);
        continue;
      }

      case S.CLASSDEFID:
        emit(T.CLASSDEF_STYLEOPTS, p, eol(p));
        pop();
        continue;

      case S.CLASS: {
        let j = wordEnd(p);
        if (j === p) return finish(T.ERROR);
        while (src.charCodeAt(j) === 44) {
          const k = wsEnd(j + 1);
          const e = wordEnd(k);
          if (e === k) break;
          j = e;
        }
        emit(T.CLASSENTITY_IDS, p, j);
        pop();
        push(S.CLASS_STYLE);
        continue;
      }

      case S.CLASS_STYLE:
        emit(T.STYLECLASS, p, eol(p));
        pop();
        continue;

      case S.STYLE: {
        let j = p;
        while (j < n && (isWord(src.charCodeAt(j)) || src.charCodeAt(j) === 44)) j++;
        const e = wsEnd(j);
        if (j === p || e === j) return finish(T.ERROR);
        emit(T.STYLE_IDS, p, e);
        pop();
        push(S.STYLEDEF_STYLES);
        continue;
      }

      case S.STYLEDEF_STYLES:
        emit(T.STYLEDEF_STYLEOPTS, p, eol(p));
        pop();
        continue;

      case S.NOTE: {
        if (word('left of', p) && !isWord(src.charCodeAt(p + 7))) {
          emit(T.left_of, p, p + 7);
          pop();
          push(S.NOTE_ID);
          continue;
        }
        if (word('right of', p) && !isWord(src.charCodeAt(p + 8))) {
          emit(T.right_of, p, p + 8);
          pop();
          push(S.NOTE_ID);
          continue;
        }
        if (c === 34) {
          p++;
          pop();
          push(S.FLOATING_NOTE);
          continue;
        }
        return finish(T.ERROR);
      }

      case S.NOTE_ID: {
        const q = wsEnd(p);
        let j = q;
        while (j < n) {
          const d = src.charCodeAt(j);
          if (d === 58 || d === 45 || isSpace(d)) break;
          j++;
        }
        if (j === q) return finish(T.ERROR);
        if (emitId(p, j)) {
          pop();
          push(S.NOTE_TEXT);
        }
        continue;
      }

      case S.NOTE_TEXT: {
        const q = wsEnd(p);
        if (src.charCodeAt(q) === 58) {
          let j = q + 1;
          while (j < n) {
            const d = src.charCodeAt(j);
            if (d === 58 || d === 10 || d === 59) break;
            j++;
          }
          if (j > q + 1) {
            // Mermaid drops two characters here, expecting " :"; with no space before the colon
            // the first character of the note goes too.
            emitText(T.NOTE_TEXT, src.slice(p + 2, j).trim(), p, j);
            pop();
            continue;
          }
        }
        let k = src.indexOf('\n', p);
        let e = -1;
        while (k !== -1) {
          e = wsEnd(k);
          if (word('end note', e) && !isWord(src.charCodeAt(e + 8))) break;
          k = src.indexOf('\n', e);
        }
        if (k === -1) return finish(T.ERROR);
        emitText(T.NOTE_TEXT, src.slice(p, e).trim(), p, e + 8);
        pop();
        continue;
      }

      case S.FLOATING_NOTE: {
        const q = wsEnd(p);
        if (word('as', q)) {
          emit(T.AS, p, wsEnd(q + 2));
          pop();
          push(S.FLOATING_NOTE_ID);
          continue;
        }
        if (c === 34) {
          p++;
          continue;
        }
        const close = src.indexOf('"', p);
        emit(T.NOTE_TEXT, p, close === -1 ? n : close);
        continue;
      }

      case S.FLOATING_NOTE_ID:
        if (emitId(p, eol(p))) pop();
        continue;
    }
  }
}
