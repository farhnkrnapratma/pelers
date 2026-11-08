import { PeleError } from '../../errors.js';
import type { RadarAst } from './parser.js';

export interface RadarAxis {
  name: string;
  label: string;
}

export interface RadarCurve {
  name: string;
  label: string;
  entries: number[];
}

export interface RadarOptions {
  showLegend: boolean;
  ticks: number;
  max: number | null;
  min: number;
  graticule: 'circle' | 'polygon';
}

export interface RadarModel {
  type: 'radar';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  axes: RadarAxis[];
  curves: RadarCurve[];
  options: RadarOptions;
}

const MAX_TICKS = 32;

function invalid(message: string): PeleError {
  return new PeleError(message, 'semantic', { type: 'radar' });
}

export function buildRadar(ast: RadarAst, title: string | undefined): RadarModel {
  const axes = ast.axes.map((axis) => ({ name: axis.name, label: axis.label ?? axis.name }));

  const curves = ast.curves.map((curve) => {
    let entries: number[];
    if (curve.entries[0].axis === undefined) {
      entries = curve.entries.map((entry) => entry.value);
    } else {
      if (axes.length === 0) throw invalid('Axes must be populated before curves for reference entries');
      const byAxis = new Map<string, number>();
      for (const entry of curve.entries) {
        if (entry.axis !== undefined && !byAxis.has(entry.axis.$refText)) byAxis.set(entry.axis.$refText, entry.value);
      }
      entries = axes.map((axis) => {
        const value = byAxis.get(axis.name);
        if (value === undefined) throw invalid('Missing entry for axis ' + axis.label);
        return value;
      });
    }
    return { name: curve.name, label: curve.label ?? curve.name, entries };
  });

  const options: RadarOptions = { showLegend: true, ticks: 5, max: null, min: 0, graticule: 'circle' };
  for (const option of ast.options) {
    if (option.name === 'showLegend') options.showLegend = option.value as boolean;
    else if (option.name === 'graticule') options.graticule = option.value as 'circle' | 'polygon';
    else options[option.name] = option.value as number;
  }
  if (options.ticks > MAX_TICKS) options.ticks = MAX_TICKS;

  return {
    type: 'radar',
    title: ast.title || title,
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : undefined,
    axes,
    curves,
    options,
  };
}
