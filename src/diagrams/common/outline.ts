import { syntaxError } from '../../errors.js';
import { isWord } from '../../util/chars.js';

// The indented outline grammar that Mermaid's mindmap and kanban diagrams share.
// Kanban adds `@{ ... }` metadata after a node and keeps `@` out of node ids.

export const enum T {
  END,
  ERROR,
  SPACELINE,
  NL,
  EOF,
  KEYWORD,
  SPACELIST,
  ICON,
  CLASS,
  NODE_DSTART,
  NODE_ID,
  NODE_DESCR,
  NODE_DEND,
  SHAPE_DATA,
}

// As the generated parser prints them. It has no name for the end of input and prints its number.
const NAMES = [
  '1',
  'INVALID',
  'SPACELINE',
  'NL',
  'EOF',
  '',
  'SPACELIST',
  'ICON',
  'CLASS',
  'NODE_DSTART',
  'NODE_ID',
  'NODE_DESCR',
  'NODE_DEND',
  'SHAPE_DATA',
];

export function tokenName(type: number, kanban: boolean): string {
  return type === T.KEYWORD ? (kanban ? 'KANBAN' : 'MINDMAP') : NAMES[type];
}

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

export const NODE_TYPE = {
  DEFAULT: 0,
  NO_BORDER: 0,
  ROUNDED_RECT: 1,
  RECT: 2,
  CIRCLE: 3,
  CLOUD: 4,
  BANG: 5,
  HEXAGON: 6,
} as const;

// Any opening delimiter pairs with any closing one, as in Mermaid.
export function getType(start: string, end: string): number {
  switch (start) {
    case '[':
      return NODE_TYPE.RECT;
    case '(':
      return end === ')' ? NODE_TYPE.ROUNDED_RECT : NODE_TYPE.CLOUD;
    case '((':
      return NODE_TYPE.CIRCLE;
    case ')':
      return NODE_TYPE.CLOUD;
    case '))':
      return NODE_TYPE.BANG;
    case '{{':
      return NODE_TYPE.HEXAGON;
    default:
      return NODE_TYPE.DEFAULT;
  }
}

export interface OutlineDb {
  getType(start: string, end: string): number;
  addNode(level: number, id: string, descr: string, type: number, shapeData?: string): void;
  decorateNode(decoration: { icon?: string; class?: string }): void;
}

const enum S {
  INITIAL,
  CLASS,
  ICON,
  NODE,
  NSTR,
  NSTR2,
  DATA,
  DATA_STR,
}

