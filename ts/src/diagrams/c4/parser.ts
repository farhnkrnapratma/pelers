import { syntaxError } from '../../errors.js';
import type { C4Db } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import type { C4Attr } from './types.js';

// Mermaid's typeC4Shape for each shape keyword, indexed from T.PERSON.
const KINDS = [
  'person',
  'external_person',
  'system',
  'system_db',
  'system_queue',
  'external_system',
  'external_system_db',
  'external_system_queue',
  'container',
  'container_db',
  'container_queue',
  'external_container',
  'external_container_db',
  'external_container_queue',
  'component',
  'component_db',
  'component_queue',
  'external_component',
  'external_component_db',
  'external_component_queue',
];
const REL_TYPES = ['rel', 'birel', 'rel_u', 'rel_d', 'rel_l', 'rel_r', 'rel_b', 'rel'];
const NODE_TYPES = ['node', 'nodeL', 'nodeR'];

const HEADERS =
  "'direction_tb', 'direction_bt', 'direction_rl', 'direction_lr', 'C4_CONTEXT', 'C4_CONTAINER', 'C4_COMPONENT', 'C4_DYNAMIC', 'C4_DEPLOYMENT'";
const ATTRIBUTES = "'STR', 'STR_KEY', 'ATTRIBUTE', 'ATTRIBUTE_EMPTY'";

const enum P {
  // A statement is required.
  Item,
  // Only a diagram statement may come next: the first one in a boundary, or one written straight after a title.
  Diagram,
  ItemOrEnd,
  AfterOther,
  AfterDiagram,
}

function isOther(t: number): boolean {
  return (
    t === T.title ||
    t === T.accDescription ||
    t === T.acc_title ||
    t === T.acc_descr ||
    t === T.acc_descr_multiline_value
  );
}

function isDiagram(t: number): boolean {
  return t >= T.PERSON && t <= T.UPDATE_LAYOUT_CONFIG;
}

// Accepts what the parser Jison generates from c4Diagram.jison accepts, and calls the model
// builder in the order its semantic actions do. Boundaries nest through a counter, not recursion.
export function parseC4(src: string, db: C4Db): void {
  const { types, starts, ends } = tokenize(src);
  let i = 0;

  const text = (at: number): string => src.slice(starts[at], ends[at]);
  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('c4', src, starts[i], '', true);
    throw syntaxError('c4', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };
  const expect = (type: number): void => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    i++;
  };

  const attributes = (): C4Attr[] => {
    const attrs: C4Attr[] = [];
    for (;;) {
      const t = types[i];
      if (t === T.STR) {
        attrs.push(text(i++).trim());
      } else if (t === T.ATTRIBUTE_EMPTY) {
        attrs.push('');
        i++;
      } else if (t === T.STR_KEY) {
        i++;
        if (types[i] !== T.STR_VALUE) fail("'STR_VALUE'");
        attrs.push({ key: text(i - 1).trim(), value: text(i).trim() });
        i++;
      } else {
        break;
      }
    }
    if (attrs.length === 0) fail(ATTRIBUTES);
    return attrs;
  };

  const other = (t: number): void => {
    if (t === T.title) {
      db.setTitle(text(i++).substring(6));
    } else if (t === T.accDescription) {
      db.setAccDescription(text(i++).substring(15));
    } else if (t === T.acc_descr_multiline_value) {
      db.setAccDescription(text(i++).trim());
    } else {
      i++;
      if (t === T.acc_title) {
        if (types[i] !== T.acc_title_value) fail("'acc_title_value'");
        // The grammar hands `accTitle:` to setTitle, so it becomes the visible title.
        db.setTitle(text(i++).trim());
      } else {
        if (types[i] !== T.acc_descr_value) fail("'acc_descr_value'");
        db.setAccDescription(text(i++).trim());
      }
    }
  };

  // Returns true when the statement opens a boundary.
  const diagram = (t: number): boolean => {
    i++;
    const attrs = attributes();
    if (t <= T.SYSTEM_EXT_QUEUE) {
      db.addPersonOrSystem(KINDS[t - T.PERSON], attrs);
    } else if (t <= T.CONTAINER_EXT_QUEUE) {
      db.addContainer(KINDS[t - T.PERSON], attrs);
    } else if (t <= T.COMPONENT_EXT_QUEUE) {
      db.addComponent(KINDS[t - T.PERSON], attrs);
    } else if (t <= T.NODE_R) {
      if (t === T.ENTERPRISE_BOUNDARY) attrs.splice(2, 0, 'ENTERPRISE');
      else if (t === T.SYSTEM_BOUNDARY) attrs.splice(2, 0, 'SYSTEM');
      else if (t === T.CONTAINER_BOUNDARY) attrs.splice(2, 0, 'CONTAINER');
      if (t >= T.NODE) db.addDeploymentNode(NODE_TYPES[t - T.NODE], attrs);
      else if (t === T.CONTAINER_BOUNDARY) db.addContainerBoundary(attrs);
      else db.addPersonOrSystemBoundary(attrs);
      if (types[i] === T.LBRACE) {
        i++;
        expect(T.NEWLINE);
      } else if (types[i] === T.NEWLINE) {
        i++;
        expect(T.LBRACE);
        if (types[i] === T.NEWLINE) i++;
      } else {
        fail("'NEWLINE', 'LBRACE'");
      }
      return true;
    } else if (t <= T.REL_INDEX) {
      // RelIndex takes a sequence number first, which Mermaid ignores.
      if (t === T.REL_INDEX) attrs.splice(0, 1);
      db.addRel(REL_TYPES[t - T.REL], attrs);
    } else if (t === T.UPDATE_EL_STYLE) {
      db.updateElStyle('update_el_style', attrs);
    } else if (t === T.UPDATE_REL_STYLE) {
      db.updateRelStyle('update_rel_style', attrs);
    } else {
      db.updateLayoutConfig('update_layout_config', attrs);
    }
    return false;
  };

  const header = types[i];
  if (header < T.C4_CONTEXT || header > T.C4_DEPLOYMENT) fail(HEADERS);
  i++;
  expect(T.NEWLINE);

  let depth = 0;
  let state: number = P.Item;
  for (;;) {
    const t = types[i];
    if (state === P.AfterOther || state === P.AfterDiagram) {
      if (t === T.NEWLINE) {
        i++;
        state = P.ItemOrEnd;
        continue;
      }
      if (state === P.AfterOther && isDiagram(t)) {
        state = P.Diagram;
        continue;
      }
    } else if (isDiagram(t)) {
      if (diagram(t)) {
        depth++;
        state = P.Diagram;
      } else {
        state = P.AfterDiagram;
      }
      continue;
    } else if (isOther(t) && state !== P.Diagram) {
      other(t);
      state = P.AfterOther;
      continue;
    } else if (state !== P.ItemOrEnd) {
      fail("'title', 'accDescription', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'PERSON', 'REL'");
    }

    if (depth > 0) {
      if (t !== T.RBRACE) fail("'NEWLINE', 'RBRACE'");
      i++;
      db.popBoundaryParseStack();
      depth--;
      state = P.AfterDiagram;
    } else {
      if (t !== T.EOF) fail("'NEWLINE', 'EOF'");
      i++;
      expect(T.END);
      db.setC4Type(text(0));
      return;
    }
  }
}
