import { syntaxError } from '../../errors.js';
import type { QuadrantDb, QuadrantText } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';

function flags(...tokens: number[]): Uint8Array {
  const out = new Uint8Array(T.EOF + 1);
  for (const t of tokens) out[t] = 1;
  return out;
}

const WORD = flags(
  T.PUNCTUATION,
  T.AMP,
  T.NUM,
  T.ALPHA,
  T.COMMA,
  T.PLUS,
  T.EQUALS,
  T.MULT,
  T.DOT,
  T.BRKT,
  T.UNDERSCORE,
  T.UNICODE_TEXT
);
const ID = flags(T.ALPHA, T.NUM, T.MINUS, T.COMMA, T.COLON, T.AMP, T.BRKT, T.MULT, T.UNICODE_TEXT);
const STYLE = flags(T.ALPHA, T.NUM, T.COLON, T.SPACE, T.BRKT, T.MINUS);
const EOL = flags(T.NEWLINE, T.SEMI, T.EOF);

const QUADRANT_SETTERS = ['setQuadrant1Text', 'setQuadrant2Text', 'setQuadrant3Text', 'setQuadrant4Text'] as const;
const TEXT_TOKENS = "'STR', 'MD_STR', 'ALPHA', 'NUM', 'UNICODE_TEXT', 'PUNCTUATION'";

export function parseQuadrant(src: string, db: QuadrantDb): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('quadrantChart', src, starts[i], '', true);
    throw syntaxError('quadrantChart', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };

  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };

  const isText = (t: number): boolean => t === T.STR || t === T.MD_STR || WORD[t] === 1;

  const text = (): QuadrantText => {
    const first = types[i];
    if (!isText(first)) fail(TEXT_TOKENS);
    let s = texts[i++];
    while (WORD[types[i]] === 1 || types[i] === T.SPACE || types[i] === T.MINUS) s += texts[i++];
    return { text: s, type: first === T.MD_STR ? 'markdown' : 'text' };
  };

  const styles = (): string[] => {
    const out: string[] = [];
    for (;;) {
      if (STYLE[types[i]] === 0) fail("'ALPHA', 'NUM', 'COLON', 'SPACE', 'BRKT', 'MINUS'");
      let s = texts[i++];
      while (STYLE[types[i]] === 1) s += texts[i++];
      out.push(s.trim());
      if (types[i] !== T.COMMA) return out;
      i++;
    }
  };

  while (EOL[types[i]] === 1 || types[i] === T.SPACE) i++;
  expect(T.QUADRANT);

  while (types[i] !== T.END) {
    while (types[i] === T.SPACE) i++;
    const t = types[i];
    switch (t) {
      case T.CLASSDEF: {
        i++;
        expect(T.SPACE);
        if (ID[types[i]] === 0) fail("'ALPHA', 'NUM', 'UNICODE_TEXT'");
        let id = texts[i++];
        while (ID[types[i]] === 1) id += texts[i++];
        expect(T.SPACE);
        db.addClass(id, styles());
        break;
      }
      case T.X_AXIS:
      case T.Y_AXIS: {
        i++;
        const from = text();
        let to: QuadrantText | undefined;
        if (types[i] === T.DELIMITER) {
          i++;
          if (isText(types[i])) to = text();
          else from.text += ' ⟶ ';
        }
        if (t === T.X_AXIS) {
          db.setXAxisLeftText(from);
          if (to) db.setXAxisRightText(to);
        } else {
          db.setYAxisBottomText(from);
          if (to) db.setYAxisTopText(to);
        }
        break;
      }
      case T.QUADRANT_1:
      case T.QUADRANT_2:
      case T.QUADRANT_3:
      case T.QUADRANT_4:
        i++;
        db[QUADRANT_SETTERS[t - T.QUADRANT_1]](text());
        break;
      case T.title:
        i++;
        db.setDiagramTitle(expect(T.title_value).trim());
        break;
      case T.acc_title:
        i++;
        db.setAccTitle(expect(T.acc_title_value).trim());
        break;
      case T.acc_descr:
        i++;
        db.setAccDescription(expect(T.acc_descr_value).trim());
        break;
      case T.acc_descr_multiline_value:
        db.setAccDescription(texts[i++].trim());
        break;
      default:
        if (isText(t)) {
          const label = text();
          const className = types[i] === T.class_name ? texts[i++] : '';
          expect(T.point_start);
          const x = expect(T.point_x);
          const y = expect(T.point_y);
          db.addPoint(label, className, x, y, STYLE[types[i]] === 1 ? styles() : []);
        }
    }
    if (EOL[types[i]] === 0) fail("'NEWLINE', 'SEMI', 'EOF'");
    i++;
  }
}
