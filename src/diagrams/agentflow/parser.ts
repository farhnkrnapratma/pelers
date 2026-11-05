import { inputOffset, replacements, syntaxError } from '../../errors.js';
import type { AgentflowDb } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import type { DocItem, FlowText, LabelType, LinkInfo, Loc } from './types.js';

const ID = new Uint8Array(T.EOF + 1);
for (const t of [
  T.NUM,
  T.NODE_STRING,
  T.DOWN,
  T.MINUS,
  T.DEFAULT,
  T.COMMA,
  T.COLON,
  T.AMP,
  T.BRKT,
  T.MULT,
  T.UNICODE_TEXT,
]) {
  ID[t] = 1;
}

const ALPHANUM = new Uint8Array(T.EOF + 1);
for (const t of [
  T.NUM,
  T.UNICODE_TEXT,
  T.NODE_STRING,
  T.DIR,
  T.DOWN,
  T.MINUS,
  T.COMMA,
  T.COLON,
  T.AMP,
  T.BRKT,
  T.MULT,
]) {
  ALPHANUM[t] = 1;
}

const NO_TAGS = new Uint8Array(T.EOF + 1);
for (const t of [
  T.NUM,
  T.NODE_STRING,
  T.SPACE,
  T.MINUS,
  T.AMP,
  T.UNICODE_TEXT,
  T.COLON,
  T.MULT,
  T.BRKT,
  T.STYLE,
  T.LINKSTYLE,
  T.CLASSDEF,
  T.CLASS,
  T.CLICK,
  T.GRAPH,
  T.DIR,
  T.flow,
  T.connector,
  T.global,
  T.end,
  T.DOWN,
  T.UP,
  T.START_LINK,
]) {
  NO_TAGS[t] = 1;
}

const STYLE_PART = new Uint8Array(T.EOF + 1);
for (const t of [T.NUM, T.NODE_STRING, T.COLON, T.SPACE, T.BRKT, T.STYLE]) STYLE_PART[t] = 1;

const SEPARATOR = new Uint8Array(T.EOF + 1);
for (const t of [T.NEWLINE, T.SEMI, T.EOF, T.COMMENT]) SEPARATOR[t] = 1;

interface Frame {
  global: boolean;
  // Token index of the `flow` keyword.
  at: number;
  id: FlowText | undefined;
  title: FlowText | undefined;
  data: string | undefined;
  dataFrom: number;
  dataTo: number;
  list: DocItem[];
  collect: boolean;
}

const TEXT_TOKENS = "'UNICODE_TEXT', 'TEXT', 'TAGSTART'";
const SEPARATORS = "'COMMENT', 'SEMI', 'NEWLINE', 'EOF'";
const TRAILING = new Uint8Array(T.EOF + 1);
for (const t of [T.COMMENT, T.NEWLINE, T.SPACE, T.EOF]) TRAILING[t] = 1;
const NO_ITEMS: string[] = [];

// Mermaid trims spaces and tabs between a closing brace and the end of its line before parsing.
const RE_BRACE_TRAIL = /}[^\S\n]*\n/g;

