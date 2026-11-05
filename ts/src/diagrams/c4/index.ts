import type { Diagram } from '../../types.js';
import { C4Db } from './db.js';
import { parseC4 } from './parser.js';
import { renderC4 } from './render.js';

export const c4: Diagram<C4Db> = {
  type: 'c4',
  parse(source, _config, title) {
    const db = new C4Db();
    if (title) db.title = title;
    parseC4(source, db);
    return db;
  },
  render: renderC4,
};
