import { buildGit, type GitModel } from '../../../src/diagrams/git/db.js';
import { parseGit, type GitGraphAst } from '../../../src/diagrams/git/parser.js';
import { toResult } from '../../support/langium.js';

export const gitGraphParse = (src: string) => toResult<GitGraphAst>(parseGit, src);

// Pele collects warnings on the model; Mermaid sends them to its logger.
export const log = {
  warn(_message: string): void {},
};

const CONFIG = {
  titleTopMargin: 25,
  diagramPadding: 8,
  nodeLabel: { width: 75, height: 100, x: -25, y: 0 },
  mainBranchName: 'main',
  mainBranchOrder: 0,
  showCommitLabel: true,
  showBranches: true,
  rotateCommitLabel: true,
  parallelCommits: false,
};

let model: GitModel = buildGit({ $type: 'GitGraph', statements: [] }, {}, undefined);

// Mermaid's git graph database is a module-level singleton; this one holds the last parsed model.
export const db = {
  clear(): void {
    model = buildGit({ $type: 'GitGraph', statements: [] }, {}, undefined);
  },
  getConfig: () => structuredClone(CONFIG),
  getCommits: () => model.commits,
  getCommitsArray: () => [...model.commits.values()].sort((a, b) => a.seq - b.seq),
  getBranches: () => model.branches,
  getBranchesAsObjArray: () => model.lanes.map((name) => ({ name })),
  getCurrentBranch: () => model.currentBranch,
  getDirection: () => model.direction,
  getHead: () => model.head,
  getDiagramTitle: () => model.title ?? '',
  getAccTitle: () => model.accTitle ?? '',
  getAccDescription: () => model.accDescr ?? '',
  prettyPrint(): void {},
};

export const parser = {
  async parse(src: string): Promise<void> {
    model = buildGit(parseGit(src), {}, undefined);
    for (const warning of model.warnings) log.warn(warning);
  },
};
