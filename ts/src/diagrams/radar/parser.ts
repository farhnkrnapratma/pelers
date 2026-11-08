import {
  ACC_DESCR,
  ACC_TITLE,
  DIRECTIVE,
  NEWLINE,
  Reader,
  SINGLE_LINE_COMMENT,
  STRING,
  WHITESPACE,
  YAML,
  accDescrValue,
  accTitleValue,
  keyword,
  stringValue,
  titleBefore,
  titleValue,
  tokenize,
  type Scanner,
  type TokenType,
  type Tokens,
} from '../common/tokens.js';

const enum T {
  radarColon,
  radar,
  showLegend,
  graticule,
  curve,
  ticks,
  axis,
  max,
  min,
  colon,
  comma,
  open,
  close,
  openBrace,
  closeBrace,
  graticuleValue,
  boolean,
  accDescr,
  accTitle,
  title,
  number,
  string,
  id,
  newline,
}

let runFrom = 0;
let runEnd = 0;

function digits(src: string, at: number): number {
  let end = at;
  for (let c = src.charCodeAt(end); c >= 48 && c <= 57; c = src.charCodeAt(end)) end++;
  return end;
}

// The NUMBER pattern without its backtracking. As a regular expression it reads to the end of a
// run of digits from every position in the run, which is quadratic for a long run of zeros.
function matchNumber(src: string, at: number): number {
  if (at < runFrom || at >= runEnd) {
    runFrom = at;
    runEnd = digits(src, at);
  }
  const whole = runEnd;
  if (whole === at) return -1;
  const dot = src.charCodeAt(whole) === 46;
  if (dot) {
    const fraction = digits(src, whole + 1);
    const length = fraction - whole - 1;
    if (length > 0 && src.charCodeAt(fraction) !== 46) return fraction;
    if (length > 1) return fraction - 1;
  }
  if (src.charCodeAt(at) === 48) return at + 1;
  if (!dot) return whole;
  return whole - 1 > at ? whole - 1 : -1;
}

