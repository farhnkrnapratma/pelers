import { T } from './tokens.js';
import { isSpace, isWord } from '../../util/chars.js';

// Token text is src.slice(starts[i], ends[i]); nothing is copied until the parser asks for it.
export interface Tokens {
  types: number[];
  starts: number[];
  ends: number[];
}

const enum S {
  INITIAL,
  struct,
  attribute,
  string,
  string_kv,
  string_kv_key,
  string_kv_value,
  acc,
  acc_descr_multiline,
}

const KEYWORDS = new Map<string, number>([
  ['C4Context', T.C4_CONTEXT],
  ['C4Container', T.C4_CONTAINER],
  ['C4Component', T.C4_COMPONENT],
  ['C4Dynamic', T.C4_DYNAMIC],
  ['C4Deployment', T.C4_DEPLOYMENT],
  ['Person', T.PERSON],
  ['Person_Ext', T.PERSON_EXT],
  ['System', T.SYSTEM],
  ['SystemDb', T.SYSTEM_DB],
  ['SystemQueue', T.SYSTEM_QUEUE],
  ['System_Ext', T.SYSTEM_EXT],
  ['SystemDb_Ext', T.SYSTEM_EXT_DB],
  ['SystemQueue_Ext', T.SYSTEM_EXT_QUEUE],
  ['Container', T.CONTAINER],
  ['ContainerDb', T.CONTAINER_DB],
  ['ContainerQueue', T.CONTAINER_QUEUE],
  ['Container_Ext', T.CONTAINER_EXT],
  ['ContainerDb_Ext', T.CONTAINER_EXT_DB],
  ['ContainerQueue_Ext', T.CONTAINER_EXT_QUEUE],
  ['Component', T.COMPONENT],
  ['ComponentDb', T.COMPONENT_DB],
  ['ComponentQueue', T.COMPONENT_QUEUE],
  ['Component_Ext', T.COMPONENT_EXT],
  ['ComponentDb_Ext', T.COMPONENT_EXT_DB],
  ['ComponentQueue_Ext', T.COMPONENT_EXT_QUEUE],
  ['Enterprise_Boundary', T.ENTERPRISE_BOUNDARY],
  ['System_Boundary', T.SYSTEM_BOUNDARY],
  ['Boundary', T.BOUNDARY],
  ['Container_Boundary', T.CONTAINER_BOUNDARY],
  ['Deployment_Node', T.NODE],
  ['Node', T.NODE],
  ['Node_L', T.NODE_L],
  ['Node_R', T.NODE_R],
  ['Rel', T.REL],
  ['BiRel', T.BIREL],
  ['Rel_Up', T.REL_U],
  ['Rel_U', T.REL_U],
  ['Rel_Down', T.REL_D],
  ['Rel_D', T.REL_D],
  ['Rel_Left', T.REL_L],
  ['Rel_L', T.REL_L],
  ['Rel_Right', T.REL_R],
  ['Rel_R', T.REL_R],
  ['Rel_Back', T.REL_B],
  ['RelIndex', T.REL_INDEX],
  ['UpdateElementStyle', T.UPDATE_EL_STYLE],
  ['UpdateRelStyle', T.UPDATE_REL_STYLE],
  ['UpdateLayoutConfig', T.UPDATE_LAYOUT_CONFIG],
]);

const RE_DIRECTION = [
  /.*direction\s+TB[^\n]*/y,
  /.*direction\s+BT[^\n]*/y,
  /.*direction\s+RL[^\n]*/y,
  /.*direction\s+LR[^\n]*/y,
];
const RE_LINE_END = /[\n\r\u2028\u2029]/g;
// Consume line breaks too: the grammar rejects blank statements.
const RE_BARE_TITLE = /title[^\S\r\n]*(?=[\r\n]|$)\s*/y;

