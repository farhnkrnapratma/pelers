import type { CNode } from '../layout/compound.js';
import type { Label } from '../text/label.js';
import { labelSvg, num, type IconResolver } from './builder.js';

// The x of every route point, by its rounded y: the edges that cross a cluster's top border.
export type Crossings = Map<number, number[]>;

export function markCrossings(crossings: Crossings, route: number[]): void {
  for (let k = 0; k < route.length; k += 3) {
    const key = Math.round(route[k + 1]);
    const list = crossings.get(key);
    if (list) list.push(route[k]);
    else crossings.set(key, [route[k]]);
  }
}

// Picks where a cluster title sits along the top edge so that edges entering there do not cross it.
export function clusterTitleX(c: CNode, width: number, crossings: Crossings, pad: number): number {
  if (width === 0) return c.x;
  const top = c.y - c.h / 2;
  const left = c.x - c.w / 2;
  const half = width / 2;
  const candidates = [left + pad - 6 + half, c.x, left + c.w - pad + 6 - half];
  const xs = crossings.get(Math.round(top));
  if (xs === undefined) return candidates[0];
  let best = candidates[0];
  let bestGap = -1;
  for (const x of candidates) {
    let gap = Infinity;
    for (const cx of xs) if (cx > left && cx < left + c.w) gap = Math.min(gap, Math.abs(cx - x) - half);
    if (gap >= 6) return x;
    if (gap > bestGap + 0.5) {
      bestGap = gap;
      best = x;
    }
  }
  return best;
}

// Whether an edge that enters at the top runs through a title placed at `x`.
export function titleCrossed(c: CNode, width: number, x: number, crossings: Crossings): boolean {
  const xs = width > 0 ? crossings.get(Math.round(c.y - c.h / 2)) : undefined;
  if (xs === undefined) return false;
  const left = c.x - c.w / 2;
  for (const cx of xs) if (cx > left && cx < left + c.w && Math.abs(cx - x) < width / 2 + 4) return true;
  return false;
}

// A title that an edge runs through, to be drawn over the edges: the text, on an outline in the
// color of the cluster behind it, so that only the line is seen to stop short of the letters.
// `nodes` and `index` find how many clusters deep the title is, each of which tints the background.
export function struckTitle(
  label: Label,
  x: number,
  y: number,
  attrs: string,
  id: string,
  nodes: readonly { parent: number }[],
  index: number,
  icons?: IconResolver
): string {
  let depth = 1;
  for (let p = nodes[index].parent; p >= 0 && depth < 8; p = nodes[p].parent) depth++;
  // Each cluster is half its fill over what is behind it.
  const fill = `color-mix(in srgb,var(--_a) ${num(100 * (1 - 0.5 ** depth))}%,var(--_bg))`;
  return (
    `<g class="pele-cluster-title" data-id="${id}">` +
    labelSvg(label, x, y, ` fill="none" stroke="${fill}" stroke-width="5" stroke-linejoin="round"`, icons) +
    labelSvg(label, x, y, attrs, icons) +
    '</g>'
  );
}
