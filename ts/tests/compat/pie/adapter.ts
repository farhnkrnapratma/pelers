import { buildPie, type PieModel } from '../../../src/diagrams/pie/model.js';
import { parsePie, type PieAst } from '../../../src/diagrams/pie/parser.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';

export const Pie = { $type: 'Pie' };

export const pieParse = (src: string) => toResult<PieAst>(parsePie, src);

export function setConfig(_config: unknown): void {}

const CONFIG = { useWidth: 984, useMaxWidth: true, textPosition: 0.75, legendPosition: 'right', donutHole: 0 };

export const DEFAULT_PIE_DB = { sections: new Map<string, number>(), showData: false, config: CONFIG };

let model: PieModel | undefined;

// Mermaid's pie database is a module-level singleton; this one holds the last parsed model.
export const db = {
  clear(): void {
    model = undefined;
  },
  getConfig: () => structuredClone(CONFIG),
  getSections: () => model?.sections ?? new Map<string, number>(),
  getShowData: () => model?.showData ?? false,
  getDiagramTitle: () => model?.title ?? '',
  getAccTitle: () => model?.accTitle ?? '',
  getAccDescription: () => model?.accDescr ?? '',
};

export const parser = {
  async parse(src: string): Promise<void> {
    model = buildPie(parsePie(src), undefined);
  },
};
