import { syntaxError } from '../../errors.js';
import { tokenize } from './lexer.js';
import { T, TOKEN_NAMES } from './tokens.js';
import { LINETYPE, PLACEMENT, type SeqBoxData, type SeqItem, type SeqStatement, type SeqText } from './types.js';

// The calls Mermaid's grammar makes on its `yy` object.
export interface SeqBuilder {
  apply(doc: SeqItem): void;
  parseMessage(str: string): SeqText;
  parseBoxData(str: string): SeqBoxData;
  setDiagramTitle(text: string): void;
  setAccTitle(text: string): void;
  setAccDescription(text: string): void;
}

const SIGNAL = new Int8Array(T.STICK_ARROW_BOTTOM_REVERSE_DOTTED + 1).fill(-1);
SIGNAL[T.SOLID_OPEN_ARROW] = LINETYPE.SOLID_OPEN;
SIGNAL[T.DOTTED_OPEN_ARROW] = LINETYPE.DOTTED_OPEN;
SIGNAL[T.SOLID_ARROW] = LINETYPE.SOLID;
SIGNAL[T.DOTTED_ARROW] = LINETYPE.DOTTED;
SIGNAL[T.BIDIRECTIONAL_SOLID_ARROW] = LINETYPE.BIDIRECTIONAL_SOLID;
SIGNAL[T.BIDIRECTIONAL_DOTTED_ARROW] = LINETYPE.BIDIRECTIONAL_DOTTED;
SIGNAL[T.SOLID_CROSS] = LINETYPE.SOLID_CROSS;
SIGNAL[T.DOTTED_CROSS] = LINETYPE.DOTTED_CROSS;
SIGNAL[T.SOLID_POINT] = LINETYPE.SOLID_POINT;
SIGNAL[T.DOTTED_POINT] = LINETYPE.DOTTED_POINT;
SIGNAL[T.SOLID_ARROW_TOP] = LINETYPE.SOLID_TOP;
SIGNAL[T.SOLID_ARROW_BOTTOM] = LINETYPE.SOLID_BOTTOM;
SIGNAL[T.STICK_ARROW_TOP] = LINETYPE.STICK_TOP;
SIGNAL[T.STICK_ARROW_BOTTOM] = LINETYPE.STICK_BOTTOM;
SIGNAL[T.SOLID_ARROW_TOP_DOTTED] = LINETYPE.SOLID_TOP_DOTTED;
SIGNAL[T.SOLID_ARROW_BOTTOM_DOTTED] = LINETYPE.SOLID_BOTTOM_DOTTED;
SIGNAL[T.STICK_ARROW_TOP_DOTTED] = LINETYPE.STICK_TOP_DOTTED;
SIGNAL[T.STICK_ARROW_BOTTOM_DOTTED] = LINETYPE.STICK_BOTTOM_DOTTED;
SIGNAL[T.SOLID_ARROW_TOP_REVERSE] = LINETYPE.SOLID_ARROW_TOP_REVERSE;
SIGNAL[T.SOLID_ARROW_BOTTOM_REVERSE] = LINETYPE.SOLID_ARROW_BOTTOM_REVERSE;
SIGNAL[T.STICK_ARROW_TOP_REVERSE] = LINETYPE.STICK_ARROW_TOP_REVERSE;
SIGNAL[T.STICK_ARROW_BOTTOM_REVERSE] = LINETYPE.STICK_ARROW_BOTTOM_REVERSE;
SIGNAL[T.SOLID_ARROW_TOP_REVERSE_DOTTED] = LINETYPE.SOLID_ARROW_TOP_REVERSE_DOTTED;
SIGNAL[T.SOLID_ARROW_BOTTOM_REVERSE_DOTTED] = LINETYPE.SOLID_ARROW_BOTTOM_REVERSE_DOTTED;
SIGNAL[T.STICK_ARROW_TOP_REVERSE_DOTTED] = LINETYPE.STICK_ARROW_TOP_REVERSE_DOTTED;
SIGNAL[T.STICK_ARROW_BOTTOM_REVERSE_DOTTED] = LINETYPE.STICK_ARROW_BOTTOM_REVERSE_DOTTED;

const enum K {
  Top,
  Box,
  Loop,
  Rect,
  Opt,
  Break,
  Alt,
  Par,
  Critical,
}

interface Frame {
  kind: K;
  text: string;
  startType: number;
  // The current section's statements. Slot 0 of a block's first section is kept for its start entry.
  list: SeqItem[];
  // Finished sections, each followed by the divider text that ended it.
  sections: (SeqItem[] | string)[] | undefined;
}

