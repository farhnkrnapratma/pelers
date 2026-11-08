import type { TreemapNode } from './model.js';

export interface Box {
  node: TreemapNode;
  parent: Box | undefined;
  // Position among the top-level nodes of the branch this box belongs to, which picks its color.
  series: number;
  // A leaf's own value, or the sum of the leaves under a section.
  value: number;
  children: Box[];
  // Small sections give their header space to their children.
  headed: boolean;
  header: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Insets {
  gap: number;
  // Space a section keeps for its header, and around its children on the other three sides.
  header: number;
  short: number;
  side: number;
}

function box(node: TreemapNode, parent: Box | undefined, series: number): Box {
  const value = node.children === undefined && Number.isFinite(node.value) && node.value! > 0 ? node.value! : 0;
  return { node, parent, series, value, children: [], headed: false, header: 0, x: 0, y: 0, w: 0, h: 0 };
}

const clamp = (v: number, low: number, high: number): number => (v >= low ? Math.min(v, high) : low);

// Lays out boxes[from..to) in the rectangle as one squarified treemap: rows are added along the
// shorter side, and a row stops growing when another box would make its boxes less square.
function squarify(boxes: Box[], total: number, x: number, y: number, w: number, h: number, gap: number): void {
  let rest = total;
  let from = 0;
  while (from < boxes.length) {
    const scale = (w * h) / rest;
    const side = Math.min(w, h);
    let sum = boxes[from].value * scale;
    const largest = sum;
    let worst = Math.max((side * side * largest) / (sum * sum), (sum * sum) / (side * side * largest));
    let to = from + 1;
    while (to < boxes.length) {
      const smallest = boxes[to].value * scale;
      const next = sum + smallest;
      const ratio = Math.max((side * side * largest) / (next * next), (next * next) / (side * side * smallest));
      if (ratio > worst) break;
      worst = ratio;
      sum = next;
      to++;
    }

    const thickness = sum / side;
    let offset = 0;
    for (let i = from; i < to; i++) {
      const b = boxes[i];
      const length = i === to - 1 ? side - offset : (b.value * scale) / thickness;
      const bx = w >= h ? x : x + offset;
      const by = w >= h ? y + offset : y;
      const bw = w >= h ? thickness : length;
      const bh = w >= h ? length : thickness;
      // Omit the gap if it would erase the box.
      const gx = Math.round(bx + bw - gap / 2) > Math.round(bx + gap / 2) ? gap / 2 : 0;
      const gy = Math.round(by + bh - gap / 2) > Math.round(by + gap / 2) ? gap / 2 : 0;
      b.x = Math.round(bx + gx);
      b.y = Math.round(by + gy);
      b.w = Math.round(bx + bw - gx) - b.x;
      b.h = Math.round(by + bh - gy) - b.y;
      offset += length;
      rest -= b.value;
    }
    if (w >= h) {
      x += thickness;
      w -= thickness;
    } else {
      y += thickness;
      h -= thickness;
    }
    from = to;
  }
}

// Returns boxes with positive values, including zero-size boxes, parents before children.
export function layoutTreemap(roots: TreemapNode[], width: number, height: number, insets: Insets): Box[] {
  const top = roots.map((node, index) => box(node, undefined, index));
  const all = top.slice();
  for (let i = 0; i < all.length; i++) {
    const parent = all[i];
    for (const node of parent.node.children ?? []) {
      const child = box(node, parent, parent.series);
      parent.children.push(child);
      all.push(child);
    }
  }
  for (let i = all.length - 1; i >= 0; i--) {
    const b = all[i];
    if (b.parent) b.parent.value += b.value;
  }

  const { gap, header, short, side } = insets;
  const placed: Box[] = [];
  const place = (children: Box[], x: number, y: number, w: number, h: number): void => {
    const shown = children.filter((b) => b.value > 0).sort((a, b) => b.value - a.value);
    let total = 0;
    for (const b of shown) total += b.value;
    if (shown.length === 0 || !Number.isFinite(total)) return;
    w = Math.max(0, w);
    h = Math.max(0, h);
    if (w > 0 && h > 0) squarify(shown, total, x - gap / 2, y - gap / 2, w + gap, h + gap, gap);
    for (const b of shown) {
      const x0 = clamp(b.x, x, x + w);
      const y0 = clamp(b.y, y, y + h);
      b.w = clamp(b.x + b.w, x0, x + w) - x0;
      b.h = clamp(b.y + b.h, y0, y + h) - y0;
      b.x = x0;
      b.y = y0;
      placed.push(b);
    }
  };

  place(top, 0, 0, width, height);
  for (let i = 0; i < placed.length; i++) {
    const b = placed[i];
    if (b.children.length === 0) continue;
    const top = b.h - header - side >= 12 ? header : short;
    b.headed = b.w > Math.max(24, 2 * side) && b.h - top - side >= 8;
    b.header = b.headed ? top : 0;
    if (b.headed) place(b.children, b.x + side, b.y + top, b.w - 2 * side, b.h - top - side);
    else place(b.children, b.x, b.y, b.w, b.h);
  }
  return placed;
}