// Follows the lexer Jison compiles from c4Diagram.jison: its rule order, its start conditions,
// and what it returns once the input runs out in each of them.
export function tokenize(src: string): Tokens {
  const n = src.length;
  const types: number[] = [];
  const starts: number[] = [];
  const ends: number[] = [];
  let state: number = S.INITIAL;
  let accValue: number = T.acc_title_value;
  let p = 0;
  let done = false;
  // Where the next `direction` is; -2 until the first search. The search is made inside the loop on purpose:
  // made up here, one optimized version of this function in V8 ran it again for every statement,
  // which took seconds on 50,000 characters.
  let dirNext = -2;
  let lineEnd = -1;
  let dirFailed = 0;

  const emit = (type: number, s: number, e: number): void => {
    types.push(type);
    starts.push(s);
    ends.push(e);
    p = e;
  };
  const skipWs = (q: number): number => {
    while (q < n && isSpace(src.charCodeAt(q))) q++;
    return q;
  };
  const upTo = (ch: string, from: number): number => {
    const q = src.indexOf(ch, from);
    return q === -1 ? n : q;
  };
  // `word\s[^#\n;]+`
  const statement = (word: string, type: number): boolean => {
    let q = p + word.length;
    if (!src.startsWith(word, p) || !isSpace(src.charCodeAt(q))) return false;
    const from = ++q;
    while (q < n) {
      const c = src.charCodeAt(q);
      if (c === 35 || c === 10 || c === 59) break;
      q++;
    }
    if (q === from) return false;
    emit(type, p, q);
    return true;
  };

  for (;;) {
    // Jison notes the end of input one call before it reports it, so a rule that matches
    // nothing gets one last turn.
    if (done) {
      emit(T.END, n, n);
      break;
    }
    if (p >= n) done = true;
    const c = src.charCodeAt(p);

    if (state === S.INITIAL) {
      if (p >= n) {
        emit(T.EOF, n, n);
        continue;
      }
      if (dirNext !== -1 && p >= dirFailed) {
        if (dirNext < p) dirNext = src.indexOf('direction', p);
        if (dirNext !== -1) {
          if (lineEnd < p) {
            RE_LINE_END.lastIndex = p;
            lineEnd = RE_LINE_END.test(src) ? RE_LINE_END.lastIndex - 1 : n;
          }
          if (dirNext < lineEnd) {
            let matched = false;
            for (let i = 0; i < 4 && !matched; i++) {
              RE_DIRECTION[i].lastIndex = p;
              if (RE_DIRECTION[i].test(src)) {
                emit(T.direction_tb + i, p, RE_DIRECTION[i].lastIndex);
                matched = true;
              }
            }
            if (matched) continue;
            // No direction statement starts anywhere in the rest of this line either.
            dirFailed = lineEnd;
          }
        }
      }
      if (c === 116) {
        RE_BARE_TITLE.lastIndex = p;
        if (RE_BARE_TITLE.test(src)) {
          p = RE_BARE_TITLE.lastIndex;
          continue;
        }
        if (statement('title', T.title)) continue;
      }
      if (c === 97) {
        if (statement('accDescription', T.accDescription)) continue;
        const title = src.startsWith('accTitle', p);
        if (title || src.startsWith('accDescr', p)) {
          const q = skipWs(p + 8);
          const d = src.charCodeAt(q);
          if (d === 58) {
            emit(title ? T.acc_title : T.acc_descr, p, skipWs(q + 1));
            accValue = title ? T.acc_title_value : T.acc_descr_value;
            state = S.acc;
            continue;
          }
          if (d === 123 && !title) {
            p = skipWs(q + 1);
            state = S.acc_descr_multiline;
            continue;
          }
        }
      }
      if (c === 37 && src.charCodeAt(p + 1) === 37) {
        // A comment takes the line breaks after it, so no NEWLINE follows.
        let q = upTo('\n', p + 2);
        while (q < n && (src.charCodeAt(q) === 10 || src.charCodeAt(q) === 13)) q++;
        p = q;
        continue;
      }
      if (isSpace(c)) {
        let q = p;
        let last = -1;
        while (q < n) {
          const d = src.charCodeAt(q);
          if (!isSpace(d)) break;
          if (d === 10) last = q;
          q++;
        }
        if (last !== -1) emit(T.NEWLINE, p, last + 1);
        else p = q;
        continue;
      }
      if (c === 123) {
        emit(T.LBRACE, p, p + 1);
        continue;
      }
      if (c === 125) {
        emit(T.RBRACE, p, p + 1);
        continue;
      }
      if (isWord(c)) {
        let q = p + 1;
        while (q < n && isWord(src.charCodeAt(q))) q++;
        const type = q - p <= 19 ? KEYWORDS.get(src.slice(p, q)) : undefined;
        if (type !== undefined) {
          if (type > T.C4_DEPLOYMENT) state = S.struct;
          emit(type, p, q);
          continue;
        }
      }
      emit(T.ERROR, p, p);
      break;
    }

    if (state === S.struct) {
      if (p >= n) {
        emit(T.EOF_IN_STRUCT, n, n);
      } else if (c === 40) {
        let q = p + 1;
        while (src.charCodeAt(q) === 32) q++;
        if (src.charCodeAt(q) === 44) emit(T.ATTRIBUTE_EMPTY, p, q + 1);
        else p++;
        state = S.attribute;
      } else if (c === 41) {
        p++;
        state = S.INITIAL;
      } else {
        emit(T.ERROR, p, p);
        break;
      }
      continue;
    }

    if (state === S.attribute) {
      if (p >= n) continue;
      if (c === 41) {
        p++;
        state = S.INITIAL;
      } else if (c === 44) {
        if (src.charCodeAt(p + 1) === 44) emit(T.ATTRIBUTE_EMPTY, p, p + 2);
        else p++;
      } else {
        let q = p;
        while (src.charCodeAt(q) === 32) q++;
        const d = src.charCodeAt(q);
        if (d === 34) {
          if (src.charCodeAt(q + 1) === 34) {
            emit(T.ATTRIBUTE_EMPTY, p, q + 2);
          } else {
            p = q + 1;
            state = S.string;
          }
        } else if (d === 36) {
          p = q + 1;
          state = S.string_kv;
        } else {
          // An unquoted value runs to the next comma, through closing parentheses and line breaks.
          emit(T.STR, p, upTo(',', p));
        }
      }
      continue;
    }

    if (state === S.string) {
      if (c === 34) {
        p++;
        state = S.attribute;
      } else {
        emit(T.STR, p, upTo('"', p));
      }
      continue;
    }

    if (state === S.string_kv) {
      emit(T.STR_KEY, p, upTo('=', p));
      state = S.string_kv_key;
      continue;
    }

    if (state === S.string_kv_key) {
      if (p >= n) continue;
      let q = p + 1;
      if (c === 61) while (src.charCodeAt(q) === 32) q++;
      if (c !== 61 || src.charCodeAt(q) !== 34) {
        emit(T.ERROR, p, p);
        break;
      }
      p = q + 1;
      state = S.string_kv_value;
      continue;
    }

    if (state === S.string_kv_value) {
      if (p >= n) continue;
      if (c === 34) {
        p++;
        state = S.attribute;
      } else {
        emit(T.STR_VALUE, p, upTo('"', p));
      }
      continue;
    }

    if (state === S.acc) {
      emit(accValue, p, upTo('\n', p));
      state = S.INITIAL;
      continue;
    }

    if (c === 125) {
      p++;
      state = S.INITIAL;
    } else {
      emit(T.acc_descr_multiline_value, p, upTo('}', p));
    }
  }

  return { types, starts, ends };
}
