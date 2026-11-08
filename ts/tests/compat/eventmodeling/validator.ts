// The source rules of Mermaid's event modeling language: which kind of frame may feed which.
// Mermaid registers them with Langium's validator but never runs it when parsing a diagram,
// so a diagram that breaks them still renders, in Mermaid and here. They are kept here
// because Mermaid's specs test them.

const COMMAND = new Set(['cmd', 'command']);
const EVENT = new Set(['evt', 'event']);
const READ_MODEL = new Set(['rmo', 'readmodel']);
const PROCESSOR = new Set(['pcr', 'processor']);
const UI = new Set(['ui']);
const TRIGGER = new Set([...UI, ...PROCESSOR]);

export interface CheckedFrame {
  modelEntityType: string;
  sourceFrames: { ref?: { modelEntityType: string } }[];
}

export type Accept = (severity: 'error', message: string, info: { node: CheckedFrame; property: 'sourceFrames' }) => void;

export function checkSourceFrameTypes(frame: CheckedFrame, accept: Accept): void {
  if (frame.sourceFrames.length === 0) return;
  const type = frame.modelEntityType;
  const rule: [Set<string>, string, string] | undefined = COMMAND.has(type)
    ? [TRIGGER, 'command', 'ui or processor']
    : EVENT.has(type)
      ? [COMMAND, 'event', 'command']
      : READ_MODEL.has(type)
        ? [EVENT, 'read model', 'event']
        : PROCESSOR.has(type)
          ? [READ_MODEL, 'processor', 'read model']
          : UI.has(type)
            ? [READ_MODEL, 'ui', 'read model']
            : undefined;
  if (!rule) return;
  const [allowed, target, expected] = rule;
  for (const source of frame.sourceFrames) {
    if (source.ref !== undefined && !allowed.has(source.ref.modelEntityType)) {
      accept('error', `A ${target} can only receive input from a ${expected}, not from '${source.ref.modelEntityType}'.`, {
        node: frame,
        property: 'sourceFrames',
      });
    }
  }
}
