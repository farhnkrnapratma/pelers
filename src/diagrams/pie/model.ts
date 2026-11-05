import { PeleError } from '../../errors.js';
import type { PieAst } from './parser.js';

export interface PieModel {
  type: 'pie';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  showData: boolean;
  sections: Map<string, number>;
}

export function buildPie(ast: PieAst, title: string | undefined): PieModel {
  const sections = new Map<string, number>();
  for (const { label, value } of ast.sections) {
    if (value < 0) {
      throw new PeleError(
        `"${label}" has invalid value: ${value}. Negative values are not allowed in pie charts. All slice values must be >= 0.`,
        'semantic',
        { type: 'pie' }
      );
    }
    if (!sections.has(label)) sections.set(label, value);
  }
  return {
    type: 'pie',
    title: ast.title || title,
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : undefined,
    showData: ast.showData,
    sections,
  };
}
