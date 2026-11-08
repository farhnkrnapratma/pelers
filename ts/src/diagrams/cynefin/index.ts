import type { Diagram } from '../../types.js';
import { buildCynefin, type CynefinModel } from './model.js';
import { parseCynefin } from './parser.js';
import { renderCynefin } from './render.js';

export const cynefin: Diagram<CynefinModel> = {
  type: 'cynefin',
  parse: (source, _config, title) => buildCynefin(parseCynefin(source), title),
  render: renderCynefin,
};
