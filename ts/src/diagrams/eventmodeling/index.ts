import type { Diagram } from '../../types.js';
import { buildEventModel, type EventModelingModel } from './model.js';
import { parseEventModel } from './parser.js';
import { renderEventModel } from './render.js';

export const eventmodeling: Diagram<EventModelingModel> = {
  type: 'eventmodeling',
  keepComments: true,
  parse: (source, _config, title) => buildEventModel(parseEventModel(source), title),
  render: renderEventModel,
};
