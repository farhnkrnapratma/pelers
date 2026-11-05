import { syntaxError } from '../../errors.js';
import type { DataPoint, XyChartDb, XyText } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';

function flags(...tokens: number[]): Uint8Array {
  const out = new Uint8Array(T.EOF + 1);
  for (const t of tokens) out[t] = 1;
  return out;
}

const WORD = flags(T.AMP, T.NUM, T.ALPHA, T.PLUS, T.EQUALS, T.MULT, T.DOT, T.BRKT, T.MINUS, T.UNDERSCORE);
const EOL = flags(T.NEWLINE, T.SEMI, T.EOF);

export function parseXyChart(src: string, db: XyChartDb): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('xychart', src, starts[i], '', true);
    throw syntaxError('xychart', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };

  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };

  const isText = (t: number): boolean => t === T.STR || WORD[t] === 1;

  const text = (): XyText => {
    if (!isText(types[i])) fail("'STR', 'ALPHA', 'NUM'");
    let s = texts[i++];
    if (types[i - 1] !== T.STR) while (WORD[types[i]] === 1) s += texts[i++];
    return { text: s, type: 'text' };
  };

  const range = (): [number, number] => {
    const min = Number(expect(T.NUMBER_WITH_DECIMAL));
    expect(T.ARROW_DELIMITER);
    return [min, Number(expect(T.NUMBER_WITH_DECIMAL))];
  };

  const points = (): DataPoint[] => {
    const out: DataPoint[] = [];
    expect(T.SQUARE_BRACES_START);
    for (;;) {
      const value = Number(expect(T.NUMBER_WITH_DECIMAL));
      out.push({ value, label: types[i] === T.STR ? texts[i++] : '' });
      if (types[i] !== T.COMMA) break;
      i++;
    }
    expect(T.SQUARE_BRACES_END);
    return out;
  };

  for (;;) {
    if (EOL[types[i]] === 1) i++;
    else if (types[i] !== T.XYCHART) break;
    else if (types[++i] === T.CHART_ORIENTATION) db.setOrientation(texts[i++]);
  }

  while (types[i] !== T.END) {
    const t = types[i++];
    switch (t) {
      case T.title:
        db.setDiagramTitle(text().text.trim());
        break;
      case T.X_AXIS:
      case T.Y_AXIS: {
        const title = isText(types[i]) ? text() : undefined;
        if (t === T.X_AXIS && types[i] === T.SQUARE_BRACES_START) {
          const categories: XyText[] = [];
          do {
            i++;
            categories.push(text());
          } while (types[i] === T.COMMA);
          expect(T.SQUARE_BRACES_END);
          db.setXAxisBand(categories);
        } else if (types[i] === T.NUMBER_WITH_DECIMAL || title === undefined) {
          const [min, max] = range();
          if (t === T.X_AXIS) db.setXAxisRangeData(min, max);
          else db.setYAxisRangeData(min, max);
        }
        if (t === T.X_AXIS) db.setXAxisTitle(title ?? { type: 'text', text: '' });
        else db.setYAxisTitle(title ?? { type: 'text', text: '' });
        break;
      }
      case T.LINE:
      case T.BAR: {
        const title: XyText = types[i] === T.SQUARE_BRACES_START ? { text: '', type: 'text' } : text();
        const data = points();
        if (t === T.LINE) db.setLineData(title, data);
        else db.setBarData(title, data);
        break;
      }
      case T.acc_title:
        db.setAccTitle(expect(T.acc_title_value).trim());
        break;
      case T.acc_descr:
        db.setAccDescription(expect(T.acc_descr_value).trim());
        break;
      case T.acc_descr_multiline_value:
        db.setAccDescription(texts[i - 1].trim());
        break;
      default:
        i--;
        fail("'title', 'X_AXIS', 'Y_AXIS', 'LINE', 'BAR', 'acc_title', 'acc_descr', 'acc_descr_multiline_value'");
    }
    while (EOL[types[i]] === 1) i++;
  }
}
