import { syntaxError } from '../../errors.js';
import type { TimelineDb } from './db.js';
import { T, TOKEN_NAMES, tokenize } from './lexer.js';

const STATEMENT =
  "'EOF', 'SPACE', 'NEWLINE', 'title', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'section', 'period', 'event'";

export function parseTimeline(src: string, db: TimelineDb): void {
  const { types, texts, starts } = tokenize(src);

  const fail = (i: number, expected: string): never => {
    const got = types[i] === T.END ? '1' : TOKEN_NAMES[types[i]];
    throw syntaxError('timeline', src, starts[i], `Expecting ${expected}, got '${got}'`);
  };

  const head = types[0];
  if (head !== T.timeline && head !== T.timeline_lr && head !== T.timeline_td) {
    fail(0, "'timeline', 'timeline_lr', 'timeline_td'");
  }
  if (head !== T.timeline) db.setDirection(head === T.timeline_lr ? 'LR' : 'TD');

  for (let i = 1; ; i++) {
    const text = texts[i];
    switch (types[i]) {
      case T.NEWLINE:
        break;
      case T.title:
        db.getCommonDb().setDiagramTitle(text.substring(6));
        break;
      case T.acc_title:
        db.getCommonDb().setAccTitle(texts[++i].trim());
        break;
      case T.acc_descr:
        db.getCommonDb().setAccDescription(texts[++i].trim());
        break;
      case T.acc_descr_multiline_value:
        db.getCommonDb().setAccDescription(text.trim());
        break;
      case T.section:
        db.addSection(text.substring(8));
        break;
      case T.period:
        db.addTask(text, 0, '');
        break;
      case T.event:
        db.addEvent(text.substring(2));
        break;
      case T.EOF:
        return;
      default:
        fail(i, STATEMENT);
    }
  }
}