const BLOCK = new Int8Array(T.STICK_ARROW_BOTTOM_REVERSE_DOTTED + 1).fill(-1);
BLOCK[T.loop] = K.Loop;
BLOCK[T.rect] = K.Rect;
BLOCK[T.opt] = K.Opt;
BLOCK[T.break] = K.Break;
BLOCK[T.alt] = K.Alt;
BLOCK[T.par] = K.Par;
BLOCK[T.par_over] = K.Par;
BLOCK[T.critical] = K.Critical;
BLOCK[T.box] = K.Box;

const SIGNAL_TOKENS =
  "'SOLID_OPEN_ARROW', 'DOTTED_OPEN_ARROW', 'SOLID_ARROW', 'DOTTED_ARROW', 'SOLID_CROSS', 'DOTTED_CROSS', 'SOLID_POINT', 'DOTTED_POINT'";

export function parseSequence(src: string, db: SeqBuilder): void {
  const { types, texts, starts } = tokenize(src);
  let i = 0;

  const fail = (expected: string): never => {
    const t = types[i];
    if (t === T.ERROR) throw syntaxError('sequence', src, starts[i], '', true);
    throw syntaxError('sequence', src, starts[i], `Expecting ${expected}, got '${TOKEN_NAMES[t]}'`);
  };
  const expect = (type: number): string => {
    if (types[i] !== type) fail(`'${TOKEN_NAMES[type]}'`);
    return texts[i++];
  };
  const actor = (): SeqStatement => {
    const t = types[i];
    if (t !== T.ACTOR && t !== T.link) fail("'ACTOR'");
    return { type: 'addParticipant', actor: texts[i++] };
  };
  const text2 = (): SeqText => db.parseMessage(expect(T.TXT).trim().substring(1));

  const participant = (): SeqStatement => {
    const t = types[i++];
    if (t === T.destroy) {
      const node = actor();
      expect(T.NEWLINE);
      node.type = 'destroyParticipant';
      return node;
    }
    const node = actor();
    if (types[i] === T.CONFIG_START && types[i - 1] === T.ACTOR) {
      i++;
      const content = expect(T.CONFIG_CONTENT);
      expect(T.CONFIG_END);
      node.config = content.trim();
    }
    node.draw = t === T.participant ? 'participant' : 'actor';
    if (types[i] === T.AS) {
      i++;
      const alias = expect(T.restOfLine);
      expect(T.NEWLINE);
      node.description = db.parseMessage(alias);
    } else if (types[i] === T.NEWLINE) {
      i++;
    } else {
      fail("'AS', 'NEWLINE'");
    }
    return node;
  };

  const signal = (from: SeqStatement): SeqItem => {
    let centralFrom = false;
    if (types[i] === T.CENTRAL) {
      centralFrom = true;
      i++;
    }
    const signalType = SIGNAL[types[i]];
    if (signalType < 0) fail(SIGNAL_TOKENS);
    i++;
    const mark = types[i];
    if (mark === T.CENTRAL || (!centralFrom && (mark === T.PLUS || mark === T.MINUS))) i++;
    const to = actor();
    const msg = text2();
    const message: SeqStatement = {
      type: 'addMessage',
      from: from.actor as string,
      to: to.actor as string,
      signalType,
      msg,
    };
    const out: SeqItem[] = [from, to, message];
    if (centralFrom) {
      const dual = mark === T.CENTRAL;
      message.activate = dual;
      message.centralConnection = dual ? LINETYPE.CENTRAL_CONNECTION_DUAL : LINETYPE.CENTRAL_CONNECTION_REVERSE;
      if (dual) out.push({ type: 'centralConnection', signalType: LINETYPE.CENTRAL_CONNECTION, actor: to.actor });
      out.push({
        type: 'centralConnectionReverse',
        signalType: LINETYPE.CENTRAL_CONNECTION_REVERSE,
        actor: from.actor,
      });
    } else if (mark === T.PLUS) {
      message.activate = true;
      out.push({ type: 'activeStart', signalType: LINETYPE.ACTIVE_START, actor: to.actor });
    } else if (mark === T.MINUS) {
      out.push({ type: 'activeEnd', signalType: LINETYPE.ACTIVE_END, actor: from.actor });
    } else if (mark === T.CENTRAL) {
      message.activate = true;
      message.centralConnection = LINETYPE.CENTRAL_CONNECTION;
      out.push({ type: 'centralConnection', signalType: LINETYPE.CENTRAL_CONNECTION, actor: to.actor });
    }
    return out;
  };

  const menu = (type: string): SeqItem => {
    i++;
    const who = actor();
    return [who, { type, actor: who.actor, text: text2() }];
  };

  const note = (): SeqItem => {
    i++;
    const t = types[i];
    if (t === T.over) {
      i++;
      const first = actor();
      let who: SeqItem = first;
      let ids = [first.actor as string, first.actor as string];
      if (types[i] === T.COMMA) {
        i++;
        const second = actor();
        who = [first, second];
        ids = [first.actor as string, second.actor as string];
      }
      return [who, { type: 'addNote', placement: PLACEMENT.OVER, actor: ids, text: text2() }];
    }
    if (t !== T.left_of && t !== T.right_of) fail("'over', 'left_of', 'right_of'");
    i++;
    const who = actor();
    const placement = t === T.left_of ? PLACEMENT.LEFTOF : PLACEMENT.RIGHTOF;
    return [who, { type: 'addNote', placement, actor: who.actor, text: text2() }];
  };

  const autonumber = (): SeqStatement => {
    i++;
    const node: SeqStatement = { type: 'sequenceIndex', sequenceVisible: true, signalType: LINETYPE.AUTONUMBER };
    if (types[i] === T.NUM) {
      node.sequenceIndex = Number(texts[i++]);
      node.sequenceIndexStep = types[i] === T.NUM ? Number(texts[i++]) : 1;
    } else if (types[i] === T.off) {
      i++;
      node.sequenceVisible = false;
    }
    if (types[i] !== T.NEWLINE) fail("'NEWLINE', 'NUM', 'off'");
    i++;
    return node;
  };

  // Folds the sections of an alt, par, or critical block from the last one back, as the grammar's
  // right recursion does, so the texts reach the model builder in the same order.
  const fold = (
    frame: Frame,
    divider: string,
    key: 'altText' | 'parText' | 'optionText',
    signalType: number
  ): SeqItem[] => {
    let list = frame.list;
    const sections = frame.sections;
    if (sections) {
      for (let k = sections.length - 2; k >= 0; k -= 2) {
        const head: SeqStatement = { type: divider, signalType };
        head[key] = db.parseMessage(sections[k + 1] as string);
        list = (sections[k] as SeqItem[]).concat([head, list]);
      }
    }
    return list;
  };

  const close = (frame: Frame): SeqItem[] => {
    const text = frame.text;
    let list = frame.list;
    switch (frame.kind) {
      case K.Box:
        list[0] = { type: 'boxStart', boxData: db.parseBoxData(text) };
        list.push({ type: 'boxEnd', boxText: text });
        break;
      case K.Loop:
        list[0] = { type: 'loopStart', loopText: db.parseMessage(text), signalType: LINETYPE.LOOP_START };
        list.push({ type: 'loopEnd', loopText: text, signalType: LINETYPE.LOOP_END });
        break;
      case K.Rect:
        list[0] = { type: 'rectStart', color: db.parseMessage(text), signalType: LINETYPE.RECT_START };
        list.push({ type: 'rectEnd', color: db.parseMessage(text), signalType: LINETYPE.RECT_END });
        break;
      case K.Opt:
        list[0] = { type: 'optStart', optText: db.parseMessage(text), signalType: LINETYPE.OPT_START };
        list.push({ type: 'optEnd', optText: db.parseMessage(text), signalType: LINETYPE.OPT_END });
        break;
      case K.Break:
        list[0] = { type: 'breakStart', breakText: db.parseMessage(text), signalType: LINETYPE.BREAK_START };
        list.push({ type: 'breakEnd', optText: db.parseMessage(text), signalType: LINETYPE.BREAK_END });
        break;
      case K.Alt:
        list = fold(frame, 'else', 'altText', LINETYPE.ALT_ELSE);
        list[0] = { type: 'altStart', altText: db.parseMessage(text), signalType: LINETYPE.ALT_START };
        list.push({ type: 'altEnd', signalType: LINETYPE.ALT_END });
        break;
      case K.Par:
        list = fold(frame, 'and', 'parText', LINETYPE.PAR_AND);
        list[0] = { type: 'parStart', parText: db.parseMessage(text), signalType: frame.startType };
        list.push({ type: 'parEnd', signalType: LINETYPE.PAR_END });
        break;
      default:
        list = fold(frame, 'option', 'optionText', LINETYPE.CRITICAL_OPTION);
        list[0] = { type: 'criticalStart', criticalText: db.parseMessage(text), signalType: LINETYPE.CRITICAL_START };
        list.push({ type: 'criticalEnd', signalType: LINETYPE.CRITICAL_END });
    }
    return list;
  };

  while (types[i] === T.NEWLINE) i++;
  expect(T.SD);

  const stack: Frame[] = [];
  let frame: Frame = { kind: K.Top, text: '', startType: 0, list: [], sections: undefined };

  while (true) {
    const t = types[i];
    const list = frame.list;
    if (frame.kind === K.Box) {
      if (t === T.participant || t === T.participant_actor || t === T.destroy) {
        list.push(participant());
      } else if (t === T.NEWLINE) {
        list.push([]);
        i++;
      } else if (t === T.end) {
        i++;
        const done = close(frame);
        frame = stack.pop()!;
        frame.list.push(done);
      } else {
        fail("'end', 'NEWLINE', 'participant', 'participant_actor', 'destroy'");
      }
      continue;
    }
    switch (t) {
      case T.NEWLINE:
      case T.INVALID:
        list.push([]);
        i++;
        break;
      case T.participant:
      case T.participant_actor:
      case T.destroy:
        list.push(participant());
        break;
      case T.create: {
        i++;
        const next = types[i];
        if (next !== T.participant && next !== T.participant_actor && next !== T.destroy) {
          fail("'participant', 'participant_actor', 'destroy'");
        }
        const node = participant();
        node.type = 'createParticipant';
        list.push(node);
        break;
      }
      case T.ACTOR:
        list.push(signal(actor()));
        expect(T.NEWLINE);
        break;
      case T.link: {
        const next = types[i + 1];
        list.push(next === T.ACTOR || next === T.link ? menu('addALink') : signal(actor()));
        expect(T.NEWLINE);
        break;
      }
      case T.links:
        list.push(menu('addLinks'));
        expect(T.NEWLINE);
        break;
      case T.properties:
        list.push(menu('addProperties'));
        expect(T.NEWLINE);
        break;
      case T.details:
        list.push(menu('addDetails'));
        expect(T.NEWLINE);
        break;
      case T.note:
        list.push(note());
        expect(T.NEWLINE);
        break;
      case T.autonumber:
        list.push(autonumber());
        break;
      case T.activate:
      case T.deactivate: {
        i++;
        const who = actor();
        expect(T.NEWLINE);
        list.push(
          t === T.activate
            ? { type: 'activeStart', signalType: LINETYPE.ACTIVE_START, actor: who.actor }
            : { type: 'activeEnd', signalType: LINETYPE.ACTIVE_END, actor: who.actor }
        );
        break;
      }
      case T.title:
      case T.legacy_title: {
        const title = texts[i++].substring(t === T.title ? 6 : 7);
        db.setDiagramTitle(title);
        list.push(title);
        break;
      }
      case T.acc_title: {
        i++;
        const value = expect(T.acc_title_value).trim();
        db.setAccTitle(value);
        list.push(value);
        break;
      }
      case T.acc_descr: {
        i++;
        const value = expect(T.acc_descr_value).trim();
        db.setAccDescription(value);
        list.push(value);
        break;
      }
      case T.acc_descr_multiline_value: {
        const value = texts[i++].trim();
        db.setAccDescription(value);
        list.push(value);
        break;
      }
      case T.loop:
      case T.rect:
      case T.opt:
      case T.break:
      case T.alt:
      case T.par:
      case T.par_over:
      case T.critical:
      case T.box: {
        i++;
        const text = expect(T.restOfLine);
        stack.push(frame);
        frame = {
          kind: BLOCK[t],
          text,
          startType: t === T.par_over ? LINETYPE.PAR_OVER_START : LINETYPE.PAR_START,
          list: [[]],
          sections: undefined,
        };
        break;
      }
      case T.else:
      case T.and:
      case T.option: {
        const kind = frame.kind;
        if (kind !== (t === T.else ? K.Alt : t === T.and ? K.Par : K.Critical)) fail("'end'");
        i++;
        const text = expect(T.restOfLine);
        (frame.sections ??= []).push(list, text);
        frame.list = [];
        break;
      }
      case T.end: {
        if (frame.kind === K.Top) fail("'NEWLINE', 'EOF'");
        i++;
        const done = close(frame);
        frame = stack.pop()!;
        frame.list.push(done);
        break;
      }
      case T.END:
        if (frame.kind !== K.Top) fail("'end'");
        db.apply(list);
        return;
      default:
        fail("'NEWLINE', 'ACTOR', 'participant', 'note', 'end'");
    }
  }
}
