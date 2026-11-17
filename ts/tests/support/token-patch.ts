import * as reference from '@mermaid-js/parser';

// Match Pelers's lexer fixes in the reference parser.

export interface ReferenceToken {
  name: string;
  PATTERN: unknown;
}

interface Builder {
  buildTokens(...args: unknown[]): ReferenceToken[];
}

export function findToken(
  tokens: ReferenceToken[],
  name: string,
  source: string
): ReferenceToken & { PATTERN: RegExp } {
  const matches = tokens.filter((t) => t.name === name && t.PATTERN instanceof RegExp && t.PATTERN.source === source);
  if (matches.length !== 1) throw new Error(`Expected one ${name} token with pattern ${source}`);
  return matches[0] as ReferenceToken & { PATTERN: RegExp };
}

// Call before the module's services are first created.
export function patchTokens(module: string, patch: (tokens: ReferenceToken[]) => void): void {
  const parser = (reference as unknown as Record<string, { parser: { TokenBuilder: () => Builder } }>)[module].parser;
  const create = parser.TokenBuilder;
  parser.TokenBuilder = () => {
    const builder = create();
    const build = builder.buildTokens.bind(builder);
    builder.buildTokens = (...args) => {
      const tokens = build(...args);
      patch(tokens);
      return tokens;
    };
    return builder;
  };
}

const TITLE = String.raw`[\t ]*title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)`;

export function patchTitle(tokens: ReferenceToken[], rest: string, name = 'TITLE', source = TITLE): void {
  const title = findToken(tokens, name, source);
  title.PATTERN = new RegExp(source.replace(/\|\)$/, `|(?!${rest}))`), title.PATTERN.flags);
}
