import type { Diagram } from '../../types.js';
import { parseRailroad, type RailroadDb } from './db.js';
import { renderRailroad } from './render.js';

export const railroad: Diagram<RailroadDb> = {
  type: 'railroad',
  parse: (source, _config, title) => parseRailroad(source, title),
  render: renderRailroad,
};
