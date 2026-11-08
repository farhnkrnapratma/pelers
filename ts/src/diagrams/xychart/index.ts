import type { Diagram } from '../../types.js';
import { XyChartDb } from './db.js';
import { parseXyChart } from './parser.js';
import { renderXyChart } from './render.js';

export const xychart: Diagram<XyChartDb> = {
  type: 'xychart',
  section: 'xyChart',
  parse(source, _config, title) {
    const db = new XyChartDb();
    db.title = title;
    parseXyChart(source, db);
    return db;
  },
  render: renderXyChart,
};
