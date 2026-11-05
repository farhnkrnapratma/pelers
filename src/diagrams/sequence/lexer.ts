import { T } from './tokens.js';
import { isDigit, isSpace, isWord } from '../../util/chars.js';

export interface Tokens {
  types: number[];
  texts: string[];
  starts: number[];
}

const enum S {
  INITIAL,
  ID,
  ALIAS,
  LINE,
  CONFIG,
  acc_title,
  acc_descr,
  acc_descr_multiline,
}

const enum C {
  ActorStart = 1,
  ActorRest = 2,
  Id = 4,
  IdWide = 8,
  Line = 16,
  Menu = 32,
}

// Character classes of Mermaid's lexer rules, for ASCII. Other characters are tested by isSpace.
const CLASS = new Uint8Array(128);
for (let c = 0; c < 128; c++) {
  const ch = String.fromCharCode(c);
  const ws = c === 32 || (c >= 9 && c <= 13);
  let bits = 0;
  if (!'/\\+()<->:\n,;'.includes(ch)) bits |= C.ActorStart;
  if (!'+<->:\n,;'.includes(ch)) bits |= C.ActorRest;
  if (!'<>:\n,;@'.includes(ch)) bits |= C.IdWide | (ws ? 0 : C.Id);
  if (!'#\n;'.includes(ch)) bits |= C.Line;
  if (!'/\\+()<>:\n,;-'.includes(ch) && !ws) bits |= C.Menu;
  CLASS[c] = bits;
}

