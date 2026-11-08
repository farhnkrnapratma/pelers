import { PeleError } from '../../errors.js';
import type { Config } from '../../preprocess.js';
import type { PacketAst } from './parser.js';

export interface PacketBlock {
  start: number;
  end: number;
  // As Mermaid counts it: one less than the width for the pieces of a field split across rows.
  bits: number;
  label: string;
}

export interface PacketModel {
  type: 'packet';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  bitsPerRow: number;
  rows: PacketBlock[][];
}

const MAX_ROWS = 10_000;

export function packetOption(config: Config, key: string, fallback: number, min: number, max: number): number {
  const value = (config.packet as Config | undefined)?.[key];
  return typeof value === 'number' && value >= min ? Math.min(value, max) : fallback;
}

function invalid(message: string): PeleError {
  return new PeleError(message, 'semantic', { type: 'packet' });
}

export function buildPacket(ast: PacketAst, config: Config, title: string | undefined): PacketModel {
  const bitsPerRow = Math.floor(packetOption(config, 'bitsPerRow', 32, 1, 4096));
  const rows: PacketBlock[][] = [];
  let lastBit = -1;
  let word: PacketBlock[] = [];
  let row = 1;

  for (const block of ast.blocks) {
    let { start, end, bits } = block;
    if (start !== undefined && end !== undefined && end < start) {
      throw invalid(`Packet block ${start} - ${end} is invalid. End must be greater than start.`);
    }
    start ??= lastBit + 1;
    if (start !== lastBit + 1) {
      throw invalid(`Packet block ${start} - ${end ?? start} is not contiguous. It should start from ${lastBit + 1}.`);
    }
    if (bits === 0) {
      throw invalid(`Packet block ${start} is invalid. Cannot have a zero bit field.`);
    }
    end ??= start + (bits ?? 1) - 1;
    bits ??= end - start + 1;
    lastBit = end;

    while (word.length <= bitsPerRow + 1 && rows.length < MAX_ROWS) {
      const rowEnd = row * bitsPerRow - 1;
      if (end <= rowEnd) {
        word.push({ start, end, bits, label: block.label });
        if (end === rowEnd) {
          rows.push(word);
          word = [];
          row++;
        }
        break;
      }
      word.push({ start, end: rowEnd, bits: rowEnd - start, label: block.label });
      rows.push(word);
      word = [];
      row++;
      start = rowEnd + 1;
      bits = end - start;
    }
  }
  if (word.length > 0) rows.push(word);

  return {
    type: 'packet',
    title: ast.title || title,
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : undefined,
    bitsPerRow,
    rows,
  };
}
