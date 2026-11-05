import { syntaxError } from '../../errors.js';
import type { ErDb } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import type { Cardinality, DocItem, Identification, ParsedAttribute, RelSpec } from './types.js';

const CARDINALITY = new Map<number, Cardinality>([
  [T.ZERO_OR_ONE, 'ZERO_OR_ONE'],
  [T.ZERO_OR_MORE, 'ZERO_OR_MORE'],
  [T.ONE_OR_MORE, 'ONE_OR_MORE'],
  [T.ONLY_ONE, 'ONLY_ONE'],
  [T.MD_PARENT, 'MD_PARENT'],
]);

const DIRECTIONS = ['TB', 'BT', 'RL', 'LR'];

const NAME = "'UNICODE_TEXT', 'NUM', 'ENTITY_NAME', 'DECIMAL_NUM', 'ENTITY_ONE'";
const CARDS = "'ZERO_OR_ONE', 'ZERO_OR_MORE', 'ONE_OR_MORE', 'ONLY_ONE', 'MD_PARENT'";
const LINE =
  "'EOF', 'NEWLINE', 'title', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'SUBGRAPH', 'direction_tb', " +
  "'direction_bt', 'direction_rl', 'direction_lr', 'CLASSDEF', 'CLASS', 'STYLE', " +
  NAME;

interface Frame {
  id: string;
  title: string;
  // What Mermaid's grammar hands to addSubGraph: the value of every statement inside. Besides entity
  // names that includes accessibility text and the keywords `style`, `class` and `classDef`.
  list: DocItem[];
}

