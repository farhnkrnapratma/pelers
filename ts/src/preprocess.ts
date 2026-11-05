import { PeleError, replacements, type Edit } from './errors.js';
import { parseYaml, type YamlValue } from './util/yaml.js';
import { isSpace as isWhitespace } from './util/chars.js';

export type Config = { [key: string]: YamlValue };

export interface Preprocessed {
  text: string;
  title: string | undefined;
  config: Config;
  // The text before its comment lines are removed, and the number of front matter lines before it.
  withComments: string;
  lineOffset: number;
}

const RE_COMMENT = /^\s*%%(?!{)[^\n]+\n?/gm;
const RE_DIRECTIVE = /%{2}{\s*(?:(\w+)\s*:|(\w+))\s*(?:(\w+)|((?:(?!}%{2}).|\r?\n)*))?\s*(?:}%{2})?/gi;
const RE_HTML_TAG = /<(\w+)([^>]*)>/g;
const RE_ATTR = /="([^"]*)"/g;
const UNSAFE_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

function removeAll(text: string, re: RegExp, steps: Edit[][] | undefined): string {
  steps?.push(replacements(text, re, 0));
  return text.replace(re, '');
}

export function cleanupComments(text: string, steps?: Edit[][]): string {
  if (text.includes('%%')) text = removeAll(text, RE_COMMENT, steps);
  const trimmed = text.trimStart();
  steps?.push([[0, text.length - trimmed.length, 0]]);
  return trimmed;
}

export function normalizeLineBreaks(text: string): string {
  return text.includes('\r') ? text.replace(/\r\n?/g, '\n') : text;
}

function isLineBreak(ch: string | undefined): boolean {
  return ch === '\n' || ch === '\r';
}

function isSpace(ch: string | undefined): boolean {
  return ch !== undefined && /\s/.test(ch);
}

interface FrontMatter {
  indent: string;
  body: string;
  length: number;
}

// Finds a `---` fenced YAML block at the start of the text the way Mermaid's front matter regex does.
function matchFrontMatter(text: string): FrontMatter | undefined {
  let cursor = 0;
  while (cursor < text.length && isSpace(text[cursor]) && !isLineBreak(text[cursor])) cursor++;
  const indent = text.slice(0, cursor);
  if (!text.startsWith('---', cursor)) return undefined;

  const openingEnds: number[] = [];
  for (let i = cursor + 3; i < text.length && isSpace(text[i]); i++) {
    if (isLineBreak(text[i])) openingEnds.push(i);
  }
  if (openingEnds.length === 0) return undefined;

  const fence = indent + '---';
  const closingEnd = (i: number): number => {
    if (!isLineBreak(text[i]) || !text.startsWith(fence, i + 1)) return -1;
    let last = -1;
    for (let j = i + 1 + fence.length; j < text.length && isSpace(text[j]); j++) {
      if (isLineBreak(text[j])) last = j;
    }
    return last;
  };

  const needle = '\n' + fence;
  let lastFence = -1;
  for (let i = text.lastIndexOf(needle); i > openingEnds[0]; i = text.lastIndexOf(needle, i - 1)) {
    if (closingEnd(i) !== -1) {
      lastFence = i;
      break;
    }
  }
  if (lastFence === -1) return undefined;

  let openingEnd = openingEnds[0];
  for (const candidate of openingEnds) if (candidate < lastFence) openingEnd = candidate;

  const bodyStart = openingEnd + 1;
  const firstAt = bodyStart > 0 ? bodyStart - 1 : 0;
  for (let i = text.indexOf(needle, firstAt); i !== -1 && i <= lastFence; i = text.indexOf(needle, i + 1)) {
    if (i < bodyStart) continue;
    const end = closingEnd(i);
    if (end !== -1) return { indent, body: text.slice(bodyStart, i), length: end + 1 };
  }
  return undefined;
}

export function isRecord(value: unknown): value is Config {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function merge(target: Config, source: Config): Config {
  for (const key of Object.keys(source)) {
    if (UNSAFE_KEYS.has(key)) continue;
    const value = source[key];
    const current = target[key];
    if (isRecord(value) && isRecord(current)) merge(current, value);
    else if (isRecord(value)) target[key] = merge({}, value);
    else target[key] = value;
  }
  return target;
}

function directives(text: string, config: Config, steps: Edit[][] | undefined): string {
  if (!text.includes('%%{')) return text;
  const normalized = text.trim().replace(/'/g, '"');
  RE_DIRECTIVE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = RE_DIRECTIVE.exec(normalized)) !== null) {
    if (match.index === RE_DIRECTIVE.lastIndex) RE_DIRECTIVE.lastIndex++;
    const type = match[1] ?? match[2];
    if (type === 'wrap') {
      config.wrap = true;
    } else if ((type === 'init' || type === 'initialize') && match[4]) {
      try {
        const args: unknown = JSON.parse(match[4].trim());
        if (isRecord(args)) merge(config, args);
      } catch {
        // Mermaid ignores an init directive whose body is not valid JSON.
      }
    }
  }
  RE_DIRECTIVE.lastIndex = 0;
  return removeAll(text, RE_DIRECTIVE, steps);
}

