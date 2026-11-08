import {
  ACC_DESCR,
  ACC_TITLE,
  DIRECTIVE,
  NEWLINE,
  Reader,
  SINGLE_LINE_COMMENT,
  TITLE,
  WHITESPACE,
  YAML,
  accDescrValue,
  accTitleValue,
  keyword,
  titleValue,
  tokenize,
  type TokenType,
} from '../common/tokens.js';

const enum T {
  showInfo,
  info,
  accDescr,
  accTitle,
  title,
  newline,
}

export const INFO_TOKENS: readonly TokenType[] = [
  keyword('showInfo'),
  keyword('info'),
  ACC_DESCR,
  ACC_TITLE,
  TITLE,
  NEWLINE,
  WHITESPACE,
  YAML,
  DIRECTIVE,
  SINGLE_LINE_COMMENT,
];

export interface InfoAst {
  $type: 'Info';
  title?: string;
  accTitle?: string;
  accDescr?: string;
}

export function parseInfo(src: string): InfoAst {
  const r = new Reader(tokenize(src, INFO_TOKENS, 'info'), INFO_TOKENS, 'info');
  const ast: InfoAst = { $type: 'Info' };

  while (r.kind === T.newline) r.i++;
  r.expect(T.info);
  while (r.kind === T.newline) r.i++;
  if (r.accept(T.showInfo)) while (r.kind === T.newline) r.i++;
  while (r.kind !== -1) {
    const kind = r.kind;
    if (kind === T.accDescr) ast.accDescr = accDescrValue(r.take());
    else if (kind === T.accTitle) ast.accTitle = accTitleValue(r.take());
    else if (kind === T.title) ast.title = titleValue(r.take());
    else r.fail('a title or the end of input');
    if (r.kind === -1) break;
    if (r.kind !== T.newline) r.fail('a line break or the end of input');
    while (r.kind === T.newline) r.i++;
  }
  return ast;
}
