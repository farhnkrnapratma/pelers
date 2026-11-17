import { PelersError, locate } from '../../errors.js';
import { accDescrValue, accTitleValue, titleValue } from '../common/tokens.js';
import type { EmFrame, EventModelAst } from './parser.js';

export type EntityKind = 'ui' | 'processor' | 'command' | 'readmodel' | 'event';
export type LaneGroup = 'trigger' | 'state' | 'stream';

export interface EmLane {
  // Mermaid's ordering key: 0 for UI/Automation, 100 for Command/Read Model, 200 for Events,
  // and the next free number after those for each namespace.
  index: number;
  label: string;
  group: LaneGroup;
  namespace?: string;
}

export interface EmBox {
  // The frame number as written, such as "01".
  id: string;
  kind: EntityKind;
  entity: string;
  name: string;
  namespace?: string;
  reset: boolean;
  lane: EmLane;
  // Example data from the frame or from the data block it refers to.
  data?: string;
  dataType?: string;
}

export interface EmRelation {
  source: EmBox;
  target: EmBox;
}

export interface EmNote {
  frame: string;
  text: string;
  dataType?: string;
}

export interface EmSpecification {
  frame: string;
  given: string[];
  when: string[];
  then: string[];
}

export interface EventModelingModel {
  type: 'eventmodeling';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  boxes: EmBox[];
  // Sorted from top to bottom.
  lanes: EmLane[];
  relations: EmRelation[];
  notes: EmNote[];
  specifications: EmSpecification[];
  entities: string[];
}

const KINDS = new Map<string, EntityKind>([
  ['ui', 'ui'],
  ['pcr', 'processor'],
  ['processor', 'processor'],
  ['rmo', 'readmodel'],
  ['readmodel', 'readmodel'],
  ['cmd', 'command'],
  ['command', 'command'],
  ['evt', 'event'],
  ['event', 'event'],
]);

// [first index, label, label prefix for a namespace]
const GROUPS = new Map<LaneGroup, readonly [number, string, string]>([
  ['trigger', [0, 'UI/Automation', 'UI/A: ']],
  ['state', [100, 'Command/Read Model', 'C/RM: ']],
  ['stream', [200, 'Events', 'Stream: ']],
]);

function groupOf(kind: EntityKind): LaneGroup {
  return kind === 'ui' || kind === 'processor' ? 'trigger' : kind === 'event' ? 'stream' : 'state';
}

// The text between the outer braces, without the indentation its lines share.
function blockText(raw: string): string {
  const open = raw.indexOf('{');
  const close = raw.lastIndexOf('}');
  const inner =
    open !== -1 && close > open ? raw.slice(open + 1, close) : open === -1 && raw.length >= 2 ? raw.slice(1, -1) : '';
  const lines = inner.replace(/\r/g, '').split('\n');
  while (lines.length > 0 && lines[0].trim() === '') lines.shift();
  while (lines.length > 0 && lines[lines.length - 1].trim() === '') lines.pop();
  let indent = Infinity;
  for (const line of lines) {
    if (line.trim() === '') continue;
    indent = Math.min(indent, line.length - line.trimStart().length);
  }
  return lines.map((line) => line.slice(Math.min(indent, line.length)).trimEnd()).join('\n');
}

export function buildEventModel(ast: EventModelAst, title: string | undefined): EventModelingModel {
  const ids = new Set<string>();
  for (const frame of ast.frames) {
    if (ids.has(frame.name)) {
      const line = locate(ast.$source, frame.$offset).line;
      throw new PelersError(`Duplicate event modeling frame ID "${frame.name}" on line ${line}`, 'semantic', {
        type: 'eventmodeling',
        line,
      });
    }
    ids.add(frame.name);
  }

  const data = new Map<string, string>();
  for (const entity of ast.dataEntities) if (!data.has(entity.name)) data.set(entity.name, entity.dataBlockValue);

  // One lane per group, and one more per namespace within a group.
  const laneOf = new Map<string, EmLane>();
  const next = new Map<LaneGroup, number>();
  const lane = (kind: EntityKind, namespace: string | undefined): EmLane => {
    const group = groupOf(kind);
    const key = namespace === undefined ? group : `${group} ${namespace}`;
    let found = laneOf.get(key);
    if (!found) {
      const [first, label, prefix] = GROUPS.get(group)!;
      let index = first;
      if (namespace !== undefined) {
        index = next.get(group) ?? first + 1;
        next.set(group, index + 1);
      }
      found = { index, label: namespace === undefined ? label : prefix + namespace, group, namespace };
      laneOf.set(key, found);
    }
    return found;
  };

  const boxes: EmBox[] = [];
  const byId = new Map<string, EmBox>();
  const relations: EmRelation[] = [];
  ast.frames.forEach((frame: EmFrame, index) => {
    // Only a two-part name has a namespace.
    const parts = frame.entityIdentifier.split('.');
    const namespace = parts.length === 2 ? parts[0] : undefined;
    const kind = KINDS.get(frame.modelEntityType) ?? 'event';
    let text = frame.dataInlineValue === undefined ? undefined : blockText(frame.dataInlineValue);
    if (frame.dataReference) {
      const block = data.get(frame.dataReference.$refText);
      if (block !== undefined) text = blockText(block);
    }
    const box: EmBox = {
      id: frame.name,
      kind,
      entity: frame.entityIdentifier,
      name: parts.length === 2 ? parts[1] : frame.entityIdentifier,
      namespace,
      reset: frame.$type === 'EmResetFrame',
      lane: lane(kind, namespace),
      data: text,
      dataType: frame.dataType,
    };
    boxes.push(box);
    byId.set(box.id, box);

    if (frame.sourceFrames.length > 0) {
      // Sources connect in frame order, and only frames that already have a box.
      const wanted = new Set(frame.sourceFrames.map((ref) => ref.$refText));
      for (const source of boxes) if (wanted.has(source.id)) relations.push({ source, target: box });
    } else if (index > 0 && !box.reset) {
      // Otherwise the nearest earlier frame in another lane feeds this one.
      for (let i = index - 1; i >= 0; i--) {
        if (boxes[i].lane !== box.lane) {
          relations.push({ source: boxes[i], target: box });
          break;
        }
      }
    }
  });

  const names = (statements: { entityIdentifier: { $refText: string } }[]): string[] =>
    statements.map((s) => s.entityIdentifier.$refText);
  return {
    type: 'eventmodeling',
    title: (ast.title && titleValue(ast.title)) || title,
    accTitle: ast.accTitle ? accTitleValue(ast.accTitle).replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? accDescrValue(ast.accDescr).replace(/\n\s+/g, '\n') : undefined,
    boxes,
    lanes: [...laneOf.values()].sort((a, b) => a.index - b.index),
    relations,
    notes: ast.noteEntities.map((note) => ({
      frame: note.sourceFrame.$refText,
      text: blockText(note.dataBlockValue),
      dataType: note.dataType,
    })),
    specifications: ast.gwtEntities.map((gwt) => ({
      frame: gwt.sourceFrame.$refText,
      given: names(gwt.givenStatements),
      when: names(gwt.whenStatements),
      then: names(gwt.thenStatements),
    })),
    entities: ast.modelEntities.map((entity) => entity.name),
  };
}
