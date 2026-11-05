export type PeleErrorCode = 'unsupported-diagram' | 'syntax' | 'semantic' | 'limit';

export class PeleError extends Error {
  code: PeleErrorCode;
  type: string | null;
  mermaidType: string | null;
  line: number;
  column: number;
  snippet: string;

  constructor(
    message: string,
    code: PeleErrorCode = 'syntax',
    info: { type?: string | null; mermaidType?: string | null; line?: number; column?: number; snippet?: string } = {}
  ) {
    super(message);
    this.name = 'PeleError';
    this.code = code;
    this.type = info.type ?? null;
    this.mermaidType = info.mermaidType ?? null;
    this.line = info.line ?? 0;
    this.column = info.column ?? 0;
    this.snippet = info.snippet ?? '';
  }
}

// Input range [from, to) replaced by `length` output characters.
export type Edit = [from: number, to: number, length: number];

export function replacements(text: string, re: RegExp, length: number): Edit[] {
  return Array.from(text.matchAll(re), (match): Edit => [match.index, match.index + match[0].length, length]);
}

function stepInput(edits: Edit[], at: number): number {
  let shift = 0;
  for (const [from, to, length] of edits) {
    const start = from - shift;
    if (at < start) break;
    if (at < start + length) return from + Math.min(at - start, to - from - 1);
    shift += to - from - length;
  }
  return at + shift;
}

export function inputOffset(steps: Edit[][], at: number): number {
  for (let s = steps.length - 1; s >= 0; s--) at = stepInput(steps[s], at);
  return at;
}

interface Origin {
  parsed: string;
  find(offset: number): [source: string, offset: number];
}

let origin: Origin | undefined;

// Report positions in the original source while `run` parses.
export function locatingIn<T>(next: Origin, run: () => T): T {
  const outer = origin;
  origin = next;
  try {
    return run();
  } finally {
    origin = outer;
  }
}

// Maps only when `src` is exactly the text being parsed.
export function written(src: string, offset: number): [source: string, offset: number] {
  return origin !== undefined && src === origin.parsed ? origin.find(offset) : [src, offset];
}

export function locate(src: string, offset: number): { line: number; column: number; snippet: string } {
  // Keep EOF errors on the last content line.
  const end = src.trimEnd().length;
  if (end > 0 && offset >= end) {
    [src, offset] = written(src, end - 1);
    offset++;
  } else {
    [src, offset] = written(src, offset);
  }
  let line = 1;
  let lineStart = 0;
  for (let i = src.indexOf('\n'); i !== -1 && i < offset; i = src.indexOf('\n', i + 1)) {
    line++;
    lineStart = i + 1;
  }
  let lineEnd = src.indexOf('\n', offset);
  if (lineEnd === -1) lineEnd = src.length;
  const column = offset - lineStart + 1;
  const text = src.slice(lineStart, lineEnd);
  return { line, column, snippet: text + '\n' + '-'.repeat(column - 1) + '^' };
}

export function syntaxError(type: string, src: string, offset: number, detail: string, lexical = false): PeleError {
  const loc = locate(src, offset);
  const head = lexical ? `Lexical error on line ${loc.line}. Unrecognized text.` : `Parse error on line ${loc.line}:`;
  const message = lexical ? `${head}\n${loc.snippet}` : `${head}\n${loc.snippet}\n${detail}`;
  return new PeleError(message, 'syntax', { type, ...loc });
}
