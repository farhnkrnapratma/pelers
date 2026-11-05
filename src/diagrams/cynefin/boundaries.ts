import { num } from '../../svg/builder.js';

// The geometry of Mermaid's Cynefin boundaries: seeded wavy lines between the domains and the
// S-shaped cliff between Clear and Chaotic. Curves are flat lists of numbers: a start point, then
// six numbers (two control points and an end point) per cubic segment.

// mulberry32, one step.
export function seededRandom(seed: number): number {
  let t = (seed + 0x6d2b79f5) | 0;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

export function hashString(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  return hash;
}

// A configured seed other than zero wins; otherwise the seed comes from the diagram's id.
export function resolveSeed(configured: number | undefined, id: string): number {
  if (typeof configured === 'number' && Number.isFinite(configured) && configured !== 0) return configured;
  return hashString(id);
}

export interface WaveOptions {
  segments: number;
  vertical: boolean;
  // Multipliers Mermaid uses to vary the seed per point and per control point.
  pointStep: number;
  controlStep: number;
  controlShift: number;
  pinStart?: boolean;
  pinEnd?: boolean;
}

export const FOLD: WaveOptions = { segments: 7, vertical: true, pointStep: 17, controlStep: 31, controlShift: 7 };
export const HORIZON: WaveOptions = { segments: 7, vertical: false, pointStep: 23, controlStep: 37, controlShift: 11 };

// A wavy line along one axis from `from` to `to`, centered on `center` in the other axis.
// Points jitter by up to the amplitude; a pinned end stays on the center line.
export function wave(
  from: number,
  to: number,
  center: number,
  seed: number,
  amplitude: number,
  o: WaveOptions
): number[] {
  const step = (to - from) / o.segments;
  const along: number[] = [];
  const across: number[] = [];
  for (let i = 0; i <= o.segments; i++) {
    const pinned = (i === 0 && o.pinStart) || (i === o.segments && o.pinEnd);
    along.push(from + i * step);
    across.push(pinned ? center : center + seededRandom(seed + i * o.pointStep) * amplitude * 2 - amplitude);
  }
  const out = o.vertical ? [across[0], along[0]] : [along[0], across[0]];
  for (let i = 0; i < o.segments; i++) {
    const mid = (along[i] + along[i + 1]) / 2;
    const offset = amplitude * 1.5 * (i % 2 === 0 ? 1 : -1) * seededRandom(seed + i * o.controlStep + o.controlShift);
    if (o.vertical) out.push(across[i] + offset, mid, across[i + 1] - offset, mid, across[i + 1], along[i + 1]);
    else out.push(mid, across[i] + offset, mid, across[i + 1] - offset, along[i + 1], across[i + 1]);
  }
  return out;
}

// The cliff: a steep S from (cx, top) down to (cx, bottom).
export function cliff(cx: number, top: number, bottom: number, amplitude: number): number[] {
  const h = bottom - top;
  return [
    cx,
    top,
    cx + amplitude,
    top + h * 0.2,
    cx - amplitude * 1.5,
    top + h * 0.55,
    cx + amplitude * 0.5,
    top + h * 0.75,
    cx - amplitude,
    top + h * 0.85,
    cx + amplitude * 0.3,
    top + h * 0.95,
    cx,
    bottom,
  ];
}

// Path commands that continue from the curve's start point to its end.
export function forward(curve: number[]): string {
  let d = '';
  for (let i = 2; i < curve.length; i += 6) {
    d += `C${num(curve[i])},${num(curve[i + 1])} ${num(curve[i + 2])},${num(curve[i + 3])} ${num(curve[i + 4])},${num(curve[i + 5])}`;
  }
  return d;
}

// Path commands that continue from the curve's end point back to its start.
export function backward(curve: number[]): string {
  let d = '';
  for (let i = curve.length - 6; i >= 2; i -= 6) {
    d += `C${num(curve[i + 2])},${num(curve[i + 3])} ${num(curve[i])},${num(curve[i + 1])} ${num(curve[i - 2])},${num(curve[i - 1])}`;
  }
  return d;
}

export function curvePath(curve: number[]): string {
  return `M${num(curve[0])},${num(curve[1])}${forward(curve)}`;
}

export function ellipsePath(cx: number, cy: number, rx: number, ry: number): string {
  const r = `${num(rx)},${num(ry)}`;
  return `M${num(cx - rx)},${num(cy)}A${r} 0 1,1 ${num(cx + rx)},${num(cy)}A${r} 0 1,1 ${num(cx - rx)},${num(cy)}Z`;
}
