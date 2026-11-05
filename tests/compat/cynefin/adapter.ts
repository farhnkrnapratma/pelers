import { FOLD, HORIZON, cliff, curvePath, ellipsePath, wave } from '../../../src/diagrams/cynefin/boundaries.js';
import { CynefinDb, buildCynefin, type DomainBlock, type Transition } from '../../../src/diagrams/cynefin/model.js';
import { parseCynefin } from '../../../src/diagrams/cynefin/parser.js';

export type { DomainBlock, Transition } from '../../../src/diagrams/cynefin/model.js';
export { hashString, resolveSeed, seededRandom } from '../../../src/diagrams/cynefin/boundaries.js';

// Mermaid's boundary functions return path strings for a diagram of the given size.
export const generateFoldPath = (width: number, height: number, seed: number, amplitude = width * 0.015): string =>
  curvePath(wave(0, height, width / 2, seed, amplitude, FOLD));
export const generateHorizontalBoundary = (width: number, height: number, seed: number, amplitude = height * 0.015): string =>
  curvePath(wave(0, width, height / 2, seed, amplitude, HORIZON));
export const generateCliffPath = (width: number, height: number): string => curvePath(cliff(width / 2, height * 0.5, height, width * 0.03));
export const generateConfusionPath = ellipsePath;

let state = new CynefinDb();
let accTitle = '';
let accDescription = '';
let diagramTitle = '';

// Mermaid's Cynefin database is a module-level singleton; this one holds the last parsed model.
export const db = {
  clear(): void {
    state = new CynefinDb();
    accTitle = accDescription = diagramTitle = '';
  },
  getDomains: () => state.domains,
  getTransitions: () => state.transitions,
  setDomains: (blocks: DomainBlock[]) => state.setDomains(blocks),
  setTransitions: (transitions: Transition[]) => state.setTransitions(transitions),
  getConfig: () => ({ width: 640, height: 440, showDomainDescriptions: true, boundaryAmplitude: 8, seed: 0 }),
  getAccTitle: () => accTitle,
  getAccDescription: () => accDescription,
  getDiagramTitle: () => diagramTitle,
};

export const parser = {
  async parse(src: string): Promise<void> {
    const model = buildCynefin(parseCynefin(src), undefined);
    state.domains = model.domains;
    state.transitions = model.transitions;
    accTitle = model.accTitle ?? '';
    accDescription = model.accDescr ?? '';
    diagramTitle = model.title ?? '';
  },
};
