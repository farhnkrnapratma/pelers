import type { Diagram } from '../../types.js';
import { VennDb } from './db.js';
import { parseVenn } from './parser.js';
import { renderVenn } from './render.js';

export const venn: Diagram<VennDb> = {
  type: 'venn',
  parse(source, _config, title) {
    const db = new VennDb();
    if (title) db.title = title;
    parseVenn(source, db);
    return db;
  },
  render: renderVenn,
};