const RE_ACC_DESCR = ACC_DESCR.pattern as RegExp;
const RE_BLOCK = /[\t ]*accDescr\s*{/y;
let lastBrace = -2;

// In the other grammars an `accDescr {` that never closes ends the lexing. Here `accDescr` is
// also a name and `{` a token, so each unclosed one would read to the end of the text again.
function matchAccDescr(src: string, at: number): number {
  RE_BLOCK.lastIndex = at;
  if (RE_BLOCK.test(src)) {
    if (lastBrace === -2) lastBrace = src.lastIndexOf('}');
    if (RE_BLOCK.lastIndex > lastBrace) return -1;
  }
  RE_ACC_DESCR.lastIndex = at;
  return RE_ACC_DESCR.test(src) ? RE_ACC_DESCR.lastIndex : -1;
}

const ID: TokenType = { name: 'ID', pattern: /[\w]([-\w]*\w)?/y };
const RE_NAME_REST = /[-\w]*\w/y;

// Gives way to a longer name, except right after its option, as in `graticule polygonaxis`.
function optionValue(pattern: RegExp, option: T): Scanner {
  return (src, at, previous) => {
    pattern.lastIndex = at;
    if (!pattern.test(src)) return -1;
    const end = pattern.lastIndex;
    if (previous === option) return end;
    RE_NAME_REST.lastIndex = end;
    return RE_NAME_REST.test(src) ? -1 : end;
  };
}

const word = (text: string): TokenType => ({ name: text, pattern: text, longer: [ID] });
const mark = (text: string): TokenType => ({ name: text, pattern: text });

export const RADAR_TOKENS: readonly TokenType[] = [
  mark('radar-beta:'),
  { ...keyword('radar-beta'), longer: [ID] },
  word('showLegend'),
  word('graticule'),
  word('curve'),
  word('ticks'),
  word('axis'),
  word('max'),
  word('min'),
  mark(':'),
  mark(','),
  mark('['),
  mark(']'),
  mark('{'),
  mark('}'),
  { name: 'GRATICULE', pattern: /circle|polygon/y, match: optionValue(/circle|polygon/y, T.graticule) },
  { name: 'BOOLEAN', pattern: /true|false/y, match: optionValue(/true|false/y, T.showLegend) },
  { ...ACC_DESCR, match: matchAccDescr },
  ACC_TITLE,
  titleBefore(String.raw`[-\w]*\w`),
  { name: 'NUMBER', pattern: /(?:[0-9]+\.[0-9]+(?!\.))|(?:0|[1-9][0-9]*(?!\.))/y, match: matchNumber },
  STRING,
  ID,
  NEWLINE,
  WHITESPACE,
  YAML,
  DIRECTIVE,
  SINGLE_LINE_COMMENT,
];

export interface AxisAst {
  $type: 'Axis';
  name: string;
  label?: string;
}

export interface EntryAst {
  $type: 'Entry';
  // Langium leaves the reference unresolved; Mermaid reads only its text.
  axis?: { $refText: string };
  value: number;
}

export interface CurveAst {
  $type: 'Curve';
  name: string;
  label?: string;
  entries: EntryAst[];
}

export interface OptionAst {
  $type: 'Option';
  name: 'showLegend' | 'ticks' | 'max' | 'min' | 'graticule';
  value: boolean | number | string;
}

export interface RadarAst {
  $type: 'Radar';
  title?: string;
  accTitle?: string;
  accDescr?: string;
  axes: AxisAst[];
  curves: CurveAst[];
  options: OptionAst[];
}

export function lexRadar(src: string): Tokens {
  runFrom = runEnd = 0;
  lastBrace = -2;
  return tokenize(src, RADAR_TOKENS, 'radar');
}

export function parseRadar(src: string): RadarAst {
  const r = new Reader(lexRadar(src), RADAR_TOKENS, 'radar');
  const ast: RadarAst = { $type: 'Radar', axes: [], curves: [], options: [] };

  const newlines = (): void => {
    while (r.kind === T.newline) r.i++;
  };

  const endOfLine = (): void => {
    if (r.kind === -1) return;
    if (r.kind !== T.newline) r.fail('a line break or the end of input');
    newlines();
  };

  const label = (): string | undefined => {
    if (!r.accept(T.open)) return undefined;
    const text = stringValue(r.expect(T.string));
    r.expect(T.close);
    return text;
  };

  const entry = (detailed: boolean): EntryAst => {
    if (!detailed) return { $type: 'Entry', value: Number(r.expect(T.number)) };
    const axis = { $refText: r.expect(T.id) };
    r.accept(T.colon);
    return { $type: 'Entry', axis, value: Number(r.expect(T.number)) };
  };

  const curve = (): CurveAst => {
    const name = r.expect(T.id);
    const text = label();
    const node: CurveAst = { $type: 'Curve', name, entries: [] };
    if (text !== undefined) node.label = text;
    r.expect(T.openBrace);
    newlines();
    if (r.kind !== T.number && r.kind !== T.id) r.fail('a number or an axis name');
    const detailed = r.kind === T.id;
    node.entries.push(entry(detailed));
    while (r.accept(T.comma)) {
      newlines();
      node.entries.push(entry(detailed));
    }
    newlines();
    r.expect(T.closeBrace);
    return node;
  };

  const axis = (): AxisAst => {
    const node: AxisAst = { $type: 'Axis', name: r.expect(T.id) };
    const text = label();
    if (text !== undefined) node.label = text;
    return node;
  };

  const option = (): OptionAst => {
    const kind = r.kind;
    const name = r.take() as OptionAst['name'];
    let value: OptionAst['value'];
    if (kind === T.showLegend) value = r.expect(T.boolean) === 'true';
    else if (kind === T.graticule) value = r.expect(T.graticuleValue);
    else value = Number(r.expect(T.number));
    return { $type: 'Option', name, value };
  };

  const isOption = (): boolean =>
    r.kind === T.showLegend || r.kind === T.ticks || r.kind === T.max || r.kind === T.min || r.kind === T.graticule;

  newlines();
  if (!r.accept(T.radarColon)) {
    r.expect(T.radar);
    r.accept(T.colon);
  }
  newlines();
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
      case T.axis:
        r.i++;
        do ast.axes.push(axis());
        while (r.accept(T.comma));
        break;
      case T.curve:
        r.i++;
        do ast.curves.push(curve());
        while (r.accept(T.comma));
        break;
      default:
        if (!isOption()) r.fail('an axis, a curve, an option, a title, or a line break');
        ast.options.push(option());
        while (r.accept(T.comma)) {
          if (!isOption()) r.fail('an option');
          ast.options.push(option());
        }
    }
  }
  return ast;
}
