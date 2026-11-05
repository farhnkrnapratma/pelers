import type { Diagram } from '../../types.js';
import { buildRadar, type RadarModel } from './model.js';
import { parseRadar } from './parser.js';
import { renderRadar } from './render.js';

export const radar: Diagram<RadarModel> = {
  type: 'radar',
  parse: (source, _config, title) => buildRadar(parseRadar(source), title),
  render: renderRadar,
};