const RE_SPACE = /\s+/y;
const RE_LINE = /.+/y;
const RE_ID = /[^([\n){}]+/y;
const RE_ID_KANBAN = /[^([\n){}@]+/y;
const RE_DESCR = /[^)\](}]+/y;
const RE_MD = /[^`"]+/y;
const RE_DATA = /[^}^"]+/y;
const RE_BR = /\n\s*/g;

// Jison's case-insensitive flag folds ASCII letters only.
function letters(src: string, p: number, word: string): boolean {
  for (let k = 0; k < word.length; k++) if ((src.charCodeAt(p + k) | 32) !== word.charCodeAt(k)) return false;
  return true;
}

export function tokenize(src: string, kanban: boolean): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const keyword = kanban ? 'kanban' : 'mindmap';
  const reId = kanban ? RE_ID_KANBAN : RE_ID;
  let state: number = S.INITIAL;
  let p = 0;
  let nextBreak = -1;

  const put = (type: number, text: string, end: number): void => {
    types.push(type);
    texts.push(text);
    starts.push(p);
    p = end;
  };
  const emit = (type: number, end: number, next: number): void => {
    put(type, src.slice(p, end), end);
    state = next;
  };
  const at = (re: RegExp, from: number): number => {
    re.lastIndex = from;
    return re.test(src) ? re.lastIndex : -1;
  };
  const upTo = (ch: string): number => {
    const e = src.indexOf(ch, p);
    return e === -1 ? n : e;
  };

  while (p < n) {
    const c = src.charCodeAt(p);
    const d = src.charCodeAt(p + 1);
    let e: number;
    if (state === S.INITIAL) {
      const w = c <= 32 || c >= 160 ? Math.max(at(RE_SPACE, p), p) : p;
      if (kanban && c === 64 && d === 123) {
        put(T.SHAPE_DATA, '', p + 2);
        state = S.DATA;
      } else if (src.charCodeAt(w) === 37 && src.charCodeAt(w + 1) === 37) {
        emit(T.SPACELINE, at(RE_LINE, w), S.INITIAL);
      } else if (w > p) {
        if (nextBreak < p) nextBreak = upTo('\n');
        if (nextBreak >= w) emit(T.SPACELIST, w, S.INITIAL);
        else if ((e = src.lastIndexOf('\n', w - 1)) > p) emit(T.SPACELINE, e + 1, S.INITIAL);
        else emit(T.NL, p + 1, S.INITIAL);
      } else if (letters(src, p, keyword) && !isWord(src.charCodeAt(p + keyword.length))) {
        emit(T.KEYWORD, p + keyword.length, S.INITIAL);
      } else if (c === 58 && d === 58 && src.charCodeAt(p + 2) === 58) {
        p += 3;
        state = S.CLASS;
      } else if (c === 58 && d === 58 && letters(src, p + 2, 'icon') && src.charCodeAt(p + 6) === 40) {
        p += 7;
        state = S.ICON;
      } else if (
        (c === 45 && d === 41) ||
        (c === 40 && (d === 45 || d === 40)) ||
        ((c === 41 || c === 123) && d === c)
      ) {
        emit(T.NODE_DSTART, p + 2, S.NODE);
      } else if (c === 41 || c === 40 || c === 91) {
        emit(T.NODE_DSTART, p + 1, S.NODE);
      } else if ((e = at(reId, p)) !== -1) {
        emit(T.NODE_ID, e, S.INITIAL);
      } else {
        break;
      }
    } else if (state === S.NODE) {
      if (c === 34) {
        state = d === 96 ? S.NSTR2 : S.NSTR;
        p += d === 96 ? 2 : 1;
      } else if (c === 41 || c === 40) {
        emit(T.NODE_DEND, d === c || (c === 40 && d === 45) ? p + 2 : p + 1, S.INITIAL);
      } else if (c === 93) {
        emit(T.NODE_DEND, p + 1, S.INITIAL);
      } else if ((c === 125 && d === 125) || (c === 45 && d === 41)) {
        emit(T.NODE_DEND, p + 2, S.INITIAL);
      } else {
        // A lone closing brace takes the rest of its line as text.
        emit(T.NODE_DESCR, at(c === 125 ? RE_LINE : RE_DESCR, p), S.NODE);
      }
    } else if (state === S.NSTR) {
      if (c === 34) {
        p++;
        state = S.NODE;
      } else {
        emit(T.NODE_DESCR, upTo('"'), S.NSTR);
      }
    } else if (state === S.NSTR2) {
      if (c === 96 && d === 34) {
        p += 2;
        state = S.NODE;
      } else if ((e = at(RE_MD, p)) !== -1) {
        emit(T.NODE_DESCR, e, S.NSTR2);
      } else {
        break;
      }
    } else if (state === S.CLASS) {
      if ((e = at(RE_LINE, p)) !== -1) {
        emit(T.CLASS, e, S.INITIAL);
      } else if (c === 10) {
        p++;
        state = S.INITIAL;
      } else {
        break;
      }
    } else if (state === S.ICON) {
      if (c === 41) {
        p++;
        state = S.INITIAL;
      } else {
        emit(T.ICON, upTo(')'), S.ICON);
      }
    } else if (state === S.DATA) {
      if (c === 34) {
        emit(T.SHAPE_DATA, p + 1, S.DATA_STR);
      } else if (c === 125) {
        p++;
        state = S.INITIAL;
      } else if ((e = at(RE_DATA, p)) !== -1) {
        emit(T.SHAPE_DATA, e, S.DATA);
      } else {
        break;
      }
    } else if (c === 34) {
      emit(T.SHAPE_DATA, p + 1, S.DATA);
    } else {
      e = upTo('"');
      put(T.SHAPE_DATA, src.slice(p, e).replace(RE_BR, '<br/>'), e);
    }
  }

  if (p < n) {
    put(T.ERROR, '', p);
  } else {
    if (state === S.INITIAL) put(T.EOF, '', n);
    put(T.END, '', n);
  }
  return { types, texts, starts };
}

const STOP = "'SPACELINE', 'NL', 'EOF'";
const START = new Uint8Array(T.SHAPE_DATA + 1);
for (const t of [T.SPACELINE, T.SPACELIST, T.ICON, T.CLASS, T.NODE_DSTART, T.NODE_ID]) START[t] = 1;

function isStop(type: number): boolean {
  return type === T.SPACELINE || type === T.NL || type === T.EOF;
}

// Makes the calls on `db` that the grammar's actions make, in the same order, and reports
// errors in the state where the generated parser would.
export function parseOutline(src: string, db: OutlineDb, kanban: boolean): void {
  const diagram = kanban ? 'kanban' : 'mindmap';
  const { types, texts, starts } = tokenize(src, kanban);
  const data = kanban ? ", 'SHAPE_DATA'" : '';
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError(diagram, src, starts[i], '', true);
    throw syntaxError(diagram, src, starts[i], `Expecting ${expected}, got '${tokenName(t, kanban)}'`);
  };

  const keyword = `'${tokenName(T.KEYWORD, kanban)}'`;
  if (types[i] === T.SPACELINE) {
    i++;
    while (types[i] === T.SPACELINE || types[i] === T.NL) i++;
    if (types[i] !== T.KEYWORD) fail(`'SPACELINE', 'NL', ${keyword}`);
  } else if (types[i] !== T.KEYWORD) {
    fail(`'SPACELINE', ${keyword}`);
  }
  i++;
  let first = "'SPACELINE', 'NL', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'";
  if (types[i] === T.NL) {
    i++;
    first = "'SPACELINE', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'";
  }

  for (;;) {
    let t = types[i];
    let level = 0;
    const indented = t === T.SPACELIST;
    if (indented) {
      level = texts[i++].length;
      t = types[i];
    }
    if (t === T.NODE_ID || t === T.NODE_DSTART) {
      let id: string | undefined;
      let descr: string;
      let type = 0;
      if (t === T.NODE_ID) id = texts[i++];
      if (types[i] === T.NODE_DSTART) {
        const start = texts[i++];
        if (types[i] !== T.NODE_DESCR) fail("'NODE_DESCR'");
        descr = texts[i++];
        if (types[i] !== T.NODE_DEND) fail("'NODE_DEND'");
        const end = texts[i++];
        if (!isStop(types[i]) && types[i] !== T.SHAPE_DATA) fail(STOP + data);
        type = db.getType(start, end);
      } else {
        if (!isStop(types[i]) && types[i] !== T.SHAPE_DATA) fail(STOP + ", 'NODE_DSTART'" + data);
        descr = id!;
      }
      if (types[i] === T.SHAPE_DATA) {
        let shapeData = texts[i++];
        while (types[i] === T.SHAPE_DATA) shapeData += texts[i++];
        if (!isStop(types[i])) fail(STOP + data);
        db.addNode(level, id ?? descr, descr, type, shapeData);
      } else {
        db.addNode(level, id ?? descr, descr, type);
      }
    } else if (t === T.ICON || t === T.CLASS) {
      const text = texts[i++];
      if (!isStop(types[i])) fail(STOP);
      db.decorateNode(t === T.ICON ? { icon: text } : { class: text });
    } else if (indented) {
      if (!isStop(t)) fail(STOP + ", 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'");
    } else if (t === T.SPACELINE) {
      if (!isStop(types[++i])) fail(STOP);
    } else {
      fail(first);
    }

    i++;
    while (types[i] === T.NL || types[i] === T.EOF) i++;
    if (types[i] === T.END) return;
    if (START[types[i]] === 0) fail(STOP + ", 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'");
  }
}