export function parseEr(src: string, db: ErDb): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('er', src, starts[i], '', true);
    throw syntaxError('er', src, starts[i], `Expecting ${expected}, got '${t === T.CHAR ? texts[i] : TOKEN_NAMES[t]}'`);
  };

  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };

  const separator = (): void => {
    const t = types[i];
    if (t !== T.NEWLINE && t !== T.SEMI && t !== T.EOF) fail("'EOF', 'NEWLINE', 'SEMI'");
    i++;
  };

  const isName = (t: number): boolean =>
    t === T.UNICODE_TEXT || t === T.ENTITY_NAME || t === T.NUM || t === T.DECIMAL_NUM || t === T.ENTITY_ONE;

  const entityName = (): string => {
    const t = types[i];
    if (!isName(t)) fail(NAME);
    const text = texts[i++];
    return t === T.ENTITY_NAME ? text.slice(1, -1) : text;
  };

  const idList = (): string[] => {
    const ids: string[] = [];
    for (;;) {
      const t = types[i];
      if (t !== T.UNICODE_TEXT && t !== T.STYLE_TEXT) fail("'UNICODE_TEXT', 'STYLE_TEXT'");
      ids.push(texts[i++]);
      if (types[i] !== T.COMMA) return ids;
      i++;
    }
  };

  const isStyleComponent = (t: number): boolean => t === T.STYLE_TEXT || t === T.NUM || t === T.COLON || t === T.BRKT;

  const stylesOpt = (): string[] => {
    const styles: string[] = [];
    for (;;) {
      if (!isStyleComponent(types[i])) fail("'STYLE_TEXT', 'COLON', 'NUM', 'BRKT'");
      let style = texts[i++];
      while (isStyleComponent(types[i])) style += texts[i++];
      styles.push(style);
      if (types[i] !== T.COMMA) return styles;
      i++;
    }
  };

  const relSpec = (): RelSpec => {
    const cardB = CARDINALITY.get(types[i++])!;
    const r = types[i];
    if (r !== T.IDENTIFYING && r !== T.NON_IDENTIFYING) fail("'NON_IDENTIFYING', 'IDENTIFYING'");
    const relType: Identification = r === T.IDENTIFYING ? 'IDENTIFYING' : 'NON_IDENTIFYING';
    i++;
    const cardA = CARDINALITY.get(types[i]);
    if (cardA === undefined) return fail(CARDS);
    i++;
    return { cardA, relType, cardB };
  };

  const role = (): string => {
    const t = types[i];
    if (t !== T.WORD && t !== T.ENTITY_NAME && t !== T.UNICODE_TEXT) fail("'UNICODE_TEXT', 'ENTITY_NAME', 'WORD'");
    const text = texts[i++];
    return t === T.UNICODE_TEXT ? text : text.slice(1, -1);
  };

  const isChar = (ch: string): boolean => types[i] === T.CHAR && texts[i] === ch;

  // Returns the attributes last to first, as Mermaid's right-recursive rule builds them.
  const attributes = (): ParsedAttribute[] => {
    const list: ParsedAttribute[] = [];
    do {
      let type = expect(T.ATTRIBUTE_WORD);
      if (isChar('?')) type += texts[i++];
      if (types[i] !== T.ATTRIBUTE_WORD) fail(type.endsWith('?') ? "'ATTRIBUTE_WORD'" : "'ATTRIBUTE_WORD', '?'");
      const attribute: ParsedAttribute = { type, name: texts[i++] };
      if (types[i] === T.ATTRIBUTE_KEY) {
        const keys = [texts[i++]];
        while (isChar(',')) {
          i++;
          keys.push(expect(T.ATTRIBUTE_KEY));
        }
        attribute.keys = keys;
      }
      if (types[i] === T.COMMENT) attribute.comment = texts[i++].slice(1, -1);
      list.push(attribute);
    } while (types[i] !== T.BLOCK_STOP);
    return list.reverse();
  };

  // Parses an optional `{ ... }` block; undefined when there is none or it is empty.
  const block = (): ParsedAttribute[] | undefined => {
    if (types[i] !== T.BLOCK_START) return undefined;
    i++;
    if (types[i] === T.BLOCK_STOP) {
      i++;
      return undefined;
    }
    const list = attributes();
    i++;
    return list;
  };

  const entityStatement = (): string[] => {
    const a = entityName();
    let t = types[i];
    if (t === T.SQS) {
      i++;
      const alias = entityName();
      expect(T.SQE);
      let classes: string[] | undefined;
      if (types[i] === T.STYLE_SEPARATOR) {
        i++;
        classes = idList();
      }
      const attrs = block();
      db.addEntity(a, alias);
      if (attrs) db.addAttributes(a, attrs);
      if (classes) db.setClass([a], classes);
      return [a];
    }
    let classes: string[] | undefined;
    if (t === T.STYLE_SEPARATOR) {
      i++;
      classes = idList();
      t = types[i];
    }
    if (CARDINALITY.has(t)) {
      const spec = relSpec();
      const b = entityName();
      let classesB: string[] | undefined;
      if (types[i] === T.STYLE_SEPARATOR) {
        i++;
        classesB = idList();
      }
      expect(T.COLON);
      const label = role();
      db.addEntity(a);
      db.addEntity(b);
      db.addRelationship(a, label, b, spec);
      if (classes) db.setClass([a], classes);
      if (classesB) db.setClass([b], classesB);
      return [a, b];
    }
    const attrs = block();
    db.addEntity(a);
    if (attrs) db.addAttributes(a, attrs);
    if (classes) db.setClass([a], classes);
    return [a];
  };

  expect(T.ER_DIAGRAM);
  // Subgraphs nest on this stack rather than on the call stack; the first frame is the diagram itself.
  const frames: Frame[] = [{ id: '', title: '', list: [] }];
  let frame = frames[0];
  let nested = false;

  for (;;) {
    const t = types[i];
    switch (t) {
      case T.NEWLINE:
        i++;
        break;
      case T.EOF:
        i++;
        if (nested) fail(`'END', ${LINE}`);
        return;
      case T.END_KW: {
        if (!nested) fail(LINE);
        i++;
        const id = db.addSubGraph({ text: frame.id }, frame.list, { text: frame.title });
        frames.pop();
        frame = frames[frames.length - 1];
        nested = frames.length > 1;
        if (nested) frame.list.push(id);
        break;
      }
      case T.SUBGRAPH: {
        i++;
        const id = entityName();
        let title = id;
        if (types[i] === T.SQS) {
          i++;
          title = entityName();
          while (isName(types[i])) title += ' ' + entityName();
          expect(T.SQE);
        }
        separator();
        frame = { id, title, list: [] };
        frames.push(frame);
        nested = true;
        break;
      }
      case T.acc_title: {
        i++;
        const value = expect(T.acc_title_value).trim();
        db.setAccTitle(value);
        if (nested) frame.list.push(value);
        break;
      }
      case T.acc_descr: {
        i++;
        const value = expect(T.acc_descr_value).trim();
        db.setAccDescription(value);
        if (nested) frame.list.push(value);
        break;
      }
      case T.acc_descr_multiline_value: {
        const value = texts[i++].trim();
        db.setAccDescription(value);
        if (nested) frame.list.push(value);
        break;
      }
      case T.direction_tb:
      case T.direction_bt:
      case T.direction_rl:
      case T.direction_lr: {
        i++;
        const value = DIRECTIONS[t - T.direction_tb];
        if (nested) frame.list.push({ stmt: 'dir', value });
        else db.setDirection(value);
        break;
      }
      case T.CLASSDEF: {
        const keyword = texts[i++];
        const ids = idList();
        const styles = stylesOpt();
        separator();
        db.addClass(ids, styles);
        if (nested) frame.list.push(keyword);
        break;
      }
      case T.CLASS: {
        const keyword = texts[i++];
        const ids = idList();
        db.setClass(ids, idList());
        if (nested) frame.list.push(keyword);
        break;
      }
      case T.STYLE: {
        const keyword = texts[i++];
        const ids = idList();
        const styles = stylesOpt();
        separator();
        db.addCssStyles(ids, styles);
        if (nested) frame.list.push(keyword);
        break;
      }
      default: {
        if (!isName(t)) fail(nested ? `'END', ${LINE}` : LINE);
        const names = entityStatement();
        if (nested) for (const name of names) frame.list.push(name);
      }
    }
  }
}
