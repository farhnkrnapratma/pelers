import type { InfoAst } from './parser.js';

export interface InfoModel {
  type: 'info';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
}

export function buildInfo(ast: InfoAst, title: string | undefined): InfoModel {
  return {
    type: 'info',
    title: ast.title || title,
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : undefined,
  };
}
