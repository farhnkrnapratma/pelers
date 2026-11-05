import { parseTreemap, type TreemapAst } from '../../../src/diagrams/treemap/parser.js';
import { toResult, type ParseResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';
export { buildHierarchy } from '../../../src/diagrams/treemap/model.js';
export type { TreemapNode } from '../../../src/diagrams/treemap/model.js';
export type {
  LeafAst as Leaf,
  SectionAst as Section,
  TreemapAst as Treemap,
  TreemapRowAst as TreemapRow,
} from '../../../src/diagrams/treemap/parser.js';

export interface LangiumParser {
  parse<T>(input: string): ParseResult<T>;
}

const LangiumParser: LangiumParser = {
  parse: <T>(input: string) => toResult(parseTreemap, input) as ParseResult<TreemapAst> as ParseResult<T>,
};

export function createTreemapServices(): { Treemap: { parser: { LangiumParser: LangiumParser } } } {
  return { Treemap: { parser: { LangiumParser } } };
}
