import { PeleError } from '../../../src/errors.js';
import { parseOrderedJson } from '../../../src/diagrams/usecase/json.js';
import { lineColumn, tokenize } from '../../../src/diagrams/usecase/lexer.js';
import { parseUsecase } from '../../../src/diagrams/usecase/parser.js';
import { T, TOKEN_NAMES } from '../../../src/diagrams/usecase/tokens.js';
import { record } from '../../support/usecase-ast.js';

interface Token {
  image: string;
  tokenType: { name: string };
  startOffset: number;
  startLine: number;
  startColumn: number;
  endLine: number;
}

interface Lexed {
  tokens: Token[] & { source?: string };
  errors: unknown[];
}

// Pele's tokens as the objects Chevrotain's lexer returns.
export const usecaseLexer = {
  tokenize(source: string): Lexed {
    const tokens: Lexed['tokens'] = [];
    tokens.source = source;
    try {
      const { types, starts, ends } = tokenize(source);
      for (let i = 0; types[i] !== T.EOF; i++) {
        const [startLine, startColumn] = lineColumn(source, starts[i]);
        tokens.push({
          image: source.slice(starts[i], ends[i]),
          tokenType: { name: TOKEN_NAMES[types[i]] },
          startOffset: starts[i],
          startLine,
          startColumn,
          endLine: lineColumn(source, ends[i] - 1)[0],
        });
      }
    } catch (error) {
      return { tokens, errors: [error] };
    }
    return { tokens, errors: [] };
  },
};

// The specs feed tokens to the Chevrotain parser and read its syntax errors. Pele parses and
// resolves in one pass, so an error from the resolving step means the grammar accepted the text.
export const usecaseParser = {
  source: '',
  errors: [] as unknown[],
  set input(tokens: Lexed['tokens']) {
    this.source = tokens.source ?? '';
    this.errors = [];
  },
  start(): { name: string } {
    try {
      parseUsecase(this.source);
    } catch (error) {
      if (error instanceof PeleError && /^Error (?:parsing|lexing) /.test(error.message)) this.errors = [error];
      else if (!(error instanceof PeleError)) throw error;
    }
    return { name: 'start' };
  },
};

export class UsecaseJsonError extends Error {
  constructor(
    message: string,
    readonly line: number,
    readonly column: number
  ) {
    super(message);
    this.name = 'UsecaseJsonError';
  }
}

export function parseOrderedJsonObject(text: string, line: number, column: number) {
  try {
    const { value, propertyOrder } = parseOrderedJson(text, () => [line, column]);
    return { value, propertyOrder: record(propertyOrder) };
  } catch (error) {
    if (error instanceof PeleError) throw new UsecaseJsonError(error.message, error.line, error.column);
    throw error;
  }
}
