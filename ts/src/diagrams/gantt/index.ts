import type { Config } from '../../preprocess.js';
import type { Diagram } from '../../types.js';
import { GanttDb } from './db.js';
import { parseGantt } from './parser.js';
import { renderGantt } from './render.js';

export const gantt: Diagram<GanttDb> = {
  type: 'gantt',
  parse(source, config, title) {
    const db = new GanttDb();
    if (title) db.title = title;
    const mode = (config.gantt as Config | undefined)?.displayMode;
    if (typeof mode === 'string') db.displayMode = mode;
    parseGantt(source, db);
    return db;
  },
  render: renderGantt,
};
