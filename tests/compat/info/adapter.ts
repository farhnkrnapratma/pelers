import { parseInfo, type InfoAst } from '../../../src/diagrams/info/parser.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';

export const Info = { $type: 'Info' };

export const infoParse = (src: string) => toResult<InfoAst>(parseInfo, src);

export const parser = {
  async parse(src: string): Promise<void> {
    parseInfo(src);
  },
};
