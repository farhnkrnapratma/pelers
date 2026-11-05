import type { Diagram } from '../../types.js';
import { JourneyDb, type JourneyModel } from './db.js';
import { parseJourney } from './parser.js';
import { renderJourney } from './render.js';

export const journey: Diagram<JourneyModel> = {
  type: 'journey',
  parse(source, _config, title) {
    const db = new JourneyDb();
    db.title = title;
    parseJourney(source, db);
    return db;
  },
  render: renderJourney,
};
