import type { Diagram } from '../../types.js';
import { IshikawaDb } from './db.js';
import { parseIshikawa } from './parser.js';
import { renderIshikawa } from './render.js';

export const ishikawa: Diagram<IshikawaDb> = {
  type: 'ishikawa',
  parse(source, _config, title) {
    const db = new IshikawaDb();
    db.title = title;
    parseIshikawa(source, db);
    return db;
  },
  render: renderIshikawa,
};
