import { accDescr } from '../common/accDescr.js';
import {
  ACC_TITLE,
  Reader,
  accDescrValue,
  accTitleValue,
  keyword,
  titleBefore,
  titleValue,
  tokenize,
  type TokenType,
} from '../common/tokens.js';
import { isBlank } from '../../util/chars.js';

const enum T {
  keyword,
  accDescr,
  accTitle,
  title,
  classAnnotation,
  iconAnnotation,
  descAnnotation,
  indentation,
  quotedName,
  ws,
  comment,
  newline,
  bareName,
}

function annotationAt(src: string, at: number): boolean {
  if (src.startsWith('##', at) || src.startsWith('icon(', at)) return true;
  if (!src.startsWith(':::', at)) return false;
  let i = at + 3;
  while (isBlank(src.charCodeAt(i))) i++;
  const c = src.charCodeAt(i);
  return (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c === 95;
}

// The pattern looks ahead from every blank for an annotation, which rescans long runs of blanks.
// This looks once per run.
function matchBareName(src: string, at: number): number {
  const first = src.charCodeAt(at);
  if (isBlank(first) || first === 10 || first === 13 || first === 34 || first === 39) return -1;
  if (src.startsWith(':::', at) || src.startsWith('icon(', at) || src.startsWith('##', at)) return -1;
  const n = src.length;
  let i = at + 1;
  while (i < n) {
    const c = src.charCodeAt(i);
    if (c === 10 || c === 13) break;
    if (isBlank(c)) {
      let j = i + 1;
      while (isBlank(src.charCodeAt(j))) j++;
      if (annotationAt(src, j)) break;
      i = j;
    } else {
      i++;
    }
  }
  return i;
}

const BARE_NAME: TokenType = {
  name: 'BARE_NAME',
  pattern: /(?!:::|icon\(|##)[^ \t\n\r"'](?:(?![ \t]+:::[ \t]*[A-Za-z_]|[ \t]+icon\(|[ \t]+##)[^\n\r])*/y,
  match: matchBareName,
};

export const TREEVIEW_TOKENS: readonly TokenType[] = [
  { ...keyword('treeView-beta'), longer: [BARE_NAME] },
  accDescr(),
  ACC_TITLE,
  titleBefore(String.raw`(?!%%)[^\n\r]`),
  { name: 'CLASS_ANNOTATION', pattern: /[ \t]+:::[ \t]*[A-Za-z_][\w-]*/y },
  { name: 'ICON_ANNOTATION', pattern: /[ \t]+icon\([\w-]*(?::[\w-]+)?\)/y },
  { name: 'DESC_ANNOTATION', pattern: /[ \t]+##[^\n\r]*/y },
  { name: 'INDENTATION', pattern: /[ \t]{1,}/y },
  { name: 'QUOTED_NAME', pattern: /"[^"]*"|'[^']*'/y },
  { name: 'WS', pattern: /[ \t]+/y, hidden: true },
  { name: 'ML_COMMENT', pattern: /\%\%[^\n]*/y, hidden: true },
  { name: 'NL', pattern: /\r?\n/y, hidden: true },
  BARE_NAME,
];

export interface TreeNodeAst {
  $type: 'TreeNode';
  indent?: number;
  name: string;
  classAnnotation?: string;
  iconAnnotation?: string;
  descAnnotation?: string;
}

export interface TreeViewAst {
  $type: 'TreeView';
  title?: string;
  accTitle?: string;
  accDescr?: string;
  nodes: TreeNodeAst[];
}

function trimBlanksEnd(text: string): string {
  let end = text.length;
  while (end > 0 && isBlank(text.charCodeAt(end - 1))) end--;
  return end === text.length ? text : text.slice(0, end);
}

// Mermaid's TreeViewValueConverter, by terminal name.
export function convertValue(rule: string, input: string): string | number | undefined {
  switch (rule) {
    case 'INDENTATION':
      return input?.length || 0;
    case 'QUOTED_NAME':
      return input.substring(1, input.length - 1);
    case 'BARE_NAME':
      return trimBlanksEnd(input);
    case 'CLASS_ANNOTATION':
      return input.trim().substring(3).trim();
    case 'ICON_ANNOTATION': {
      const trimmed = input.trim();
      return trimmed.substring(5, trimmed.length - 1);
    }
    case 'DESC_ANNOTATION':
      return input.trim().substring(2).trim();
    default:
      return undefined;
  }
}

export function parseTreeView(src: string): TreeViewAst {
  const r = new Reader(tokenize(src, TREEVIEW_TOKENS, 'treeView'), TREEVIEW_TOKENS, 'treeView');
  const ast: TreeViewAst = { $type: 'TreeView', nodes: [] };
  r.expect(T.keyword);
  for (;;) {
    if (r.kind === T.accDescr) ast.accDescr = accDescrValue(r.take());
    else if (r.kind === T.accTitle) ast.accTitle = accTitleValue(r.take());
    else if (r.kind === T.title) ast.title = titleValue(r.take());
    else break;
  }
  while (r.kind !== -1) {
    const node: TreeNodeAst = { $type: 'TreeNode', name: '' };
    if (r.kind === T.indentation) node.indent = r.take().length;
    if (r.kind === T.quotedName) {
      const text = r.take();
      node.name = text.substring(1, text.length - 1);
    } else if (r.kind === T.bareName) {
      node.name = trimBlanksEnd(r.take());
    } else {
      r.fail('a name');
    }
    for (;;) {
      if (r.kind === T.classAnnotation) node.classAnnotation = convertValue('CLASS_ANNOTATION', r.take()) as string;
      else if (r.kind === T.iconAnnotation) node.iconAnnotation = convertValue('ICON_ANNOTATION', r.take()) as string;
      else if (r.kind === T.descAnnotation) node.descAnnotation = convertValue('DESC_ANNOTATION', r.take()) as string;
      else break;
    }
    ast.nodes.push(node);
  }
  return ast;
}
