import {
  ACC_DESCR,
  ACC_TITLE,
  Reader,
  accDescrValue,
  accTitleValue,
  titleBefore,
  titleValue,
  tokenize,
  type TokenType,
} from '../common/tokens.js';

const enum T {
  accDescr,
  accTitle,
  title,
  keyword,
  classDef,
  styleSeparator,
  separator,
  comma,
  indentation,
  whitespace,
  comment,
  newline,
  id,
  number,
  string,
}

// Mermaid's treemap grammar has its own terminals. Every run of spaces or tabs is an INDENTATION
// token, since that comes before WS, and line breaks are hidden, so rows are told apart by their
// tokens alone and the indentation of a row is whatever whitespace precedes it.
export const TREEMAP_TOKENS: readonly TokenType[] = [
  ACC_DESCR,
  ACC_TITLE,
  titleBefore(String.raw`\w`),
  { name: 'TREEMAP_KEYWORD', pattern: /treemap-beta|treemap/y },
  { name: 'CLASS_DEF', pattern: /classDef\s+([a-zA-Z_][a-zA-Z0-9_]+)(?:\s+([^;\r\n]*))?(?:;)?/y },
  { name: 'STYLE_SEPARATOR', pattern: /:::/y },
  { name: 'SEPARATOR', pattern: /:/y },
  { name: 'COMMA', pattern: /,/y },
  { name: 'INDENTATION', pattern: /[ \t]{1,}/y },
  { name: 'WS', pattern: /[ \t]+/y, hidden: true },
  { name: 'ML_COMMENT', pattern: /\%\%[^\n]*/y, hidden: true },
  { name: 'NL', pattern: /\r?\n/y, hidden: true },
  { name: 'ID2', pattern: /[a-zA-Z_][a-zA-Z0-9_]*/y },
  { name: 'NUMBER2', pattern: /[0-9_\.\,]+/y },
  { name: 'STRING2', pattern: /"[^"]*"|'[^']*'/y },
];

const RE_CLASS_DEF = /classDef\s+([A-Z_a-z]\w+)(?:\s+([^\n\r;]*))?;?/;

export interface SectionAst {
  $type: 'Section';
  name: string;
  classSelector?: string;
}

export interface LeafAst {
  $type: 'Leaf';
  name: string;
  value: number;
  classSelector?: string;
}

export interface TreemapRowAst {
  $type: 'TreemapRow';
  indent?: number;
  item: SectionAst | LeafAst;
}

export interface ClassDefAst {
  $type: 'ClassDefStatement';
  indent?: number;
  className: string;
  styleText?: string;
}

export interface TreemapAst {
  $type: 'Treemap';
  title?: string;
  accTitle?: string;
  accDescr?: string;
  TreemapRows: (TreemapRowAst | ClassDefAst)[];
}

export function parseTreemap(src: string): TreemapAst {
  const r = new Reader(tokenize(src, TREEMAP_TOKENS, 'treemap'), TREEMAP_TOKENS, 'treemap');
  const kinds = r.kinds;
  const ast: TreemapAst = { $type: 'Treemap', TreemapRows: [] };

  const classSelector = (): string | undefined => (r.accept(T.styleSeparator) ? r.expect(T.id) : undefined);

  r.expect(T.keyword);
  while (r.kind !== -1) {
    const kind = r.kind;
    if (kind === T.accDescr) {
      ast.accDescr = accDescrValue(r.take());
      continue;
    }
    if (kind === T.accTitle) {
      ast.accTitle = accTitleValue(r.take());
      continue;
    }
    if (kind === T.title) {
      ast.title = titleValue(r.take());
      continue;
    }
    if (kind !== T.indentation && kind !== T.string && kind !== T.classDef)
      r.fail('a node, a class definition, or a title');

    const indent = kind === T.indentation ? r.take().length : undefined;
    if (r.kind === T.classDef) {
      const m = RE_CLASS_DEF.exec(r.take())!;
      const row: ClassDefAst = { $type: 'ClassDefStatement', className: m[1] };
      if (m[2]) row.styleText = m[2];
      if (indent !== undefined) row.indent = indent;
      ast.TreemapRows.push(row);
      continue;
    }

    const name = r.expect(T.string).slice(1, -1);
    // A leaf is a name followed by a colon or comma, with or without whitespace before it.
    const after = kinds[r.i] === T.indentation ? r.i + 1 : r.i;
    let item: SectionAst | LeafAst;
    if (kinds[after] === T.separator || kinds[after] === T.comma) {
      r.i = after + 1;
      r.accept(T.indentation);
      item = { $type: 'Leaf', name, value: parseFloat(r.expect(T.number).replace(/,/g, '')) };
    } else {
      item = { $type: 'Section', name };
    }
    const selector = classSelector();
    if (selector !== undefined) item.classSelector = selector;
    const row: TreemapRowAst = { $type: 'TreemapRow', item };
    if (indent !== undefined) row.indent = indent;
    ast.TreemapRows.push(row);
  }
  return ast;
}
