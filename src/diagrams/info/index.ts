import type { Diagram } from '../../types.js';
import { buildInfo, type InfoModel } from './model.js';
import { parseInfo } from './parser.js';
import { renderInfo } from './render.js';

export const info: Diagram<InfoModel> = {
  type: 'info',
  parse: (source, _config, title) => buildInfo(parseInfo(source), title),
  render: renderInfo,
};
