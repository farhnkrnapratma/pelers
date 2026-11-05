import { syntaxError } from '../../errors.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';

// The calls the grammar's actions make, under Mermaid's names.
export interface GanttBuilder {
  setDateFormat(txt: string): void;
  enableInclusiveEndDates(): void;
  TopAxis(): void;
  setAxisFormat(txt: string): void;
  setTickInterval(txt: string): void;
  setExcludes(txt: string): void;
  setIncludes(txt: string): void;
  setTodayMarker(txt: string): void;
  setWeekday(txt: string): void;
  setWeekend(txt: string): void;
  setDiagramTitle(txt: string): void;
  setAccTitle(txt: string): void;
  setAccDescription(txt: string): void;
  addSection(txt: string): void;
  addTask(descr: string, data: string): void;
  setClickEvent(ids: string, functionName: string, functionArgs: string | null): void;
  setLink(ids: string, link: string): void;
}

const WEEKDAYS = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'];

const LINE =
  "'EOF', 'SPACE', 'NL', 'weekday_monday', 'weekday_tuesday', 'weekday_wednesday', 'weekday_thursday', " +
  "'weekday_friday', 'weekday_saturday', 'weekday_sunday', 'weekend_friday', 'weekend_saturday', 'dateFormat', " +
  "'inclusiveEndDates', 'topAxis', 'axisFormat', 'tickInterval', 'excludes', 'includes', 'todayMarker', 'title', " +
  "'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'section', 'taskTxt', 'click'";

// Tokens that can start a line, which are also the only ones that can follow a complete statement.
const STARTS_LINE = new Uint8Array(T.EOF + 1);
for (let t = T.weekday_monday; t <= T.weekend_saturday; t++) STARTS_LINE[t] = 1;
for (const t of [
  T.EOF,
  T.NL,
  T.dateFormat,
  T.inclusiveEndDates,
  T.topAxis,
  T.axisFormat,
  T.tickInterval,
  T.excludes,
  T.includes,
  T.todayMarker,
  T.title,
  T.acc_title,
  T.acc_descr,
  T.acc_descr_multiline_value,
  T.section,
  T.taskTxt,
  T.click,
]) {
  STARTS_LINE[t] = 1;
}

export function parseGantt(src: string, db: GanttBuilder): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    throw syntaxError('gantt', src, starts[i], `Expecting ${expected}, got '${t === T.END ? '1' : TOKEN_NAMES[t]}'`);
  };

  // Mermaid's parser runs a statement's action only after seeing that the next token can follow it.
  const end = (also = ''): void => {
    if (STARTS_LINE[types[i]] !== 1) fail(LINE + also);
  };

  if (types[i] !== T.gantt) fail("'gantt'");
  i++;

  while (true) {
    const t = types[i];
    const text = texts[i];
    if (STARTS_LINE[t] !== 1) fail(LINE);
    i++;
    switch (t) {
      case T.EOF:
        if (types[i] === T.END) return;
        end();
        break;
      case T.NL:
        break;
      case T.dateFormat:
        end();
        db.setDateFormat(text.slice(11));
        break;
      case T.inclusiveEndDates:
        end();
        db.enableInclusiveEndDates();
        break;
      case T.topAxis:
        end();
        db.TopAxis();
        break;
      case T.axisFormat:
        end();
        db.setAxisFormat(text.slice(11));
        break;
      case T.tickInterval:
        end();
        db.setTickInterval(text.slice(13));
        break;
      case T.excludes:
        end();
        db.setExcludes(text.slice(9));
        break;
      case T.includes:
        end();
        db.setIncludes(text.slice(9));
        break;
      case T.todayMarker:
        end();
        db.setTodayMarker(text.slice(12));
        break;
      case T.weekend_friday:
        end();
        db.setWeekend('friday');
        break;
      case T.weekend_saturday:
        end();
        db.setWeekend('saturday');
        break;
      case T.title:
        end();
        db.setDiagramTitle(text.slice(6));
        break;
      case T.acc_title: {
        if (types[i] !== T.acc_title_value) fail("'acc_title_value'");
        const value = texts[i++];
        end();
        db.setAccTitle(value.trim());
        break;
      }
      case T.acc_descr: {
        if (types[i] !== T.acc_descr_value) fail("'acc_descr_value'");
        const value = texts[i++];
        end();
        db.setAccDescription(value.trim());
        break;
      }
      case T.acc_descr_multiline_value:
        end();
        db.setAccDescription(text.trim());
        break;
      case T.section:
        end();
        db.addSection(text.slice(8));
        break;
      case T.taskTxt: {
        if (types[i] !== T.taskData) fail("'taskData'");
        const data = texts[i++];
        end();
        db.addTask(text, data);
        break;
      }
      case T.click: {
        let name: string | undefined;
        let args: string | null = null;
        let href: string | undefined;
        if (types[i] === T.callbackname) {
          name = texts[i++];
          if (types[i] === T.callbackargs) args = texts[i++];
          if (types[i] === T.href) {
            href = texts[i++];
            end();
          } else {
            end(args === null ? ", 'callbackargs', 'href'" : ", 'href'");
          }
        } else if (types[i] === T.href) {
          href = texts[i++];
          if (types[i] === T.callbackname) {
            name = texts[i++];
            if (types[i] === T.callbackargs) {
              args = texts[i++];
              end();
            } else {
              end(", 'callbackargs'");
            }
          } else {
            end(", 'callbackname'");
          }
        } else {
          fail("'callbackname', 'href'");
        }
        if (name !== undefined) db.setClickEvent(text, name, args);
        if (href !== undefined) db.setLink(text, href);
        break;
      }
      default:
        end();
        db.setWeekday(WEEKDAYS[t - T.weekday_monday]);
    }
  }
}
