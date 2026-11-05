import { TimelineDb } from '../../../src/diagrams/timeline/db.js';
import { parseTimeline } from '../../../src/diagrams/timeline/parser.js';

// Exposes Pele's timeline model through the functions Mermaid's specs import from timelineDb.js,
// commonDb.js and the generated parser. The spec assigns this module itself as the parser's `yy`.

let db = new TimelineDb();

export function clear(): void {
  db = new TimelineDb();
}

export const getCommonDb = () => db;
export const setDirection = (direction: 'LR' | 'TD'): void => db.setDirection(direction);
export const getDirection = () => db.direction;
export const addSection = (text: string): void => db.addSection(text);
export const getSections = () => db.sections;
export const addTask = (period: string, length?: number, event?: string): void => db.addTask(period, length, event);
export const addEvent = (event: string): void => db.addEvent(event);
export const getDiagramTitle = () => db.title ?? '';

export function getTasks() {
  return db.periods.map((period) => ({
    section: period.section,
    type: period.section,
    task: period.text,
    score: 0,
    events: period.events,
  }));
}

export function setLogLevel(_level: string): void {}

export const parser = {
  yy: undefined as unknown,
  parse(src: string): void {
    parseTimeline(src, parser.yy as TimelineDb);
  },
};
