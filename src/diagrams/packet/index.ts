import type { Diagram } from '../../types.js';
import { buildPacket, type PacketModel } from './model.js';
import { parsePacket } from './parser.js';
import { renderPacket } from './render.js';

export const packet: Diagram<PacketModel> = {
  type: 'packet',
  parse: (source, config, title) => buildPacket(parsePacket(source), config, title),
  render: renderPacket,
};