// `steps` maps normalized source to `withComments`.
export function preprocess(source: string, steps?: Edit[][]): Preprocessed {
  let text = normalizeLineBreaks(source);
  // A tag needs a closing bracket, so nothing after the last one can match.
  const lastClose = text.includes('<') ? text.lastIndexOf('>') : -1;
  if (lastClose > 0) {
    text =
      text
        .slice(0, lastClose + 1)
        .replace(RE_HTML_TAG, (_m, tag: string, attrs: string) => '<' + tag + attrs.replace(RE_ATTR, "='$1'") + '>') +
      text.slice(lastClose + 1);
  }

  const config: Config = {};
  let title: string | undefined;
  let lineOffset = 0;

  const front = text.includes('---') ? matchFrontMatter(text) : undefined;
  if (front) {
    const body = front.indent
      ? front.body
          .split('\n')
          .map((line) => (line.startsWith(front.indent) ? line.slice(front.indent.length) : line))
          .join('\n')
      : front.body;
    let parsed: YamlValue;
    try {
      parsed = parseYaml(body);
    } catch (error) {
      throw new PeleError(`Front matter is not valid YAML. ${(error as Error).message}`, 'syntax');
    }
    if (isRecord(parsed)) {
      if (parsed.title) title = String(parsed.title);
      if (isRecord(parsed.config)) merge(config, parsed.config);
      if (parsed.displayMode) {
        const gantt = isRecord(config.gantt) ? config.gantt : (config.gantt = {});
        gantt.displayMode = String(parsed.displayMode);
      }
    }
    for (let i = text.indexOf('\n'); i !== -1 && i < front.length; i = text.indexOf('\n', i + 1)) lineOffset++;
    text = text.slice(front.length);
    steps?.push([[0, front.length, 0]]);
  }

  text = directives(text, config, steps);
  return { text: cleanupComments(text), title, config, withComments: text, lineOffset };
}

const RE_ENTITY = /#\w+;/g;
const RE_INT = /^\+?\d+$/;
const RE_LINE_BREAK = new RegExp('[\\n\\r' + String.fromCharCode(0x2028, 0x2029) + ']', 'g');

function recording(steps: Edit[][] | undefined): Edit[] | undefined {
  if (steps === undefined) return undefined;
  const edits: Edit[] = [];
  steps.push(edits);
  return edits;
}

// Does what Mermaid's `text.replace(/keyword.*:\S*#.*;/g, drop the last character)` does:
// on a line where the keyword is followed by a colon, a color-like `#`, and later a semicolon,
// the last semicolon goes. Mermaid's regex backtracks without bound; this is one pass.
function dropColorSemicolons(text: string, keyword: string, steps: Edit[][] | undefined): string {
  const edits = recording(steps);
  let from = text.indexOf(keyword);
  if (from === -1) return text;
  let out = '';
  let last = 0;
  while (from !== -1) {
    RE_LINE_BREAK.lastIndex = from;
    const lineEnd = RE_LINE_BREAK.test(text) ? RE_LINE_BREAK.lastIndex - 1 : text.length;
    let afterColon = false;
    let hash = -1;
    for (let j = from + keyword.length; j < lineEnd; j++) {
      const c = text.charCodeAt(j);
      if (c === 58) {
        afterColon = true;
      } else if (c === 35) {
        if (afterColon) {
          hash = j;
          break;
        }
      } else if (isWhitespace(c)) {
        afterColon = false;
      }
    }
    if (hash !== -1) {
      const semi = text.lastIndexOf(';', lineEnd - 1);
      if (semi > hash) {
        edits?.push([semi, semi + 1, 0]);
        out += text.slice(last, semi);
        last = semi + 1;
      }
    }
    from = lineEnd >= text.length ? -1 : text.indexOf(keyword, lineEnd + 1);
  }
  return last === 0 ? text : out + text.slice(last);
}

// Mermaid hides `#name;` entity codes behind placeholder characters so `#` and `;` survive parsing.
export function encodeEntities(text: string, steps?: Edit[][]): string {
  if (!text.includes('#')) return text;
  text = dropColorSemicolons(dropColorSemicolons(text, 'style', steps), 'classDef', steps);
  const edits = recording(steps);
  return text.replace(RE_ENTITY, (s, at: number) => {
    const inner = s.substring(1, s.length - 1);
    const code = (RE_INT.test(inner) ? 'ﬂ°°' : 'ﬂ°') + inner + '¶ß';
    edits?.push([at, at + s.length, code.length]);
    return code;
  });
}
