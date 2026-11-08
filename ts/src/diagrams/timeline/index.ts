import type { Diagram } from '../../types.js';
import { TimelineDb, type TimelineModel } from './db.js';
import { parseTimeline } from './parser.js';
import { renderTimeline } from './render.js';

export const timeline: Diagram<TimelineModel> = {
  type: 'timeline',
  parse(source, _config, title) {
    const db = new TimelineDb();
    db.title = title;
    parseTimeline(source, db);
    return db;
  },
  render: renderTimeline,
};
