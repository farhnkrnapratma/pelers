import { PeleError, locate, syntaxError } from '../../errors.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import type { StateStmt, Stmt } from './types.js';

// The calls Mermaid's grammar makes on its `yy` object.
export interface StateBuilder {
  setRootDoc(doc: Stmt[]): void;
  trimColon(text: string): string;
  getDividerId(): string;
  setDirection(dir: string): void;
  setAccTitle(text: string): void;
  setAccDescription(text: string): void;
}

const DIRECTIONS = ['TB', 'BT', 'RL', 'LR'];
const STATEMENT =
  "'NL', 'ID', 'HIDE_EMPTY', 'scale', 'COMPOSIT_STATE', 'STATE_DESCR', 'FORK', 'JOIN', 'CHOICE', 'CONCURRENT', 'note', " +
  "'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'CLICK', 'classDef', 'style', 'class', 'direction_tb', " +
  "'direction_bt', 'direction_rl', 'direction_lr', 'EDGE_STATE'";

export function parseState(src: string, db: StateBuilder): Stmt[] {
  const { types, texts, starts, error } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) {
      if (error !== undefined) throw new PeleError(error, 'syntax', { type: 'state', ...locate(src, starts[i]) });
      throw syntaxError('state', src, starts[i], '', true);
    }
    throw syntaxError('state', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };
  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };
  const idStatement = (): StateStmt => {
    const t = types[i];
    if (t !== T.ID && t !== T.EDGE_STATE) fail("'ID', 'EDGE_STATE'");
    const id = texts[i++].trim();
    if (types[i] !== T.STYLE_SEPARATOR) return { stmt: 'state', id, type: 'default', description: '' };
    i++;
    return { stmt: 'state', id, classes: [expect(T.ID).trim()], type: 'default', description: '' };
  };

  while (types[i] === T.NL) i++;
  if (types[i] !== T.SD) fail("'NL', 'SD'");
  i++;

  let doc: Stmt[] = [];
  const outer: Stmt[][] = [];
  // The grammar drops a statement whose value is the string it uses for a blank line.
  const add = (value: Stmt): void => {
    if (value !== 'nl') doc.push(value);
  };
  const open = (id: string, description: string): void => {
    i++;
    const inner: Stmt[] = [];
    doc.push({ stmt: 'state', id, type: 'default', description, doc: inner });
    outer.push(doc);
    doc = inner;
  };

  for (;;) {
    const t = types[i];
    switch (t) {
      case T.NL:
        i++;
        break;
      case T.ID:
      case T.EDGE_STATE: {
        const first = idStatement();
        if (types[i] === T.DESCR) {
          first.description = db.trimColon(texts[i++]);
          add(first);
        } else if (types[i] === T.ARROW) {
          i++;
          const second = idStatement();
          if (types[i] === T.DESCR) {
            add({ stmt: 'relation', state1: first, state2: second, description: db.trimColon(texts[i++]) });
          } else {
            add({ stmt: 'relation', state1: first, state2: second });
          }
        } else {
          add(first);
        }
        break;
      }
      case T.HIDE_EMPTY:
        add(texts[i++]);
        break;
      case T.scale:
        i++;
        expect(T.WIDTH);
        add(texts[i - 2]);
        break;
      case T.COMPOSIT_STATE:
        if (types[i + 1] === T.STRUCT_START) open(texts[i++], '');
        else add(texts[i++]);
        break;
      case T.STATE_DESCR: {
        const description = texts[i++];
        expect(T.AS);
        const id = expect(T.ID);
        if (types[i] === T.STRUCT_START) {
          open(id, description);
        } else if (id.includes(':')) {
          const parts = id.split(':');
          add({ stmt: 'state', id: parts[0], type: 'default', description: [description.trim(), parts[1]] });
        } else {
          add({ stmt: 'state', id, type: 'default', description: description.trim() });
        }
        break;
      }
      case T.FORK:
        add({ stmt: 'state', id: texts[i++], type: 'fork' });
        break;
      case T.JOIN:
        add({ stmt: 'state', id: texts[i++], type: 'join' });
        break;
      case T.CHOICE:
        add({ stmt: 'state', id: texts[i++], type: 'choice' });
        break;
      case T.CONCURRENT:
        i++;
        add({ stmt: 'state', id: db.getDividerId(), type: 'divider' });
        break;
      case T.note: {
        const keyword = texts[i++];
        if (types[i] === T.left_of || types[i] === T.right_of) {
          const position = texts[i++].trim();
          const id = expect(T.ID).trim();
          add({ stmt: 'state', id, note: { position, text: expect(T.NOTE_TEXT).trim() } });
        } else {
          if (types[i] !== T.NOTE_TEXT) fail("'NOTE_TEXT', 'left_of', 'right_of'");
          i++;
          expect(T.AS);
          expect(T.ID);
          add(keyword);
        }
        break;
      }
      case T.direction_tb:
      case T.direction_bt:
      case T.direction_rl:
      case T.direction_lr: {
        const value = DIRECTIONS[t - T.direction_tb];
        i++;
        db.setDirection(value);
        add({ stmt: 'dir', value });
        break;
      }
      case T.acc_title: {
        i++;
        const value = expect(T.acc_title_value).trim();
        db.setAccTitle(value);
        add(value);
        break;
      }
      case T.acc_descr: {
        i++;
        const value = expect(T.acc_descr_value).trim();
        db.setAccDescription(value);
        add(value);
        break;
      }
      case T.acc_descr_multiline_value: {
        const value = texts[i++].trim();
        db.setAccDescription(value);
        add(value);
        break;
      }
      case T.CLICK: {
        i++;
        const id = idStatement();
        let url: string;
        let tooltip = '';
        if (types[i] === T.HREF) {
          i++;
          url = expect(T.STRING);
        } else {
          if (types[i] !== T.STRING) fail("'STRING', 'HREF'");
          url = texts[i++];
          tooltip = expect(T.STRING);
        }
        expect(T.NL);
        add({ stmt: 'click', id, url, tooltip });
        break;
      }
      case T.classDef: {
        i++;
        const id = expect(T.CLASSDEF_ID).trim();
        add({ stmt: 'classDef', id, classes: expect(T.CLASSDEF_STYLEOPTS).trim() });
        break;
      }
      case T.style: {
        i++;
        const id = expect(T.STYLE_IDS).trim();
        add({ stmt: 'style', id, styleClass: expect(T.STYLEDEF_STYLEOPTS).trim() });
        break;
      }
      case T.class: {
        i++;
        const id = expect(T.CLASSENTITY_IDS).trim();
        add({ stmt: 'applyClass', id, styleClass: expect(T.STYLECLASS).trim() });
        break;
      }
      case T.STRUCT_STOP:
        if (outer.length === 0) fail(`'$end', ${STATEMENT}`);
        i++;
        doc = outer.pop()!;
        break;
      case T.END:
        if (outer.length > 0) fail(`'STRUCT_STOP', ${STATEMENT}`);
        db.setRootDoc(doc);
        return doc;
      default:
        fail(outer.length > 0 ? `'STRUCT_STOP', ${STATEMENT}` : `'$end', ${STATEMENT}`);
    }
  }
}