export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const texts: string[] = [];
  const starts: number[] = [];
  const stack: number[] = [];
  let state: number = S.INITIAL;
  let p = 0;
  let nextNl = -1;

  const emit = (type: number, s: number, e: number): void => {
    types.push(type);
    texts.push(src.slice(s, e));
    starts.push(s);
    p = e;
  };
  const emitText = (type: number, s: number, e: number, text: string): void => {
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
  const eol = (q: number): number => {
    if (nextNl < q) {
      nextNl = src.indexOf('\n', q);
      if (nextNl === -1) nextNl = n;
    }
    return nextNl;
  };
  const has = (q: number, bit: number): boolean => {
    const c = src.charCodeAt(q);
    if (c < 128) return (CLASS[c] & bit) !== 0;
    return bit === C.Id || bit === C.Menu ? !isSpace(c) : true;
  };
  // Matches a lowercase literal without regard to the case of the source.
  const ci = (q: number, word: string): boolean => {
    for (let k = 0; k < word.length; k++) {
      const w = word.charCodeAt(k);
      const c = src.charCodeAt(q + k);
      if (w >= 97 && w <= 122 ? (c | 32) !== w : c !== w) return false;
    }
    return true;
  };
  const word = (lit: string): boolean => ci(p, lit) && !isWord(src.charCodeAt(p + lit.length));
  const skipWs = (q: number): number => {
    while (q < n && isSpace(src.charCodeAt(q))) q++;
    return q;
  };
  const skipSameLine = (): boolean => {
    let q = p;
    while (q < n) {
      const c = src.charCodeAt(q);
      if (c === 10 || !isSpace(c)) break;
      q++;
    }
    if (q === p) return false;
    p = q;
    return true;
  };
  const lineRun = (q: number): number => {
    while (q < n && has(q, C.Line)) q++;
    return q;
  };
  const begin = (type: number, len: number, next: number): void => {
    emit(type, p, p + len);
    push(next);
  };
  const keyword = (lit: string, type: number, next = -1): boolean => {
    if (!word(lit)) return false;
    emit(type, p, p + lit.length);
    if (next >= 0) push(next);
    return true;
  };
  // The actor menu keywords count only when spaces and the start of a name follow.
  const menu = (lit: string, type: number): boolean => {
    if (!ci(p, lit)) return false;
    let q = p + lit.length;
    let c = src.charCodeAt(q);
    if (c !== 32 && c !== 9) return false;
    while (c === 32 || c === 9) c = src.charCodeAt(++q);
    if (q >= n || !has(q, C.Menu)) return false;
    emit(type, p, p + lit.length);
    return true;
  };

  const arrowAhead = (q: number): boolean => {
    const c = src.charCodeAt(q);
    const d = src.charCodeAt(q + 1);
    if (c === 45) {
      if (d === 120 || d === 88 || d === 41 || d === 92 || d === 47 || d === 45) return true;
      const e = src.charCodeAt(q + 2);
      return d === 124 && (e === 92 || e === 47);
    }
    if (c === 47 || c === 92) return (d === 124 || d === c) && src.charCodeAt(q + 2) === 45;
    return c === 40 && d === 41;
  };

  const actor = (): void => {
    let q = p + 1;
    while (q < n && has(q, C.ActorStart)) q++;
    while (q < n && !arrowAhead(q)) {
      let r = q;
      while (src.charCodeAt(r) === 45) r++;
      if (r >= n || !has(r, C.ActorRest)) break;
      r++;
      while (r < n && has(r, C.ActorRest)) r++;
      q = r;
    }
    emitText(T.ACTOR, p, q, src.slice(p, q).trim());
  };

  const number = (): boolean => {
    let q = p;
    while (isDigit(src.charCodeAt(q))) q++;
    const whole = q > p;
    if (src.charCodeAt(q) === 46 && isDigit(src.charCodeAt(q + 1))) {
      const r = isDigit(src.charCodeAt(q + 2)) ? q + 3 : q + 2;
      const after = src.charCodeAt(r);
      if (after === 32 || after === 10) {
        emit(T.NUM, p, r);
        return true;
      }
    }
    const after = src.charCodeAt(q);
    if (whole && (after === 32 || after === 10)) {
      emit(T.NUM, p, q);
      return true;
    }
    return false;
  };

  const dash = (): void => {
    const d = src.charCodeAt(p + 1);
    const e = src.charCodeAt(p + 2);
    if (d === 62) emit(e === 62 ? T.SOLID_ARROW : T.SOLID_OPEN_ARROW, p, p + (e === 62 ? 3 : 2));
    else if (d === 120 || d === 88) emit(T.SOLID_CROSS, p, p + 2);
    else if (d === 41) emit(T.SOLID_POINT, p, p + 2);
    else if (d === 124 && e === 92) emit(T.SOLID_ARROW_TOP, p, p + 3);
    else if (d === 124 && e === 47) emit(T.SOLID_ARROW_BOTTOM, p, p + 3);
    else if (d === 92 && e === 92) emit(T.STICK_ARROW_TOP, p, p + 3);
    else if (d === 47 && e === 47) emit(T.STICK_ARROW_BOTTOM, p, p + 3);
    else if (d === 45) {
      const f = src.charCodeAt(p + 3);
      if (e === 62) emit(f === 62 ? T.DOTTED_ARROW : T.DOTTED_OPEN_ARROW, p, p + (f === 62 ? 4 : 3));
      else if (e === 120 || e === 88) emit(T.DOTTED_CROSS, p, p + 3);
      else if (e === 41) emit(T.DOTTED_POINT, p, p + 3);
      else if (e === 124 && f === 92) emit(T.SOLID_ARROW_TOP_DOTTED, p, p + 4);
      else if (e === 124 && f === 47) emit(T.SOLID_ARROW_BOTTOM_DOTTED, p, p + 4);
      else if (e === 92 && f === 92) emit(T.STICK_ARROW_TOP_DOTTED, p, p + 4);
      else if (e === 47 && f === 47) emit(T.STICK_ARROW_BOTTOM_DOTTED, p, p + 4);
      else emit(T.MINUS, p, p + 1);
    } else emit(T.MINUS, p, p + 1);
  };

  const reverse = (c: number): boolean => {
    const d = src.charCodeAt(p + 1);
    if ((d !== 124 && d !== c) || src.charCodeAt(p + 2) !== 45) return false;
    const dotted = src.charCodeAt(p + 3) === 45;
    const top = c === 47;
    let type: number;
    if (d === 124) {
      if (dotted) type = top ? T.SOLID_ARROW_TOP_REVERSE_DOTTED : T.SOLID_ARROW_BOTTOM_REVERSE_DOTTED;
      else type = top ? T.SOLID_ARROW_TOP_REVERSE : T.SOLID_ARROW_BOTTOM_REVERSE;
    } else if (dotted) type = top ? T.STICK_ARROW_TOP_REVERSE_DOTTED : T.STICK_ARROW_BOTTOM_REVERSE_DOTTED;
    else type = top ? T.STICK_ARROW_TOP_REVERSE : T.STICK_ARROW_BOTTOM_REVERSE;
    emit(type, p, p + (dotted ? 4 : 3));
    return true;
  };

  const title = (): boolean => {
    if (!ci(p, 'title')) return false;
    let q = p + 5;
    const legacy = src.charCodeAt(q) === 58;
    if (legacy) q++;
    let e = q;
    while (e < n && src.charCodeAt(e) !== 10 && isSpace(src.charCodeAt(e))) e++;
    if (e === n || src.charCodeAt(e) === 10) {
      p = e;
      return true;
    }
    const sep = src.charCodeAt(q);
    if (!isSpace(sep) || sep === 13 || !has(q + 1, C.Line)) return false;
    emit(legacy ? T.legacy_title : T.title, p, lineRun(q + 1));
    return true;
  };

  const acc = (): boolean => {
    const descr = ci(p, 'accdescr');
    if (!descr && !ci(p, 'acctitle')) return false;
    const q = skipWs(p + 8);
    const c = src.charCodeAt(q);
    if (c === 58) {
      emit(descr ? T.acc_descr : T.acc_title, p, skipWs(q + 1));
      push(descr ? S.acc_descr : S.acc_title);
      return true;
    }
    if (descr && c === 123) {
      p = skipWs(q + 1);
      push(S.acc_descr_multiline);
      return true;
    }
    return false;
  };

  const keywords = (c: number): boolean => {
    switch (c | 32) {
      case 97:
        return (
          keyword('actor', T.participant_actor, S.ID) ||
          keyword('alt', T.alt, S.LINE) ||
          keyword('and', T.and, S.LINE) ||
          keyword('activate', T.activate, S.ID) ||
          acc() ||
          keyword('autonumber', T.autonumber)
        );
      case 98:
        return keyword('box', T.box, S.LINE) || keyword('break', T.break, S.LINE);
      case 99:
        return keyword('create', T.create) || keyword('critical', T.critical, S.LINE);
      case 100:
        return (
          keyword('destroy', T.destroy, S.ID) || menu('details', T.details) || keyword('deactivate', T.deactivate, S.ID)
        );
      case 101:
        return keyword('else', T.else, S.LINE) || keyword('end', T.end);
      case 108:
        return (
          keyword('loop', T.loop, S.LINE) ||
          keyword('left of', T.left_of) ||
          menu('links', T.links) ||
          menu('link', T.link)
        );
      case 110:
        return keyword('note', T.note);
      case 111:
        return (
          keyword('opt', T.opt, S.LINE) ||
          keyword('option', T.option, S.LINE) ||
          keyword('over', T.over) ||
          keyword('off', T.off)
        );
      case 112:
        return (
          keyword('participant', T.participant, S.ID) ||
          keyword('par', T.par, S.LINE) ||
          keyword('par_over', T.par_over, S.LINE) ||
          menu('properties', T.properties)
        );
      case 114:
        return keyword('rect', T.rect, S.LINE) || keyword('right of', T.right_of);
      case 115:
        return keyword('sequencediagram', T.SD);
      case 116:
        return title();
      default:
        return false;
    }
  };

  const initial = (): void => {
    const c = src.charCodeAt(p);
    if (c === 10) {
      let q = p + 1;
      while (src.charCodeAt(q) === 10) q++;
      emit(T.NEWLINE, p, q);
      return;
    }
    if (isSpace(c)) {
      p = skipWs(p + 1);
      return;
    }
    if (c === 35 || (c === 37 && src.charCodeAt(p + 1) !== 123)) {
      p = eol(p);
      return;
    }
    if ((isDigit(c) || c === 46) && number()) return;
    if (c < 128 && (c | 32) >= 97 && (c | 32) <= 122 && keywords(c)) return;
    if (c === 44) emit(T.COMMA, p, p + 1);
    else if (c === 59) emit(T.NEWLINE, p, p + 1);
    else if (has(p, C.ActorStart)) actor();
    else if (c === 45) dash();
    else if (c === 60 && src.startsWith('<<->>', p)) emit(T.BIDIRECTIONAL_SOLID_ARROW, p, p + 5);
    else if (c === 60 && src.startsWith('<<-->>', p)) emit(T.BIDIRECTIONAL_DOTTED_ARROW, p, p + 6);
    else if ((c === 47 || c === 92) && reverse(c)) return;
    else if (c === 58) emit(T.TXT, p, lineRun(p + 1));
    else if (c === 43) emit(T.PLUS, p, p + 1);
    else if (c === 40 && src.charCodeAt(p + 1) === 41) emit(T.CENTRAL, p, p + 2);
    else emit(T.INVALID, p, p + 1);
  };

  // A participant name, up to an alias, a config object, or the end of the statement.
  const id = (): boolean => {
    if (skipSameLine()) return true;
    const c = src.charCodeAt(p);
    if (c === 35) {
      p = eol(p);
      return true;
    }
    if (c === 64 && src.charCodeAt(p + 1) === 123) {
      begin(T.CONFIG_START, 2, S.CONFIG);
      return true;
    }
    let a = p;
    while (a < n && has(a, C.Id)) a++;
    if (a > p) {
      const w = skipWs(a);
      if (src.charCodeAt(w) === 64 && src.charCodeAt(w + 1) === 123) {
        emit(T.ACTOR, p, a);
        return true;
      }
      if (w > a && ci(w, 'as') && isSpace(src.charCodeAt(w + 2))) {
        emit(T.ACTOR, p, a);
        push(S.ALIAS);
        return true;
      }
    }
    let b = a;
    while (b < n && has(b, C.IdWide)) b++;
    if (b > p) {
      const end = src.charCodeAt(b);
      // Before any other delimiter the name must be followed by a comment, so it ends at the last one.
      let cut = b;
      if (b < n && end !== 10 && end !== 59) {
        cut--;
        while (cut > p && src.charCodeAt(cut) !== 35) cut--;
      }
      if (cut > p) {
        emitText(T.ACTOR, p, cut, src.slice(p, cut).trim());
        pop();
        return true;
      }
    }
    if (src.charCodeAt(b) === 60) {
      emit(T.INVALID, p, eol(p));
      pop();
      return true;
    }
    if (c !== 10) {
      const e = eol(p);
      emitText(T.INVALID, p, e, src.slice(p, e).trim());
      pop();
      return true;
    }
    return false;
  };

  const step = (): boolean => {
    switch (state) {
      case S.INITIAL:
        initial();
        return true;
      case S.ID:
        return id();
      case S.ALIAS:
        if (skipSameLine()) return true;
        if (src.charCodeAt(p) === 35) {
          p = eol(p);
          return true;
        }
        pop();
        pop();
        if (word('as')) begin(T.AS, 2, S.LINE);
        else emit(T.NEWLINE, p, p);
        return true;
      case S.LINE:
        if (skipSameLine()) return true;
        if (src.charCodeAt(p) === 35) {
          p = eol(p);
          return true;
        }
        emit(T.restOfLine, p, lineRun(p));
        pop();
        return true;
      case S.CONFIG:
        if (src.charCodeAt(p) !== 125) {
          let q = src.indexOf('}', p);
          if (q === -1) q = n;
          emit(T.CONFIG_CONTENT, p, q);
          return true;
        } else {
          const w = skipWs(p + 1);
          const alias = w > p + 1 && ci(w, 'as') && isSpace(src.charCodeAt(w + 2));
          emit(T.CONFIG_END, p, p + 1);
          pop();
          if (alias) push(S.ALIAS);
          else pop();
          return true;
        }
      case S.acc_title:
      case S.acc_descr:
        emit(state === S.acc_title ? T.acc_title_value : T.acc_descr_value, p, eol(p));
        pop();
        return true;
      default:
        if (src.charCodeAt(p) === 125) {
          p++;
          pop();
        } else {
          let q = src.indexOf('}', p);
          if (q === -1) q = n;
          emit(T.acc_descr_multiline_value, p, q);
        }
        return true;
    }
  };

  while (p < n) {
    if (!step()) {
      types.push(T.ERROR);
      texts.push('');
      starts.push(p);
      return { types, texts, starts };
    }
  }

  // Mermaid's lexer tries its rules once more on the empty remainder before reporting the end.
  if (state === S.INITIAL || state === S.ALIAS) emit(T.NEWLINE, n, n);
  else if (state === S.LINE) emit(T.restOfLine, n, n);
  else if (state === S.acc_title) emit(T.acc_title_value, n, n);
  else if (state === S.acc_descr) emit(T.acc_descr_value, n, n);
  else if (state === S.acc_descr_multiline) emit(T.acc_descr_multiline_value, n, n);
  types.push(T.END);
  texts.push('');
  starts.push(n);
  return { types, texts, starts };
}
