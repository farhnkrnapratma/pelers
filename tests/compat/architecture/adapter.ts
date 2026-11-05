import { ArchitectureDb, populate } from '../../../src/diagrams/architecture/db.js';
import { parseArchitecture, type ArchitectureAst } from '../../../src/diagrams/architecture/parser.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';

export const Architecture = { $type: 'Architecture' };

export const architectureParse = (src: string) => toResult<ArchitectureAst>(parseArchitecture, src);

// Mermaid's defaults, for the specs that read them back. Pele has no config store behind this.
const CONFIG: Record<string, unknown> = {
  padding: 40,
  iconSize: 80,
  fontSize: 16,
  randomize: false,
  nodeSeparation: 75,
  idealEdgeLengthMultiplier: 1.5,
  edgeElasticity: 0.45,
  numIter: 2500,
  seed: 1,
};

export class ArchitectureDB extends ArchitectureDb {
  getServices = () => [...this.nodes.values()].filter((node) => node.type === 'service');
  getJunctions = () => [...this.nodes.values()].filter((node) => node.type === 'junction');
  getGroups = () => [...this.groups.values()];
  getEdges = () => this.edges;
  getLayoutHints = () => this.layoutHints;
  getDiagramTitle = (): string => this.title ?? '';
  getAccTitle = (): string => this.accTitle ?? '';
  getAccDescription = (): string => this.accDescr ?? '';
  getConfigField = (field: string): unknown => CONFIG[field];
}

export const parser: { parser: { yy: ArchitectureDB | undefined }; parse(src: string): Promise<void> } = {
  parser: { yy: undefined },
  async parse(src) {
    populate(parseArchitecture(src), parser.parser.yy!);
  },
};

export function setConfig(_config: unknown): void {}

export function reset(): void {}

// Only named by a skipped test.
export const cytoscape = undefined;
