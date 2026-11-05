import { locate, PeleError } from '../../errors.js';

// Mermaid's box-drawing preprocessor: turns `├──`, `└──`, `│` (and the heavy variants) into the
// indentation the grammar reads.

const ALL_BOX_CHARS = /[─━│┃└┗├┣]/;
const BRANCH_CHAR = /[└┗├┣]/;
const DASH_CHAR = /[─━]/;
const DECORATION_ONLY = /^[\s│┃]+$/;
const BOX_ONLY = /^[\s─━│┃└┗├┣]+$/;
const METADATA_LINE = /^\s*(title[\t ]|accTitle[\t ]*:|accDescr[\t ]*[:{])/;
const COMMENT_LINE = /^\s*%%/;
const INDENT_UNIT = '    ';

export interface PreprocessResult {
  text: string;
  // Output line number to original line number, both from 1. Empty when the text is unchanged.
  lineMap: Map<number, number>;
  // Per output line, maps rewritten indices to original indices.
  columns: Map<number, (at: number) => number>;
}

export function isBoxDrawingFormat(lines: string[]): boolean {
  return lines.some((line) => ALL_BOX_CHARS.test(line));
}

function inferSegmentWidth(contentLines: string[]): number {
  for (const line of contentLines) {
    const match = BRANCH_CHAR.exec(line);
    if (match?.index && match.index > 0) return match.index;
  }
  return 4;
}

export function remapErrorLines(message: string, lineMap: Map<number, number>): string {
  return message.replace(/\bline\s+(\d+)\b/gi, (match, lineStr: string) => {
    const original = lineMap.get(parseInt(lineStr, 10));
    return original ? `line ${original}` : match;
  });
}

export function lineStart(text: string, line: number): number {
  let at = 0;
  for (let n = 1; n < line; n++) at = text.indexOf('\n', at) + 1;
  return at;
}

function fail(input: string, index: number, message: string): never {
  const { line } = locate(input, lineStart(input, index + 1));
  throw new PeleError(`Line ${line}: ${message}`, 'syntax', { type: 'treeView', line });
}

// Undo four-space tab expansion.
function beforeTabs(line: string, at: number): number {
  let i = 0;
  for (let n = 0; i < line.length && n < at; i++) n += line[i] === '\t' ? 4 : 1;
  return i;
}

export function preprocessBoxDrawing(input: string): PreprocessResult {
  const lineMap = new Map<number, number>();
  const columns = new Map<number, (at: number) => number>();
  if (!ALL_BOX_CHARS.test(input)) return { text: input, lineMap, columns };
  const lines = input.split('\n');

  let keywordIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === 'treeView-beta') {
      keywordIdx = i;
      break;
    }
  }
  if (keywordIdx === -1) return { text: input, lineMap, columns };

  const passes = (line: string): boolean => line.trim() === '' || COMMENT_LINE.test(line) || METADATA_LINE.test(line);

  const contentLines: string[] = [];
  for (let i = keywordIdx + 1; i < lines.length; i++) {
    const line = lines[i];
    if (passes(line) || DECORATION_ONLY.test(line)) continue;
    contentLines.push(line.replace(/\t/g, '    '));
  }
  if (!isBoxDrawingFormat(contentLines)) return { text: input, lineMap, columns };

  const segmentWidth = inferSegmentWidth(contentLines);
  const out: string[] = [];
  const keep = (text: string, index: number, column?: (at: number) => number): void => {
    out.push(text);
    lineMap.set(out.length, index + 1);
    if (column) columns.set(out.length, column);
  };

  for (let i = 0; i <= keywordIdx; i++) keep(lines[i], i);
  for (let i = keywordIdx + 1; i < lines.length; i++) {
    const line = lines[i];
    if (passes(line)) {
      keep(line, i);
      continue;
    }
    if (DECORATION_ONLY.test(line)) continue;

    const normalized = line.replace(/\t/g, '    ');
    const branch = BRANCH_CHAR.exec(normalized);
    if (branch) {
      const depth = Math.round(branch.index / segmentWidth) + 1;
      let pos = branch.index + 1;
      while (pos < normalized.length && DASH_CHAR.test(normalized[pos])) pos++;
      while (pos < normalized.length && normalized[pos] === ' ') pos++;
      const content = normalized.slice(pos).trimEnd();
      if (!content) fail(input, i, 'Empty node — expected a filename or directory name after the box-drawing prefix');
      const indent = INDENT_UNIT.repeat(depth);
      keep(indent + content, i, (at) => beforeTabs(line, pos + Math.max(at - indent.length, 0)));
    } else if (BOX_ONLY.test(normalized)) {
      continue;
    } else if (ALL_BOX_CHARS.test(normalized)) {
      // A name that happens to contain a box character, such as "Section ─ A.txt", is a root item.
      keep(line, i);
    } else if (/^\s+/.test(normalized)) {
      fail(
        input,
        i,
        'Unexpected indentation without box-drawing characters. In box-drawing format, use ├── or └── prefixes for indented nodes.'
      );
    } else {
      keep(line, i);
    }
  }
  return { text: out.join('\n'), lineMap, columns };
}
