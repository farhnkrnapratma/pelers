import type { DiagramType } from '../detect.js';
import { isRecord, type Config } from '../preprocess.js';
import { esc } from './builder.js';
import { ink, plainColor } from './ink.js';

// Mermaid's palette theme variables, which set a diagram's own series colors. `first` is the
// number of the variable for the first series. Mindmap branches start at cScale1, as cScale0
// is the root's in Mermaid's styles. A venn palette repeats the colors it sets, as in Mermaid.
const PALETTES: Partial<Record<DiagramType, { prefix: string; first: number; repeat?: boolean }>> = {
  pie: { prefix: 'pie', first: 1 },
  gitGraph: { prefix: 'git', first: 0 },
  timeline: { prefix: 'cScale', first: 0 },
  radar: { prefix: 'cScale', first: 0 },
  treemap: { prefix: 'cScale', first: 0 },
  mindmap: { prefix: 'cScale', first: 1 },
  venn: { prefix: 'venn', first: 1, repeat: true },
};

const SLOTS = 8;
const RE_CHARS = /^[\w\s#%.,+\-/()]{1,200}$/;
const RE_FUNCTION = /([\w-]*)\s*\(/g;
const RE_VAR = /var\(\s*([\w-]*)/gi;
const FUNCTIONS = new Set([
  'rgb',
  'rgba',
  'hsl',
  'hsla',
  'hwb',
  'lab',
  'lch',
  'oklab',
  'oklch',
  'color',
  'color-mix',
  'light-dark',
  'var',
]);

const record = (value: unknown): Config => (isRecord(value) ? value : {});

// A color that can't load a resource, end its declaration or read the page's own variables.
// Values outside functions are checked as colors, so a typo keeps the theme color.
function safeColor(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  const color = value.trim();
  if (!RE_CHARS.test(color)) return undefined;
  if (!color.includes('(')) return plainColor(color) ? color : undefined;
  let depth = 0;
  for (const char of color) {
    if (char === '(') depth++;
    else if (char === ')' && --depth < 0) return undefined;
  }
  if (depth !== 0) return undefined;
  for (const [, name] of color.matchAll(RE_FUNCTION)) {
    if (!FUNCTIONS.has(name.toLowerCase())) return undefined;
  }
  for (const [, name] of color.matchAll(RE_VAR)) {
    if (!name.startsWith('--pele-')) return undefined;
  }
  return color;
}

// Splits at the commas between colors, not those inside rgb() and other functions.
function splitList(value: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < value.length; i++) {
    const char = value[i];
    if (char === '(') depth++;
    else if (char === ')') depth--;
    else if (char === ',' && depth === 0) {
      parts.push(value.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(value.slice(start));
  return parts.map((part) => part.trim());
}

function entries(type: DiagramType, vars: Config): { list: unknown[]; repeat: boolean } | undefined {
  if (type === 'xychart') {
    const list = record(vars.xyChart).plotColorPalette;
    return typeof list === 'string' ? { list: splitList(list), repeat: true } : undefined;
  }
  const palette = PALETTES[type];
  if (palette === undefined) return undefined;
  let list: unknown[] = [];
  for (let i = 0; i < SLOTS; i++) list.push(vars[palette.prefix + (palette.first + i)]);
  if (palette.repeat) list = list.filter((value) => value !== undefined && value !== null && value !== '');
  return { list, repeat: palette.repeat === true };
}

export interface Palette {
  style: string;
  // Label colors for text on each series, where the contrast of its color is known.
  inks: (string | undefined)[];
}

const NONE: Palette = { style: '', inks: [] };

// The --pele-series-* declarations for the palette a diagram's theme variables set. An entry
// that isn't a usable color leaves its series in the theme's color.
export function palette(type: DiagramType, config: Config): Palette {
  const found = entries(type, record(config.themeVariables));
  if (found === undefined || found.list.length === 0) return NONE;
  const { list, repeat } = found;
  let style = '';
  const inks: (string | undefined)[] = [];
  for (let i = 0; i < SLOTS; i++) {
    if (!repeat && i >= list.length) break;
    const color = safeColor(list[i % list.length]);
    if (color === undefined) continue;
    style += `;--pele-series-${i + 1}:${color}`;
    inks[i] = ink(`;fill:${color}`);
  }
  return style === '' ? NONE : { style: esc(style), inks };
}

let current = NONE;

export function withPalette<T>(next: Palette, run: () => T): T {
  const outer = current;
  current = next;
  try {
    return run();
  } finally {
    current = outer;
  }
}

export const paletteStyle = (): string => current.style;

// A fill for a label on the series color, when the palette sets one that needs it.
export function seriesInkFill(index: number): string {
  const color = current.inks[((index % SLOTS) + SLOTS) % SLOTS];
  return color ? ` fill="${color}"` : '';
}
