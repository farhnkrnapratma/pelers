import {
  ACC_DESCR,
  ACC_TITLE,
  DIRECTIVE,
  NEWLINE,
  Reader,
  SINGLE_LINE_COMMENT,
  STRING,
  TITLE,
  WHITESPACE,
  YAML,
  accDescrValue,
  accTitleValue,
  keyword,
  stringValue,
  titleValue,
  tokenize,
  type Scanner,
  type TokenType,
} from '../common/tokens.js';

const enum T {
  showData,
  pie,
  colon,
  number,
  accDescr,
  accTitle,
  title,
  string,
  newline,
}

function digits(src: string, from: number): number {
  let i = from;
  for (let c = src.charCodeAt(i); c >= 48 && c <= 57; c = src.charCodeAt(i)) i++;
  return i;
}

// Answers as NUMBER_PIE's pattern does. The pattern reads a whole run of digits before it finds
// there is no fraction, and would do so again from every digit of a long run of zeros.
function numberScanner(): Scanner {
  let from = 0;
  let end = 0;
  let fraction = -1;
  return (src, p) => {
    const at = src.charCodeAt(p) === 45 ? p + 1 : p;
    if (at < from || at >= end) {
      from = at;
      end = digits(src, at);
      fraction = -1;
      if (end > at && src.charCodeAt(end) === 46) {
        const stop = digits(src, end + 1);
        const length = stop - end - 1;
        if (length > 0 && src.charCodeAt(stop) !== 46) fraction = stop;
        else if (length > 1) fraction = stop - 1;
      }
    }
    if (end === at) return -1;
    if (fraction >= 0) return fraction;
    if (src.charCodeAt(at) === 48) return src.charCodeAt(at + 1) === 46 ? -1 : at + 1;
    if (src.charCodeAt(end) !== 46) return end;
    return end - 1 > at ? end - 1 : -1;
  };
}

export const PIE_TOKENS: readonly TokenType[] = [
  keyword('showData'),
  keyword('pie'),
  { name: ':', pattern: ':' },
  {
    name: 'NUMBER_PIE',
    pattern: /(?:-?[0-9]+\.[0-9]+(?!\.))|(?:-?(0|[1-9][0-9]*)(?!\.))/y,
    scanner: numberScanner,
    first: '-0123456789',
  },
  ACC_DESCR,
  ACC_TITLE,
  TITLE,
  STRING,
  NEWLINE,
  WHITESPACE,
  YAML,
  DIRECTIVE,
  SINGLE_LINE_COMMENT,
];

export interface PieAst {
  $type: 'Pie';
  showData: boolean;
  title?: string;
  accTitle?: string;
  accDescr?: string;
  sections: { $type: 'PieSection'; label: string; value: number }[];
}

export function parsePie(src: string): PieAst {
  const r = new Reader(tokenize(src, PIE_TOKENS, 'pie'), PIE_TOKENS, 'pie');
  const ast: PieAst = { $type: 'Pie', showData: false, sections: [] };

  const endOfLine = (): void => {
    if (r.kind === -1) return;
    if (r.kind !== T.newline) r.fail('a line break or the end of input');
    while (r.kind === T.newline) r.i++;
  };

  while (r.kind === T.newline) r.i++;
  r.expect(T.pie);
  ast.showData = r.accept(T.showData);
  while (r.kind !== -1) {
    switch (r.kind) {
      case T.newline:
        r.i++;
        break;
      case T.accDescr:
        ast.accDescr = accDescrValue(r.take());
        endOfLine();
        break;
      case T.accTitle:
        ast.accTitle = accTitleValue(r.take());
        endOfLine();
        break;
      case T.title:
        ast.title = titleValue(r.take());
        endOfLine();
        break;
      case T.string: {
        const label = stringValue(r.take());
        r.expect(T.colon);
        const value = Number(r.expect(T.number));
        ast.sections.push({ $type: 'PieSection', label, value });
        endOfLine();
        break;
      }
      default:
        r.fail('a section, a title, or a line break');
    }
  }
  return ast;
}
