import { GanttDb, parseDuration } from '../../../src/diagrams/gantt/db.js';
import { parseGantt } from '../../../src/diagrams/gantt/parser.js';

export const log = {
  debug: (..._args: unknown[]): void => {},
  info: (..._args: unknown[]): void => {},
  warn: (..._args: unknown[]): void => {},
  error: (..._args: unknown[]): void => {},
};

const options = { warn: (message: string) => log.warn(message) };

// Mermaid's gantt database is a module-level singleton that `clear()` resets. This one holds a Pele
// model and exposes it through the names the specs call.
class GanttDB extends GanttDb {
  constructor() {
    super(options);
  }

  clear(): void {
    Object.assign(this, new GanttDb(options));
  }

  parseDuration = parseDuration;

  getDateFormat() {
    return this.dateFormat;
  }

  getAxisFormat() {
    return this.axisFormat;
  }

  getTodayMarker() {
    return this.todayMarker;
  }

  getExcludes() {
    return this.excludes;
  }

  getIncludes() {
    return this.includes;
  }

  getSections() {
    return this.sections;
  }

  endDatesAreInclusive() {
    return this.inclusiveEndDates;
  }

  setDisplayMode(mode: string): void {
    this.displayMode = mode;
  }

  getDisplayMode() {
    return this.displayMode;
  }

  getWeekday() {
    return this.weekday ?? 'sunday';
  }

  getAccTitle() {
    return this.accTitle ?? '';
  }

  getAccDescription() {
    return this.accDescr ?? '';
  }
}

export const ganttDb = new GanttDB();

export const parser = {
  yy: ganttDb,
  parse(src: string): void {
    parseGantt(src, parser.yy);
  },
};

// The specs use dayjs only to write expected dates in two fixed formats. Pele has no dayjs to depend on,
// so this reads those formats with the Date constructor, independently of the code under test.
export function dayjs(text: string, format: string): { toDate(): Date } {
  const pattern = format === 'YYYY-MM-DD' ? /^(\d{4})-(\d\d)-(\d\d)$/ : /^(\d{4})-(\d\d)-(\d\d) (\d\d):(\d\d):(\d\d)$/;
  if (format !== 'YYYY-MM-DD' && format !== 'YYYY-MM-DD HH:mm:ss') throw new Error(`Unexpected format ${format}`);
  const m = pattern.exec(text);
  if (!m) throw new Error(`Unexpected date ${text}`);
  const [year, month, day, hours = 0, minutes = 0, seconds = 0] = m.slice(1).map(Number);
  return { toDate: () => new Date(year, month - 1, day, hours, minutes, seconds) };
}

// Mermaid's helper that turns a tagged template table into the rows `it.each` takes.
export const convert = (template: TemplateStringsArray, ...params: unknown[]) => {
  const header = template[0]
    .trim()
    .split('|')
    .map((s) => s.trim());
  if (header.length === 0 || params.length % header.length !== 0) {
    throw new Error('Table column count mismatch');
  }
  const out = [];
  for (let i = 0; i < params.length; i += header.length) {
    const chunk = params.slice(i, i + header.length);
    out.push(Object.fromEntries(chunk.map((v, j) => [header[j], v])));
  }
  return out;
};