// `raw` parses the text as it is, the way Mermaid's generated parser does without its wrapper.
export function parseAgentflow(source: string, db: AgentflowDb, raw = false): void {
  const src = !raw && source.includes('}') ? source.replace(RE_BRACE_TRAIL, '}\n') : source;
  const { types, texts, starts, locs } = tokenize(src);
  let i = 0;
  let pendingData: string | undefined;
  let dataFrom = 0;
  let dataTo = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    let from = starts[i];
    if (t === T.EOF || t === T.END) {
      // Comments are tokens here, so skip trailing ones.
      let k = i;
      while (k > 0 && TRAILING[types[k - 1]]) k--;
      from = src.slice(0, starts[k]).trimEnd().length;
    }
    const at = src === source ? from : inputOffset([replacements(source, RE_BRACE_TRAIL, 2)], from);
    if (t === T.ERROR) throw syntaxError('agentflow', source, at, '', true);
    throw syntaxError('agentflow', source, at, `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };

  // The span from the start of token `a` to the end of token `b`.
  const span = (a: number, b: number): Loc => ({
    first_line: locs[4 * a],
    first_column: locs[4 * a + 1],
    last_line: locs[4 * b + 2],
    last_column: locs[4 * b + 3],
  });

  const expect = (type: number, also = ''): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'${also ? ', ' + also : ''}`);
    return texts[i++];
  };

  const separator = (): void => {
    if (SEPARATOR[types[i]] === 0) fail(SEPARATORS);
    i++;
  };

  const idString = (): string => {
    if (ID[types[i]] === 0) fail("'NODE_STRING'");
    let s = texts[i++];
    while (ID[types[i]] === 1) s += texts[i++];
    return s;
  };

  const alphaNum = (): string => {
    if (ALPHANUM[types[i]] === 0) fail("'NODE_STRING'");
    let s = texts[i++];
    while (ALPHANUM[types[i]] === 1) s += texts[i++];
    return s;
  };

  const text = (): FlowText => {
    const t = types[i];
    let type: LabelType;
    if (t === T.STR) type = 'string';
    else if (t === T.MD_STR) type = 'markdown';
    else if (t === T.TEXT || t === T.TAGSTART || t === T.TAGEND || t === T.UNICODE_TEXT) type = 'text';
    else return fail(`${TEXT_TOKENS}, 'TAGEND', 'STR', 'MD_STR'`);
    let s = texts[i++];
    while (true) {
      const n = types[i];
      if (n !== T.TEXT && n !== T.TAGSTART && n !== T.TAGEND && n !== T.UNICODE_TEXT) break;
      s += texts[i++];
    }
    return { text: s, type };
  };

  const closed = (type: number): void => {
    expect(type, TEXT_TOKENS);
  };

  const edgeText = (): FlowText => {
    const t = types[i];
    let type: LabelType;
    if (t === T.STR) type = 'string';
    else if (t === T.MD_STR) type = 'markdown';
    else if (t === T.EDGE_TEXT || t === T.UNICODE_TEXT) type = 'text';
    else return fail("'EDGE_TEXT', 'STR', 'MD_STR'");
    let s = texts[i++];
    while (types[i] === T.EDGE_TEXT || types[i] === T.UNICODE_TEXT) s += texts[i++];
    return { text: s, type };
  };

  const textNoTags = (): FlowText => {
    const t = types[i];
    let type: LabelType;
    if (t === T.STR) type = 'text';
    else if (t === T.MD_STR) type = 'markdown';
    else if (NO_TAGS[t] === 1) type = 'text';
    else return fail("'NODE_STRING', 'STR', 'MD_STR'");
    let s = texts[i++];
    while (NO_TAGS[types[i]] === 1) s += texts[i++];
    return { text: s, type };
  };

  const shapeData = (): string => {
    dataFrom = i;
    let s = texts[i++];
    while (types[i] === T.SHAPE_DATA) s += texts[i++];
    dataTo = i - 1;
    return s;
  };

  const attach = (id: string, data: string, from: number, to: number): void => {
    const loc = span(from, to);
    db.addVertex(id, undefined, undefined, undefined, undefined, undefined, undefined, data, loc);
    db.extendVertexMapping(id, loc);
  };

  const attachData = (id: string, frame: Frame): void => {
    if (frame.data === undefined) return;
    db.addVertex(
      id,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      frame.data,
      span(frame.dataFrom, frame.dataTo)
    );
  };

  const vertex = (): string => {
    const from = i;
    const id = idString();
    let label: FlowText | undefined;
    let type: string | undefined;
    let props: Record<string, string> | undefined;
    switch (types[i]) {
      case T.SQS:
        i++;
        label = text();
        closed(T.SQE);
        type = 'square';
        break;
      case T.DOUBLECIRCLESTART:
        i++;
        label = text();
        closed(T.DOUBLECIRCLEEND);
        type = 'doublecircle';
        break;
      case T.PS:
        i++;
        if (types[i] === T.PS) {
          i++;
          label = text();
          closed(T.PE);
          expect(T.PE);
          type = 'circle';
        } else {
          label = text();
          closed(T.PE);
          type = 'round';
        }
        break;
      case T.ELLIPSE_START:
        i++;
        label = text();
        closed(T.ELLIPSE_END);
        type = 'ellipse';
        break;
      case T.STADIUMSTART:
        i++;
        label = text();
        closed(T.STADIUMEND);
        type = 'stadium';
        break;
      case T.SUBROUTINESTART:
        i++;
        label = text();
        closed(T.SUBROUTINEEND);
        type = 'subroutine';
        break;
      case T.VERTEX_WITH_PROPS_START: {
        i++;
        const field = expect(T.NODE_STRING);
        expect(T.COLON);
        const value = expect(T.NODE_STRING);
        expect(T.PIPE);
        label = text();
        closed(T.SQE);
        type = 'rect';
        props = { [field]: value };
        break;
      }
      case T.CYLINDERSTART:
        i++;
        label = text();
        closed(T.CYLINDEREND);
        type = 'cylinder';
        break;
      case T.DIAMOND_START:
        i++;
        if (types[i] === T.DIAMOND_START) {
          i++;
          label = text();
          closed(T.DIAMOND_STOP);
          expect(T.DIAMOND_STOP);
          type = 'hexagon';
        } else {
          label = text();
          closed(T.DIAMOND_STOP);
          type = 'diamond';
        }
        break;
      case T.TAGEND:
        i++;
        label = text();
        closed(T.SQE);
        type = 'odd';
        break;
      case T.TRAPSTART:
        i++;
        label = text();
        if (types[i] === T.INVTRAPEND) {
          i++;
          type = 'lean_right';
        } else {
          closed(T.TRAPEND);
          type = 'trapezoid';
        }
        break;
      case T.INVTRAPSTART:
        i++;
        label = text();
        if (types[i] === T.TRAPEND) {
          i++;
          type = 'lean_left';
        } else {
          closed(T.INVTRAPEND);
          type = 'inv_trapezoid';
        }
        break;
    }
    if (props !== undefined) db.addVertex(id, label, type, undefined, undefined, undefined, props);
    else if (type !== undefined) db.addVertex(id, label, type);
    else db.addVertex(id);
    db.addVertexMapping(id, label, type, span(from, i - 1));
    return id;
  };

  const styledVertex = (): string => {
    const id = vertex();
    if (types[i] === T.STYLE_SEPARATOR) {
      i++;
      db.setClass(id, idString());
    }
    return id;
  };

  const node = (): string[] => {
    const ids = [styledVertex()];
    pendingData = undefined;
    while (true) {
      const hasData = types[i] === T.SHAPE_DATA;
      const data = hasData ? shapeData() : undefined;
      const from = dataFrom;
      const to = dataTo;
      if (types[i] !== T.SPACE) {
        pendingData = data;
        return ids;
      }
      do i++;
      while (types[i] === T.SPACE);
      if (types[i] !== T.AMP) {
        if (data !== undefined) fail("'AMP'");
        return ids;
      }
      i++;
      if (types[i] !== T.SPACE) fail("'SPACE'");
      do i++;
      while (types[i] === T.SPACE);
      const next = styledVertex();
      if (data !== undefined) attach(ids[ids.length - 1], data, from, to);
      ids.push(next);
    }
  };

  const link = (): LinkInfo => {
    let id: string | undefined;
    if (types[i] === T.LINK_ID) id = texts[i++];
    if (types[i] === T.LINK) {
      const info: LinkInfo = db.destructLink(texts[i++]);
      if (id !== undefined) info.id = id;
      if (types[i] === T.PIPE) {
        i++;
        info.text = text();
        closed(T.PIPE);
        if (types[i] === T.SPACE) i++;
      }
      return info;
    }
    if (types[i] === T.START_LINK) {
      const start = texts[i++];
      const label = edgeText();
      const end = expect(T.LINK, "'EDGE_TEXT'");
      const info: LinkInfo = db.destructLink(end, start);
      info.text = label;
      if (id !== undefined) info.id = id;
      return info;
    }
    return fail("'LINK', 'START_LINK'");
  };

  const vertexStatement = (): string[] => {
    const from = i;
    let stmt = node();
    if (pendingData !== undefined) attach(stmt[stmt.length - 1], pendingData, dataFrom, dataTo);
    let nodes = stmt;
    while (true) {
      const t = types[i];
      if (t !== T.LINK && t !== T.START_LINK && t !== T.LINK_ID) return nodes;
      const info = link();
      const next = node();
      if (pendingData !== undefined) attach(next[next.length - 1], pendingData, dataFrom, dataTo);
      db.addLink(stmt, next, info);
      db.addEdgeMapping(stmt, next, info, span(from, i - 1));
      nodes = next.concat(nodes);
      stmt = next;
    }
  };

  const style = (): string => {
    if (STYLE_PART[types[i]] === 0) fail("'NODE_STRING', 'NUM', 'COLON', 'BRKT'");
    let s = texts[i++];
    while (STYLE_PART[types[i]] === 1) s += texts[i++];
    return s;
  };

  const stylesOpt = (): string[] => {
    const out = [style()];
    while (types[i] === T.COMMA) {
      i++;
      out.push(style());
    }
    return out;
  };

  const linkStyleStatement = (): void => {
    i++;
    expect(T.SPACE);
    let positions: string[];
    if (types[i] === T.DEFAULT) {
      positions = [texts[i++]];
    } else {
      positions = [expect(T.NUM, "'DEFAULT'")];
      while (types[i] === T.COMMA) {
        i++;
        positions.push(expect(T.NUM));
      }
    }
    expect(T.SPACE);
    if (types[i] !== T.INTERPOLATE) {
      db.updateLink(positions, stylesOpt());
      return;
    }
    i++;
    expect(T.SPACE);
    const interpolate = alphaNum();
    if (types[i] === T.SPACE) {
      i++;
      const styles = stylesOpt();
      db.updateLinkInterpolate(positions, interpolate);
      db.updateLink(positions, styles);
    } else {
      db.updateLinkInterpolate(positions, interpolate);
    }
  };

  const linkTail = (id: string, url: string): void => {
    if (types[i] !== T.SPACE) {
      db.setLink(id, url);
      return;
    }
    i++;
    if (types[i] === T.LINK_TARGET) {
      db.setLink(id, url, texts[i++]);
      return;
    }
    const tooltip = expect(T.STR, "'LINK_TARGET'");
    if (types[i] === T.SPACE) {
      i++;
      db.setLink(id, url, expect(T.LINK_TARGET));
    } else {
      db.setLink(id, url);
    }
    db.setTooltip(id, tooltip);
  };

  const clickStatement = (): void => {
    const id = texts[i++];
    switch (types[i]) {
      case T.CALLBACKNAME: {
        const name = texts[i++];
        const hasArgs = types[i] === T.CALLBACKARGS;
        const args = hasArgs ? texts[i++] : undefined;
        let tooltip: string | undefined;
        if (types[i] === T.SPACE) {
          i++;
          tooltip = expect(T.STR);
        }
        if (hasArgs) db.setClickEvent(id, name, args);
        else db.setClickEvent(id, name);
        if (tooltip !== undefined) db.setTooltip(id, tooltip);
        return;
      }
      case T.HREF:
        i++;
        linkTail(id, expect(T.STR));
        return;
      case T.STR:
        linkTail(id, texts[i++]);
        return;
      default: {
        const name = alphaNum();
        if (types[i] === T.SPACE) {
          i++;
          const tooltip = expect(T.STR);
          db.setClickEvent(id, name);
          db.setTooltip(id, tooltip);
        } else {
          db.setClickEvent(id, name);
        }
      }
    }
  };

  // Reads `flow` or `connector` up to the separator: an id, an optional bracketed title, optional metadata.
  const header = (at: number): Frame => {
    expect(T.SPACE, SEPARATORS);
    const id = textNoTags();
    let title: FlowText = { text: '', type: 'text' };
    if (types[i] === T.SQS) {
      i++;
      title = text();
      closed(T.SQE);
    }
    const data = types[i] === T.SHAPE_DATA ? shapeData() : undefined;
    if (SEPARATOR[types[i]] === 0) fail(`${SEPARATORS}, 'SHAPE_DATA'`);
    return { global: false, at, id, title, data, dataFrom, dataTo, list: [], collect: false };
  };

  const statement = (): DocItem => {
    const t = types[i];
    switch (t) {
      case T.STYLE: {
        i++;
        expect(T.SPACE);
        const id = idString();
        expect(T.SPACE);
        db.addVertex(id, undefined, undefined, stylesOpt());
        separator();
        return NO_ITEMS;
      }
      case T.LINKSTYLE:
        linkStyleStatement();
        separator();
        return NO_ITEMS;
      case T.CLASSDEF: {
        i++;
        expect(T.SPACE);
        const id = idString();
        expect(T.SPACE);
        db.addClass(id, stylesOpt());
        separator();
        return NO_ITEMS;
      }
      case T.CLASS: {
        i++;
        expect(T.SPACE);
        const ids = idString();
        expect(T.SPACE);
        db.setClass(ids, idString());
        separator();
        return NO_ITEMS;
      }
      case T.CLICK:
        clickStatement();
        separator();
        return NO_ITEMS;
      case T.connector: {
        const frame = header(i++);
        const sep = i++;
        const id = db.addConnector(frame.id!, frame.title);
        attachData(id, frame);
        db.addConnectorMapping(frame.id, frame.title, span(frame.at, frame.at), span(sep, sep));
        return id;
      }
      case T.direction_tb:
      case T.direction_bt:
      case T.direction_rl:
      case T.direction_lr:
      case T.direction_td:
        i++;
        return { stmt: 'dir', value: DIRECTIONS[t - T.direction_tb] };
      case T.acc_title: {
        i++;
        const value = expect(T.acc_title_value).trim();
        db.setAccTitle(value);
        return value;
      }
      case T.acc_descr: {
        i++;
        const value = expect(T.acc_descr_value).trim();
        db.setAccDescription(value);
        return value;
      }
      case T.acc_descr_multiline_value: {
        const value = texts[i++].trim();
        db.setAccDescription(value);
        return value;
      }
      default: {
        if (ID[t] === 0) fail(STATEMENT_START);
        const nodes = vertexStatement();
        separator();
        return nodes;
      }
    }
  };

  // Blocks nest without recursion: each open one keeps the list and collect flag of its parent.
  const document = (): void => {
    const open: Frame[] = [];
    let list: DocItem[] = [];
    let collect = false;
    while (true) {
      const t = types[i];
      if (t === T.END) break;
      if (t === T.end) {
        const frame = open.pop();
        if (frame === undefined) break;
        const at = i++;
        if (frame.global) {
          db.addGlobal(list);
          list = frame.list;
          collect = frame.collect;
          continue;
        }
        const id = db.addSubGraph(frame.id, list, frame.title, 'flow');
        attachData(id, frame);
        db.addSubgraphMapping(frame.id, frame.title, span(frame.at, frame.at), span(at, at));
        list = frame.list;
        collect = frame.collect;
        if (collect) list.push(id);
      } else if (t === T.SEMI || t === T.NEWLINE || t === T.SPACE || t === T.EOF) {
        if (collect) list.push(texts[i]);
        i++;
      } else if (t === T.COMMENT) {
        i++;
      } else if (t === T.flow || t === T.global) {
        const at = i++;
        let frame: Frame;
        if (t === T.global || SEPARATOR[types[i]] === 1) {
          frame = {
            global: t === T.global,
            at,
            id: undefined,
            title: undefined,
            data: undefined,
            dataFrom: 0,
            dataTo: 0,
            list,
            collect,
          };
          separator();
        } else {
          frame = header(at);
          frame.list = list;
          frame.collect = collect;
          i++;
        }
        open.push(frame);
        list = [];
        collect = true;
      } else {
        const item = statement();
        if (collect && (!Array.isArray(item) || item.length > 0)) list.push(item);
      }
    }
    if (open.length > 0) fail("'end'");
  };

  while (types[i] === T.SPACE || types[i] === T.NEWLINE || types[i] === T.COMMENT) i++;
  expect(T.GRAPH, "'SPACE', 'NEWLINE', 'COMMENT'");
  if (types[i] === T.NODIR) {
    i++;
    db.setDirection('TB');
  } else {
    const dir = expect(T.DIR, "'NODIR'");
    let t = types[i];
    if (t === T.SPACE) {
      do i++;
      while (types[i] === T.SPACE);
      t = types[i];
      if (t !== T.NEWLINE) fail("'NEWLINE', 'SPACE'");
    } else if (t !== T.SEMI && t !== T.NEWLINE && t !== T.COMMENT) {
      fail("'COMMENT', 'SEMI', 'NEWLINE', 'SPACE'");
    }
    i++;
    db.setDirection(dir);
  }
  document();
  if (types[i] !== T.END) fail(STATEMENT_START);
}

const DIRECTIONS = ['TB', 'BT', 'RL', 'LR', 'TD'];
const STATEMENT_START =
  "'COMMENT', 'SEMI', 'NEWLINE', 'SPACE', 'EOF', 'flow', 'global', 'connector', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'STYLE', 'LINKSTYLE', 'CLASSDEF', 'CLASS', 'CLICK', 'NODE_STRING'";
