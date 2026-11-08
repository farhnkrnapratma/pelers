import { JourneyDb } from '../../../src/diagrams/journey/db.js';
import { parseJourney } from '../../../src/diagrams/journey/parser.js';

// Exposes Pele's journey model through the object Mermaid's specs import from journeyDb.js.

let db = new JourneyDb();

const journeyDb = {
  clear(): void {
    db = new JourneyDb();
  },
  setDiagramTitle: (text: string): void => db.setDiagramTitle(text),
  getDiagramTitle: () => db.title ?? '',
  setAccTitle: (text: string): void => db.setAccTitle(text),
  getAccTitle: () => db.accTitle ?? '',
  setAccDescription: (text: string): void => db.setAccDescription(text),
  getAccDescription: () => db.accDescr ?? '',
  addSection: (text: string): void => db.addSection(text),
  getSections: () => db.sections,
  addTask: (task: string, data: string): void => db.addTask(task, data),
  getTasks: () => db.tasks.map((task) => ({ ...task, type: task.section })),
  getActors: () => db.getActors(),
};

export default journeyDb;

export const parser = {
  yy: undefined as unknown,
  parse(src: string): void {
    parseJourney(src, parser.yy as JourneyDb);
  },
};

// Mermaid's test helper that turns a tagged table into the rows `it.each` takes.
export function convert(template: TemplateStringsArray, ...params: unknown[]): Record<string, unknown>[] {
  const header = template[0].trim().split('|').map((s) => s.trim());
  const rows: Record<string, unknown>[] = [];
  for (let i = 0; i < params.length; i += header.length) {
    rows.push(Object.fromEntries(params.slice(i, i + header.length).map((value, k) => [header[k], value])));
  }
  return rows;
}
