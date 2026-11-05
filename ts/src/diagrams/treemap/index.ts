import type { Diagram } from '../../types.js';
import { buildTreemap, type TreemapModel } from './model.js';
import { parseTreemap } from './parser.js';
import { renderTreemap } from './render.js';

export const treemap: Diagram<TreemapModel> = {
  type: 'treemap',
  parse: (source, _config, title) => buildTreemap(parseTreemap(source), title),
  render: renderTreemap,
};
