import { T } from './tokens.js';
import { isSpace } from '../../util/chars.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

const enum S {
  INITIAL,
  acc_title,
  acc_descr,
  acc_descr_multiline,
  href,
  callbackname,
  callbackargs,
  click,
}

const RE_ACC_TITLE = /accTitle\s*:\s*/iy;
const RE_ACC_DESCR = /accDescr\s*:\s*/iy;
const RE_ACC_DESCR_ML = /accDescr\s*\{\s*/iy;
const RE_HREF = /href\s+"/iy;
const RE_CALL = /call\s+/iy;
const RE_CLICK = /click\s+/iy;
const RE_EMPTY_CALL = /\(\s*\)/y;
const RE_WEEKDAY = /weekday\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b/iy;
const DAYS = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'];
const RE_WEEKEND = /weekend\s+(?:(friday)|saturday)\b/iy;
const RE_DATE = /\d\d\d\d-\d\d-\d\d\b/y;
const RE_BARE_TITLE = /title[^\S\r\n]*(?=[\r\n]|$)/iy;

// Statements that start with a keyword, grouped by the keyword's first letter and kept in the grammar's rule order.
const KEYWORDS = new Map<number, [RegExp, number][]>([
  [
    97,
    [
      [/axisFormat\s[^#\n;]+/iy, T.axisFormat],
      [/accDescription\s[^#\n;]+/iy, T.accDescription],
    ],
  ],
  [103, [[/gantt\b/iy, T.gantt]]],
  [100, [[/dateFormat\s[^#\n;]+/iy, T.dateFormat]]],
  [
    105,
    [
      [/inclusiveEndDates\b/iy, T.inclusiveEndDates],
      [/includes\s[^#\n;]+/iy, T.includes],
    ],
  ],
  [
    116,
    [
      [/topAxis\b/iy, T.topAxis],
      [/tickInterval\s[^#\n;]+/iy, T.tickInterval],
      [/todayMarker\s[^\n;]+/iy, T.todayMarker],
      [/title[^\S\r\n][^\n]+/iy, T.title],
    ],
  ],
  [101, [[/excludes\s[^#\n;]+/iy, T.excludes]]],
  [115, [[/section\s[^\n]+/iy, T.section]]],
]);

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;

  const emit = (type: number, s: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(s, e));
    starts.push(s);
    p = e;
  };
  const at = (re: RegExp): number => {
    re.lastIndex = p;
    return re.test(src) ? re.lastIndex : -1;
  };
  const until = (ch: string, from: number): number => {
    const e = src.indexOf(ch, from);
    return e === -1 ? n : e;
  };

  const keyword = (c: number): boolean => {
    let e: number;
    switch (c | 32) {
      case 97:
        if ((e = at(RE_ACC_TITLE)) >= 0) {
          state = S.acc_title;
          emit(T.acc_title, p, e);
          return true;
        }
        if ((e = at(RE_ACC_DESCR)) >= 0) {
          state = S.acc_descr;
          emit(T.acc_descr, p, e);
          return true;
        }
        if ((e = at(RE_ACC_DESCR_ML)) >= 0) {
          state = S.acc_descr_multiline;
          p = e;
          return true;
        }
        break;
      case 104:
        if ((e = at(RE_HREF)) >= 0) {
          state = S.href;
          p = e;
          return true;
        }
        break;
      case 99:
        if ((e = at(RE_CALL)) >= 0) {
          state = S.callbackname;
          p = e;
          return true;
        }
        if ((e = at(RE_CLICK)) >= 0) {
          state = S.click;
          p = e;
          return true;
        }
        break;
      case 116:
        if ((e = at(RE_BARE_TITLE)) >= 0) {
          p = e;
          return true;
        }
        break;
      case 119: {
        RE_WEEKDAY.lastIndex = p;
        let m = RE_WEEKDAY.exec(src);
        if (m) {
          emit(T.weekday_monday + DAYS.indexOf(m[1].toLowerCase()), p, RE_WEEKDAY.lastIndex);
          return true;
        }
        RE_WEEKEND.lastIndex = p;
        m = RE_WEEKEND.exec(src);
        if (m) {
          emit(m[1] !== undefined ? T.weekend_friday : T.weekend_saturday, p, RE_WEEKEND.lastIndex);
          return true;
        }
        break;
      }
    }
    const rules = KEYWORDS.get(c | 32);
    if (rules) {
      for (const [re, type] of rules) {
        if ((e = at(re)) >= 0) {
          emit(type, p, e);
          return true;
        }
      }
    }
    return false;
  };

  while (true) {
    if (p >= n) {
      let type: number = T.EOF;
      switch (state) {
        case S.acc_title:
          type = T.acc_title_value;
          break;
        case S.acc_descr:
          type = T.acc_descr_value;
          break;
        case S.acc_descr_multiline:
          type = T.acc_descr_multiline_value;
          break;
        case S.href:
          type = T.href;
          break;
        case S.callbackname:
          type = T.callbackname;
          break;
        case S.callbackargs:
          type = T.callbackargs;
          break;
        case S.click:
          type = T.click;
          break;
      }
      emit(type, n, n);
      break;
    }

    const c = src.charCodeAt(p);

    switch (state) {
      case S.INITIAL: {
        const d = src.charCodeAt(p + 1);
        if (c === 37) {
          if (d !== 37) {
            let e = until('\n', p + 1);
            while (src.charCodeAt(e) === 10) e++;
            p = e;
          } else if (src.charCodeAt(p + 2) === 123) {
            // The grammar enters a lexer state it never defines, so nothing can follow.
            emit(T.open_directive, p, p + 3);
            emit(T.ERROR, p, p);
            return { types, texts, starts };
          } else {
            p = until('\n', p + 2);
          }
        } else if (c === 10) {
          let e = p + 1;
          while (src.charCodeAt(e) === 10) e++;
          emit(T.NL, p, e);
        } else if (isSpace(c)) {
          p++;
          while (p < n && isSpace(src.charCodeAt(p))) p++;
        } else if (keyword(c)) {
          break;
        } else if (c >= 48 && c <= 57 && at(RE_DATE) >= 0) {
          emit(T.date, p, p + 10);
        } else if (c !== 58) {
          let e = p + 1;
          while (e < n) {
            const ch = src.charCodeAt(e);
            if (ch === 58 || ch === 10) break;
            e++;
          }
          emit(T.taskTxt, p, e);
        } else {
          let e = p + 1;
          while (e < n) {
            const ch = src.charCodeAt(e);
            if (ch === 35 || ch === 10 || ch === 59) break;
            e++;
          }
          emit(e > p + 1 ? T.taskData : T.COLON, p, e);
        }
        break;
      }

      case S.acc_title:
      case S.acc_descr: {
        const type = state === S.acc_title ? T.acc_title_value : T.acc_descr_value;
        state = S.INITIAL;
        emit(type, p, until('\n', p));
        break;
      }

      case S.acc_descr_multiline:
        if (c === 125) {
          state = S.INITIAL;
          p++;
        } else {
          emit(T.acc_descr_multiline_value, p, until('}', p));
        }
        break;

      case S.href:
        if (c === 34) {
          state = S.INITIAL;
          p++;
        } else {
          emit(T.href, p, until('"', p));
        }
        break;

      case S.callbackname:
        if (c === 40) {
          const e = at(RE_EMPTY_CALL);
          if (e >= 0) {
            state = S.INITIAL;
            p = e;
          } else {
            state = S.callbackargs;
            p++;
          }
        } else {
          emit(T.callbackname, p, until('(', p));
        }
        break;

      case S.callbackargs:
        if (c === 41) {
          state = S.INITIAL;
          p++;
        } else {
          emit(T.callbackargs, p, until(')', p));
        }
        break;

      case S.click:
        if (isSpace(c)) {
          state = S.INITIAL;
          p++;
        } else {
          let e = p + 1;
          while (e < n && !isSpace(src.charCodeAt(e))) e++;
          emit(T.click, p, e);
        }
        break;
    }
  }

  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
