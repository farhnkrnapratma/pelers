import { accDescr } from '../common/accDescr.js';
import {
  ACC_TITLE,
  DIRECTIVE,
  Reader,
  SINGLE_LINE_COMMENT,
  YAML,
  keyword,
  titleBefore,
  tokenize,
  type TokenType,
} from '../common/tokens.js';

const EM_ID: TokenType = { name: 'EM_ID', pattern: /[_a-zA-Z][\w_]*/y };

// Langium gives every keyword that is also an identifier the identifier as a longer alternative.
const word = (text: string): TokenType => ({ name: text, pattern: text, longer: [EM_ID] });
const sign = (text: string): TokenType => ({ name: text, pattern: text });

const WORDS = [
  'resetframe',
  'readmodel',
  'processor',
  'timeframe',
  'command',
  'entity',
  'event',
  'jsobj',
  'figma',
  'given',
  'json',
  'salt',
  'html',
  'text',
  'data',
  'note',
  'when',
  'then',
  'rmo',
  'cmd',
  'evt',
  'pcr',
  'uri',
];

export const EVENTMODELING_TOKENS: readonly TokenType[] = [
  { ...keyword('eventmodeling'), longer: [EM_ID] },
  ...WORDS.map(word),
  sign('->>'),
  word('gwt'),
  word('ui'),
  word('md'),
  word('tf'),
  sign('[['),
  sign(']]'),
  word('rf'),
  sign('`'),
  sign('.'),
  EM_ID,
  { name: 'EM_FID', pattern: /\d{1,3}/y },
  { name: 'EM_DATA_INLINE', pattern: /\{(.*)\}|"(.*)"|'(.*)'/y },
  { name: 'EM_DATA_BLOCK', pattern: /\{[\t ]*\r?\n(?:[\S\s]*?\r?\n)?\}(?:\r?\n|(?!\S))/y },
  accDescr('EM_ACC_DESCR'),
  { ...ACC_TITLE, name: 'EM_ACC_TITLE' },
  { ...titleBefore(String.raw`\w`), name: 'EM_TITLE' },
  { name: 'EM_WS', pattern: /\s+/y, hidden: true },
  { ...YAML, name: 'EM_YAML' },
  { ...DIRECTIVE, name: 'EM_DIRECTIVE' },
  { ...SINGLE_LINE_COMMENT, name: 'EM_SINGLE_LINE_COMMENT' },
  { name: 'EM_ML_COMMENT', pattern: /\/\*[\s\S]*?\*\//y, hidden: true },
  { name: 'EM_SL_COMMENT', pattern: /\/\/[^\n\r]*/y, hidden: true },
];

const index = (name: string): number => EVENTMODELING_TOKENS.findIndex((t) => t.name === name);
const kinds = (names: string[]): Set<number> => new Set(names.map(index));

const T = {
  keyword: 0,
  arrow: index('->>'),
  open: index('[['),
  close: index(']]'),
  tick: index('`'),
  dot: index('.'),
  id: index('EM_ID'),
  frameId: index('EM_FID'),
  inline: index('EM_DATA_INLINE'),
  block: index('EM_DATA_BLOCK'),
  accDescr: index('EM_ACC_DESCR'),
  accTitle: index('EM_ACC_TITLE'),
  title: index('EM_TITLE'),
  entity: index('entity'),
  data: index('data'),
  note: index('note'),
  gwt: index('gwt'),
  given: index('given'),
  when: index('when'),
  then: index('then'),
};
const TIME_FRAME = kinds(['tf', 'timeframe']);
const RESET_FRAME = kinds(['rf', 'resetframe']);
const ENTITY_TYPE = kinds(['rmo', 'readmodel', 'ui', 'cmd', 'command', 'evt', 'event', 'pcr', 'processor']);
const DATA_TYPE = kinds(['json', 'jsobj', 'figma', 'salt', 'uri', 'md', 'html', 'text']);

// A cross-reference as Mermaid reads it: by its text, never linked.
export interface Ref {
  $refText: string;
}

export interface EmFrame {
  $type: 'EmTimeFrame' | 'EmResetFrame';
  name: string;
  modelEntityType: string;
  entityIdentifier: string;
  sourceFrames: Ref[];
  dataReference?: Ref;
  dataType?: string;
  dataInlineValue?: string;
  // Offset of the frame in the source, for error messages.
  $offset: number;
}

export interface EmDataEntity {
  $type: 'EmDataEntity';
  name: string;
  dataType?: string;
  dataBlockValue: string;
}

export interface EmNoteEntity {
  $type: 'EmNoteEntity';
  sourceFrame: Ref;
  dataType?: string;
  dataBlockValue: string;
}

export interface EmGwtStatement {
  $type: 'EmGwtStatement';
  entityIdentifier: Ref;
}

export interface EmGwt {
  $type: 'EmGwt';
  sourceFrame: Ref;
  givenStatements: EmGwtStatement[];
  whenStatements: EmGwtStatement[];
  thenStatements: EmGwtStatement[];
}

export interface EventModelAst {
  $type: 'EventModel';
  // The whole token, keyword included: the grammar's own terminals get no value conversion.
  title?: string;
  accTitle?: string;
  accDescr?: string;
  modelEntities: { $type: 'EmModelEntity'; name: string }[];
  frames: EmFrame[];
  dataEntities: EmDataEntity[];
  noteEntities: EmNoteEntity[];
  gwtEntities: EmGwt[];
  $source: string;
}

export function parseEventModel(src: string): EventModelAst {
  const r = new Reader(tokenize(src, EVENTMODELING_TOKENS, 'eventmodeling'), EVENTMODELING_TOKENS, 'eventmodeling');
  const ast: EventModelAst = {
    $type: 'EventModel',
    modelEntities: [],
    frames: [],
    dataEntities: [],
    noteEntities: [],
    gwtEntities: [],
    $source: src,
  };
  const at = (kind: number): boolean => r.kinds[r.i] === kind;
  const one = (set: Set<number>, expected: string): string => (set.has(r.kinds[r.i]) ? r.take() : r.fail(expected));
  const qualifiedName = (): string => {
    let name = r.expect(T.id);
    while (r.accept(T.dot)) name += '.' + r.expect(T.id);
    return name;
  };
  const dataType = (): string | undefined => {
    if (!r.accept(T.tick)) return undefined;
    const type = one(DATA_TYPE, 'a data type');
    r.expect(T.tick);
    return type;
  };
  const statements = (): EmGwtStatement[] => {
    const out: EmGwtStatement[] = [];
    do {
      one(ENTITY_TYPE, 'an entity type');
      out.push({ $type: 'EmGwtStatement', entityIdentifier: { $refText: r.expect(T.id) } });
    } while (ENTITY_TYPE.has(r.kinds[r.i]));
    return out;
  };

  r.expect(T.keyword);
  while (!at(-1)) {
    const kind = r.kinds[r.i];
    if (kind === T.accDescr) {
      ast.accDescr = r.take();
    } else if (kind === T.accTitle) {
      ast.accTitle = r.take();
    } else if (kind === T.title) {
      ast.title = r.take();
    } else if (kind === T.entity) {
      r.i++;
      ast.modelEntities.push({ $type: 'EmModelEntity', name: qualifiedName() });
    } else if (TIME_FRAME.has(kind) || RESET_FRAME.has(kind)) {
      const frame: EmFrame = {
        $type: TIME_FRAME.has(kind) ? 'EmTimeFrame' : 'EmResetFrame',
        name: '',
        modelEntityType: '',
        entityIdentifier: '',
        sourceFrames: [],
        $offset: r.tokens.starts[r.i],
      };
      r.i++;
      frame.name = r.expect(T.frameId);
      frame.modelEntityType = one(ENTITY_TYPE, 'an entity type');
      frame.entityIdentifier = qualifiedName();
      while (r.accept(T.arrow)) frame.sourceFrames.push({ $refText: r.expect(T.frameId) });
      if (r.accept(T.open)) {
        frame.dataReference = { $refText: r.expect(T.id) };
        r.expect(T.close);
      }
      if (at(T.tick) || at(T.inline)) {
        const type = dataType();
        if (type !== undefined) frame.dataType = type;
        frame.dataInlineValue = r.expect(T.inline);
      }
      ast.frames.push(frame);
    } else if (kind === T.data) {
      r.i++;
      const entity: EmDataEntity = { $type: 'EmDataEntity', name: r.expect(T.id), dataBlockValue: '' };
      const type = dataType();
      if (type !== undefined) entity.dataType = type;
      entity.dataBlockValue = r.expect(T.block);
      ast.dataEntities.push(entity);
    } else if (kind === T.note) {
      r.i++;
      const note: EmNoteEntity = {
        $type: 'EmNoteEntity',
        sourceFrame: { $refText: r.expect(T.frameId) },
        dataBlockValue: '',
      };
      const type = dataType();
      if (type !== undefined) note.dataType = type;
      note.dataBlockValue = r.expect(T.block);
      ast.noteEntities.push(note);
    } else if (kind === T.gwt) {
      r.i++;
      const gwt: EmGwt = {
        $type: 'EmGwt',
        sourceFrame: { $refText: r.expect(T.frameId) },
        givenStatements: [],
        whenStatements: [],
        thenStatements: [],
      };
      r.expect(T.given);
      gwt.givenStatements = statements();
      if (r.accept(T.when)) gwt.whenStatements = statements();
      r.expect(T.then);
      gwt.thenStatements = statements();
      ast.gwtEntities.push(gwt);
    } else {
      r.fail('a frame, an entity, a data block, a note, or a specification');
    }
  }
  return ast;
}
