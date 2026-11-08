import type { Diagram } from '../../types.js';
import { SankeyDb } from './db.js';
import { parseSankey, prepareTextForParsing } from './parser.js';
import { renderSankey } from './render.js';

export const sankey: Diagram<SankeyDb> = {
  type: 'sankey',
  parse(source, _config, title) {
    const db = new SankeyDb();
    if (title) db.title = title;
    parseSankey(prepareTextForParsing(source), db, source);
    return db;
  },
  render: renderSankey,
};
