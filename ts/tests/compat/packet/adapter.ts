import { buildPacket, type PacketBlock, type PacketModel } from '../../../src/diagrams/packet/model.js';
import { parsePacket, type PacketAst } from '../../../src/diagrams/packet/parser.js';
import { PeleError } from '../../../src/errors.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';

export const Packet = { $type: 'Packet' };

export const packetParse = (src: string) => toResult<PacketAst>(parsePacket, src);

// Mermaid's parser fills a database object assigned to `parser.parser.yy`; this one holds the model.
export class PacketDB {
  model: PacketModel | undefined;

  getPacket(): PacketBlock[][] {
    return this.model?.rows ?? [];
  }

  getDiagramTitle(): string {
    return this.model?.title ?? '';
  }

  getAccTitle(): string {
    return this.model?.accTitle ?? '';
  }

  getAccDescription(): string {
    return this.model?.accDescr ?? '';
  }
}

export const parser = {
  parser: { yy: undefined as PacketDB | undefined },
  async parse(src: string): Promise<void> {
    try {
      const model = buildPacket(parsePacket(src), {}, undefined);
      if (parser.parser.yy) parser.parser.yy.model = model;
    } catch (error) {
      // The specs snapshot errors with their class name, which is `Error` in Mermaid.
      if (error instanceof PeleError) throw new Error(error.message);
      throw error;
    }
  },
};
