import { syntaxError } from '../../errors.js';
import type { BlockDb } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import type { Block, Statement } from './types.js';

export type BlockBuilder = Pick<
  BlockDb,
  | 'typeStr2Type'
  | 'edgeStrToEdgeData'
  | 'edgeStrToEdgeStartData'
  | 'edgeStrToThickness'
  | 'edgeStrToPattern'
  | 'generateId'
  | 'setHierarchy'
>;

interface NodeInfo {
  id: string;
  label?: string;
  typeStr?: string;
  directions?: string[];
}

interface Frame {
  list: Statement[];
  header: Statement | undefined;
}

const STATEMENT_START =
  "'NODE_ID', 'COLUMNS', 'SPACE_BLOCK', 'BLOCK_DIAGRAM_KEY', 'id-block', 'classDef', 'class', 'style'";

// The grammar inlines a document's last statement when it is a list.
function closeDocument(list: Statement[]): Statement[] {
  const last = list[list.length - 1];
  if (!Array.isArray(last)) return list;
  list.pop();
  return list.concat(last);
}

export function parseBlock(src: string, db: BlockBuilder): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('block', src, starts[i], '', true);
    throw syntaxError('block', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };

  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };

  const node = (): NodeInfo => {
    const id = expect(T.NODE_ID);
    if (types[i] === T.NODE_DSTART) {
      const open = texts[i++];
      const label = expect(T.STR);
      return { id, label, typeStr: open + expect(T.NODE_DEND) };
    }
    if (types[i] === T.BLOCK_ARROW_START) {
      const open = texts[i++];
      const label = expect(T.STR);
      const directions = [expect(T.DIR)];
      while (types[i] === T.DIR) directions.push(texts[i++]);
      return { id, label, typeStr: open + expect(T.BLOCK_ARROW_END), directions };
    }
    return { id };
  };

  const nodeStatement = (): Statement => {
    const first = node();
    const size = types[i] === T.SIZE ? parseInt(texts[i++], 10) : 1;
    let from: Block = {
      id: first.id,
      label: first.label,
      type: db.typeStr2Type(first.typeStr),
      directions: first.directions,
      widthInColumns: size,
    };
    let chain: Block[] | undefined;
    while (types[i] === T.LINK || types[i] === T.START_LINK) {
      let label = '';
      if (types[i++] === T.START_LINK) {
        expect(T.LINK_LABEL);
        label = expect(T.STR);
        expect(T.LINK);
      }
      const link = texts[i - 1];
      const to = node();
      const arrowTypeEnd = db.edgeStrToEdgeData(link);
      const arrowTypeStart = db.edgeStrToEdgeStartData(link);
      const thickness = db.edgeStrToThickness(link);
      const pattern = db.edgeStrToPattern(link);
      // Mermaid keeps only the last link of `A --> B --> C` and loses its start. Here every link counts.
      chain ??= [{ id: from.id, label: from.label, type: from.type, directions: from.directions }];
      from = { id: to.id, label: to.label, type: db.typeStr2Type(to.typeStr), directions: to.directions };
      chain.push(
        {
          id: chain[chain.length - 1].id + '-' + to.id,
          start: chain[chain.length - 1].id,
          end: to.id,
          label,
          type: 'edge',
          thickness,
          pattern,
          directions: to.directions,
          arrowTypeEnd,
          arrowTypeStart,
        },
        from
      );
    }
    return chain ?? from;
  };

  expect(T.KEY);
  // Composites nest without recursion: each open one keeps the statement list of its parent.
  const open: Frame[] = [];
  let list: Statement[] = [];
  while (true) {
    switch (types[i]) {
      case T.NODE_ID:
        list.push(nodeStatement());
        break;
      case T.COLUMNS:
        list.push({ type: 'column-setting', columns: parseInt(texts[i++], 10) } as Block);
        break;
      case T.SPACE_BLOCK: {
        const width = parseInt(texts[i++], 10);
        list.push({ id: db.generateId(), type: 'space', label: '', width, children: [] });
        break;
      }
      case T.KEY:
        i++;
        open.push({ list, header: undefined });
        list = [];
        break;
      case T.ID_BLOCK:
        i++;
        if (types[i] !== T.NODE_ID) fail("'NODE_ID'");
        open.push({ list, header: nodeStatement() });
        list = [];
        break;
      case T.end: {
        const frame = open.pop();
        if (frame === undefined || list.length === 0) return fail(STATEMENT_START);
        i++;
        const id = db.generateId();
        const children = closeDocument(list) as Block[];
        list = frame.list;
        list.push(
          frame.header === undefined
            ? { id, type: 'composite', label: '', children }
            : ({ ...frame.header, type: 'composite', children } as Block)
        );
        break;
      }
      case T.CLASSDEF: {
        i++;
        const id = expect(T.CLASSDEF_ID).trim();
        list.push({ type: 'classDef', id, css: expect(T.CLASSDEF_STYLEOPTS).trim() });
        break;
      }
      case T.CLASS: {
        i++;
        const id = expect(T.CLASSENTITY_IDS).trim();
        list.push({ type: 'applyClass', id, styleClass: expect(T.STYLECLASS).trim() });
        break;
      }
      case T.STYLE: {
        i++;
        const id = expect(T.STYLE_ENTITY_IDS).trim();
        list.push({ type: 'applyStyles', id, stylesStr: expect(T.STYLE_DEFINITION_DATA).trim() });
        break;
      }
      case T.EOF:
        if (open.length > 0) fail(STATEMENT_START + ", 'end'");
        if (list.length === 0) fail(STATEMENT_START);
        db.setHierarchy(closeDocument(list));
        return;
      default:
        fail(list.length === 0 ? STATEMENT_START : STATEMENT_START + (open.length > 0 ? ", 'end'" : ", 'EOF'"));
    }
  }
}
