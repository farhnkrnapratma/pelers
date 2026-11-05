import { PeleError } from '../../errors.js';

export interface QuadrantText {
  text: string;
  type: 'text' | 'markdown';
}

export interface PointStyle {
  radius?: number;
  color?: string;
  strokeColor?: string;
  strokeWidth?: string;
}

export interface QuadrantPoint extends PointStyle {
  x: number;
  y: number;
  text: QuadrantText;
  className: string;
}

export interface QuadrantModel {
  type: 'quadrantChart';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  // Quadrants 1 to 4: top right, top left, bottom left, bottom right.
  quadrants: (QuadrantText | undefined)[];
  xAxisLeft: QuadrantText | undefined;
  xAxisRight: QuadrantText | undefined;
  yAxisBottom: QuadrantText | undefined;
  yAxisTop: QuadrantText | undefined;
  points: QuadrantPoint[];
  classes: Map<string, PointStyle>;
}

const RE_NUMBER = /^\d+$/;
const RE_HEX = /^#?([\dA-Fa-f]{6}|[\dA-Fa-f]{3})$/;
const RE_PIXELS = /^\d+px$/;

function invalid(style: string, value: string, kind: string): PeleError {
  return new PeleError(`value for ${style} ${value} is invalid, please use a valid ${kind}`, 'semantic', {
    type: 'quadrantChart',
  });
}

// The first two parts of Mermaid's `style.trim().split(/\s*:\s*/)`, found in one pass.
function keyValue(style: string): [string, string] {
  const s = style.trim();
  const colon = s.indexOf(':');
  if (colon === -1) return [s, 'undefined'];
  const rest = s.slice(colon + 1).trimStart();
  const next = rest.indexOf(':');
  return [s.slice(0, colon).trimEnd(), next === -1 ? rest : rest.slice(0, next).trimEnd()];
}

export function parseStyles(styles: string[]): PointStyle {
  const out: PointStyle = {};
  for (const style of styles) {
    const [key, value] = keyValue(style);
    if (key === 'radius') {
      if (!RE_NUMBER.test(value)) throw invalid(key, value, 'number');
      out.radius = parseInt(value);
    } else if (key === 'color') {
      if (!RE_HEX.test(value)) throw invalid(key, value, 'hex code');
      out.color = value;
    } else if (key === 'stroke-color') {
      if (!RE_HEX.test(value)) throw invalid(key, value, 'hex code');
      out.strokeColor = value;
    } else if (key === 'stroke-width') {
      if (!RE_PIXELS.test(value)) throw invalid(key, value, 'number of pixels (eg. 10px)');
      out.strokeWidth = value;
    } else {
      throw new PeleError(`style named ${key} is not supported.`, 'semantic', { type: 'quadrantChart' });
    }
  }
  return out;
}

function trimmed(label: QuadrantText): QuadrantText {
  return { text: label.text.trim(), type: label.type };
}

export class QuadrantDb implements QuadrantModel {
  readonly type = 'quadrantChart';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  quadrants: (QuadrantText | undefined)[] = [undefined, undefined, undefined, undefined];
  xAxisLeft: QuadrantText | undefined;
  xAxisRight: QuadrantText | undefined;
  yAxisBottom: QuadrantText | undefined;
  yAxisTop: QuadrantText | undefined;
  points: QuadrantPoint[] = [];
  classes = new Map<string, PointStyle>();

  setQuadrant1Text(label: QuadrantText): void {
    this.quadrants[0] = trimmed(label);
  }

  setQuadrant2Text(label: QuadrantText): void {
    this.quadrants[1] = trimmed(label);
  }

  setQuadrant3Text(label: QuadrantText): void {
    this.quadrants[2] = trimmed(label);
  }

  setQuadrant4Text(label: QuadrantText): void {
    this.quadrants[3] = trimmed(label);
  }

  setXAxisLeftText(label: QuadrantText): void {
    this.xAxisLeft = trimmed(label);
  }

  setXAxisRightText(label: QuadrantText): void {
    this.xAxisRight = trimmed(label);
  }

  setYAxisBottomText(label: QuadrantText): void {
    this.yAxisBottom = trimmed(label);
  }

  setYAxisTopText(label: QuadrantText): void {
    this.yAxisTop = trimmed(label);
  }

  addPoint(label: QuadrantText, className: string, x: string, y: string, styles: string[]): void {
    this.points.push({ x: Number(x), y: Number(y), text: trimmed(label), className, ...parseStyles(styles) });
  }

  addClass(className: string, styles: string[]): void {
    this.classes.set(className, parseStyles(styles));
  }

  setDiagramTitle(text: string): void {
    this.title = text;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }
}
