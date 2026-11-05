import type { Diagram } from '../../types.js';
import { buildWardley, type WardleyModel } from './model.js';
import { parseWardley } from './parser.js';
import { renderWardley } from './render.js';

export const wardley: Diagram<WardleyModel> = {
  type: 'wardley',
  section: 'wardley-beta',
  parse: (source, _config, title) => buildWardley(parseWardley(source), title),
  render: renderWardley,
};
