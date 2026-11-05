import { syntaxError } from '../../errors.js';
import type { ClassDb } from './db.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import type { RelationEnd, ClassRelation } from './types.js';

const COUNT = T.PUNCTUATION + 1;

const ALPHANUM = new Uint8Array(COUNT);
for (const t of [T.UNICODE_TEXT, T.ALPHA, T.MINUS]) ALPHANUM[t] = 1;

const STYLE_PART = new Uint8Array(COUNT);
for (const t of [T.ALPHA, T.COLON, T.BRKT, T.STYLE, T.PCT, T.LABEL]) STYLE_PART[t] = 1;

// Mermaid's relationType numbers, plus one so that zero means "not an end marker".
const END_TYPE = new Uint8Array(COUNT);
END_TYPE[T.AGGREGATION] = 1;
END_TYPE[T.EXTENSION] = 2;
END_TYPE[T.COMPOSITION] = 3;
END_TYPE[T.DEPENDENCY] = 4;
END_TYPE[T.LOLLIPOP] = 5;

const NAME_TOKENS = "'UNICODE_TEXT', 'NUM', 'ALPHA', 'MINUS', 'BQUOTE_STR'";

interface Frame {
  id: string;
  classes: string[];
  notes: string[];
}

export function parseClassDiagram(src: string, db: ClassDb): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('class', src, starts[i], '', true);
    throw syntaxError('class', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };

  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };

  const alphaNum = (): string => {
    if (ALPHANUM[types[i]] === 0) fail("'UNICODE_TEXT', 'NUM', 'ALPHA', 'MINUS'");
    return texts[i++];
  };

  const name = (generic: boolean): string => {
    let out = '';
    while (true) {
      if (types[i] === T.BQUOTE_STR) {
        out += texts[i++];
        break;
      }
      if (ALPHANUM[types[i]] === 0) fail(NAME_TOKENS);
      out += texts[i++];
      const t = types[i];
      if (t === T.DOT) {
        out += '.';
        i++;
      } else if (ALPHANUM[t] === 0 && t !== T.BQUOTE_STR) {
        break;
      }
    }
    if (generic && types[i] === T.GENERICTYPE) out += '~' + texts[i++] + '~';
    return out;
  };

  const classLabel = (): string => {
    i++;
    const label = expect(T.STR);
    expect(T.SQE);
    return label;
  };

  const members = (): string[] => {
    if (types[i] !== T.MEMBER) fail("'STRUCT_STOP', 'MEMBER'");
    const list: string[] = [];
    while (types[i] === T.MEMBER) list.push(texts[i++]);
    expect(T.STRUCT_STOP);
    // The grammar collects members last to first, and addMembers turns them back.
    return list.reverse();
  };

  const classStatement = (): string => {
    i++;
    const id = name(true);
    db.addClass(id);
    if (types[i] === T.SQS) db.setClassLabel(id, classLabel());
    switch (types[i]) {
      case T.STYLE_SEPARATOR: {
        i++;
        const css = alphaNum();
        db.setCssClass(id, css);
        if (types[i] === T.STRUCT_START) {
          i++;
          db.addMembers(id, members());
        }
        break;
      }
      case T.STRUCT_START:
        i++;
        if (types[i] === T.STRUCT_STOP) i++;
        else db.addMembers(id, members());
        break;
      case T.ANNOTATION_START: {
        i++;
        const annotation = alphaNum();
        expect(T.ANNOTATION_END);
        db.addAnnotation(id, annotation);
        if (types[i] === T.STRUCT_START) {
          i++;
          if (types[i] === T.STRUCT_STOP) i++;
          else db.addMembers(id, members());
        }
        break;
      }
    }
    return id;
  };

  const noteStatement = (): string => {
    if (types[i++] === T.NOTE) return db.addNote(expect(T.STR));
    const id = name(true);
    return db.addNote(expect(T.STR), id);
  };

  // Namespaces nest, so they are parsed with a stack of open ones.
  const namespaceStatement = (): void => {
    const open: Frame[] = [];
    while (true) {
      i++;
      const ns = name(false);
      const id = types[i] === T.SQS ? db.addNamespace(ns, classLabel()) : db.addNamespace(ns);
      expect(T.STRUCT_START);
      if (types[i] === T.NEWLINE) i++;
      open.push({ id, classes: [], notes: [] });
      let item = true;
      while (item) {
        const frame = open[open.length - 1];
        const t = types[i];
        if (t === T.NAMESPACE) break;
        if (t === T.CLASS) frame.classes.push(classStatement());
        else if (t === T.NOTE || t === T.NOTE_FOR) frame.notes.push(noteStatement());
        else fail("'NAMESPACE', 'CLASS', 'NOTE_FOR', 'NOTE'");
        while (true) {
          if (types[i] === T.NEWLINE) {
            const next = types[i + 1];
            if (next === T.NAMESPACE || next === T.CLASS || next === T.NOTE || next === T.NOTE_FOR) {
              i++;
              break;
            }
            i++;
          }
          expect(T.STRUCT_STOP);
          const done = open.pop()!;
          db.addClassesToNamespace(done.id, done.classes, done.notes);
          db.popNamespace();
          if (open.length === 0) {
            item = false;
            break;
          }
        }
      }
      if (open.length === 0) return;
    }
  };

  const relationEnd = (): RelationEnd => {
    const type = END_TYPE[types[i]];
    if (type === 0) return 'none';
    i++;
    return type - 1;
  };

  const classOrRelation = (): void => {
    const id1 = name(true);
    let t = types[i];
    if (t === T.LABEL) {
      db.addMember(id1, db.cleanupLabel(texts[i++]));
      return;
    }
    let title1 = 'none';
    if (t === T.STR) {
      title1 = texts[i++];
      t = types[i];
    } else if (END_TYPE[t] === 0 && t !== T.LINE && t !== T.DOTTED_LINE) {
      return;
    }
    const type1 = relationEnd();
    t = types[i];
    if (t !== T.LINE && t !== T.DOTTED_LINE) {
      fail("'AGGREGATION', 'EXTENSION', 'COMPOSITION', 'DEPENDENCY', 'LOLLIPOP', 'LINE', 'DOTTED_LINE'");
    }
    i++;
    const type2 = relationEnd();
    const title2 = types[i] === T.STR ? texts[i++] : 'none';
    const id2 = name(true);
    const relation: ClassRelation = {
      id1,
      id2,
      relation: { type1, type2, lineType: t === T.LINE ? 0 : 1 },
      relationTitle1: title1,
      relationTitle2: title2,
    };
    if (types[i] === T.LABEL) relation.title = db.cleanupLabel(texts[i++]);
    db.addRelation(relation);
  };

  const styles = (): string[] => {
    const out: string[] = [];
    while (true) {
      if (STYLE_PART[types[i]] === 0) fail("'ALPHA', 'NUM', 'COLON', 'UNIT', 'SPACE', 'BRKT', 'STYLE', 'PCT', 'LABEL'");
      let s = texts[i++];
      while (STYLE_PART[types[i]] === 1) s += texts[i++];
      out.push(s);
      if (types[i] !== T.COMMA) return out;
      i++;
    }
  };

  const tooltip = (id: string): void => {
    if (types[i] === T.STR) db.setTooltip(id, texts[i++]);
  };

  const linkStatement = (id: string): void => {
    const href = expect(T.STR);
    const tip = types[i] === T.STR ? texts[i++] : undefined;
    if (types[i] === T.LINK_TARGET) db.setLink(id, href, texts[i++]);
    else db.setLink(id, href);
    if (tip !== undefined) db.setTooltip(id, tip);
  };

  const statement = (): void => {
    const t = types[i];
    switch (t) {
      case T.UNICODE_TEXT:
      case T.ALPHA:
      case T.MINUS:
      case T.BQUOTE_STR:
        classOrRelation();
        return;
      case T.NAMESPACE:
        namespaceStatement();
        return;
      case T.CLASS:
        classStatement();
        return;
      case T.MEMBER:
        i++;
        return;
      case T.ANNOTATION_START: {
        i++;
        const annotation = alphaNum();
        expect(T.ANNOTATION_END);
        db.addAnnotation(name(true), annotation);
        return;
      }
      case T.CALLBACK: {
        i++;
        const id = name(true);
        db.setClickEvent(id, expect(T.STR));
        tooltip(id);
        return;
      }
      case T.LINK:
        i++;
        linkStatement(name(true));
        return;
      case T.CLICK: {
        i++;
        const id = name(true);
        if (types[i] === T.HREF) {
          i++;
          linkStatement(id);
          return;
        }
        const fn = expect(T.CALLBACK_NAME);
        if (types[i] === T.CALLBACK_ARGS) db.setClickEvent(id, fn, texts[i++]);
        else db.setClickEvent(id, fn);
        tooltip(id);
        return;
      }
      case T.STYLE: {
        i++;
        const id = expect(T.ALPHA);
        db.setCssStyle(id, styles());
        return;
      }
      case T.CSSCLASS: {
        i++;
        const ids = expect(T.STR);
        db.setCssClass(ids, expect(T.ALPHA));
        return;
      }
      case T.NOTE:
      case T.NOTE_FOR:
        noteStatement();
        return;
      case T.CLASSDEF: {
        i++;
        const ids = [expect(T.ALPHA)];
        while (types[i] === T.COMMA) {
          i++;
          ids.push(expect(T.ALPHA));
        }
        db.defineClass(ids, styles());
        return;
      }
      case T.direction_tb:
      case T.direction_bt:
      case T.direction_rl:
      case T.direction_lr:
        i++;
        db.setDirection(t === T.direction_tb ? 'TB' : t === T.direction_bt ? 'BT' : t === T.direction_rl ? 'RL' : 'LR');
        return;
      case T.acc_title:
        i++;
        db.setAccTitle(expect(T.acc_title_value).trim());
        return;
      case T.acc_descr:
        i++;
        db.setAccDescription(expect(T.acc_descr_value).trim());
        return;
      case T.acc_descr_multiline_value:
        db.setAccDescription(texts[i++].trim());
        return;
    }
    fail(
      "'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'NAMESPACE', 'ANNOTATION_START', 'CLASS', 'MEMBER', 'NOTE_FOR', 'NOTE', 'CLASSDEF', 'direction_tb', 'CALLBACK', 'LINK', 'CLICK', 'STYLE', 'CSSCLASS', 'ALPHA', 'MINUS', 'BQUOTE_STR'"
    );
  };

  // Mermaid's grammar also accepts statements with no `classDiagram` line before them.
  const header = types[i] === T.CLASS_DIAGRAM;
  if (header) {
    i++;
    expect(T.NEWLINE);
  }
  while (true) {
    statement();
    if (types[i] !== T.NEWLINE) break;
    i++;
    if (types[i] === T.EOF || types[i] === T.END) break;
  }
  if (header) expect(T.EOF);
  expect(T.END);
}
