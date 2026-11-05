import type { ArchEdge, ArchGroup, ArchLayoutHint, ArchNode } from './db.js';
import type { Side } from './parser.js';

// Grid placement. Mermaid hands the edge sides to a force-directed solver as relative placement
// constraints. Pele applies them directly: every service and junction takes a cell, an edge
// `a:R -- L:b` puts b in the cell to the right of a, and a group is a solid rectangle of cells, so a
// group never encloses a node that is not in it. Each container (the diagram, then every group) is
// laid out on its own, innermost first, and a finished group is placed in its parent as one block.

export interface Grid {
  // Cell of each node, in the order of the nodes passed in.
  col: Int32Array;
  row: Int32Array;
  // Cell rectangle and parent of each group. A group with nothing in it takes one cell.
  groupCol: Int32Array;
  groupRow: Int32Array;
  groupCols: Int32Array;
  groupRows: Int32Array;
  groupParent: Int32Array;
  groupEmpty: Uint8Array;
  nodeParent: Int32Array;
}

const DX = [-1, 1, 0, 0];
const DY = [0, 0, -1, 1];

export function sideIndex(side: Side): number {
  return side === 'L' ? 0 : side === 'R' ? 1 : side === 'T' ? 2 : 3;
}

// Blocks up to this many cells are found through the cell map; larger ones are compared one by one.
const SMALL = 64;
const OFFSET = 2 ** 25;
const SPAN = 2 ** 26;

export function placeOnGrid(nodes: ArchNode[], groups: ArchGroup[], edges: ArchEdge[], hints: ArchLayoutHint[]): Grid {
  const N = nodes.length;
  const G = groups.length;
  const root = G;
  const nodeIndex = new Map<string, number>();
  const groupIndex = new Map<string, number>();
  // Items are nodes (0..N-1) then groups (N..N+G-1). Containers are groups (0..G-1) then the root.
  const parent = new Int32Array(N + G).fill(root);
  const depth = new Int32Array(G + 1);
  const kids: number[][] = [];
  for (let c = 0; c <= G; c++) kids.push([]);
  groups.forEach((group, g) => {
    groupIndex.set(group.id, g);
    const p = group.in === undefined ? undefined : groupIndex.get(group.in);
    if (p !== undefined && p !== g) {
      parent[N + g] = p;
      depth[g] = depth[p] + 1;
    } else {
      depth[g] = 1;
    }
  });
  nodes.forEach((node, i) => {
    nodeIndex.set(node.id, i);
    const p = node.in === undefined ? undefined : groupIndex.get(node.in);
    if (p !== undefined) parent[i] = p;
    kids[parent[i]].push(i);
  });
  for (let g = 0; g < G; g++) kids[parent[N + g]].push(N + g);

  // A link asks for one node to sit beside another. It belongs to the innermost container that
  // holds both ends, and there it relates the two children of that container the ends are in.
  const linkA: number[] = [];
  const linkB: number[] = [];
  const sideA: number[] = [];
  const sideB: number[] = [];
  const itemA: number[] = [];
  const itemB: number[] = [];
  const hintLinks: (number[] | undefined)[] = new Array(N + G);
  const edgeLinks: (number[] | undefined)[] = new Array(N + G);
  const addLink = (a: number, b: number, sa: number, sb: number, lists: (number[] | undefined)[]): void => {
    if (a === b) return;
    let ia = a;
    let ib = b;
    let ca = parent[a];
    let cb = parent[b];
    while (ca !== cb) {
      if (depth[ca] >= depth[cb]) {
        ia = N + ca;
        ca = parent[ia];
      } else {
        ib = N + cb;
        cb = parent[ib];
      }
    }
    const k = linkA.length;
    linkA.push(a);
    linkB.push(b);
    sideA.push(sa);
    sideB.push(sb);
    itemA.push(ia);
    itemB.push(ib);
    (lists[ia] ??= []).push(k);
    (lists[ib] ??= []).push(k);
  };
  for (const hint of hints) {
    const row = hint.direction === 'row';
    for (let i = 1; i < hint.members.length; i++) {
      const a = nodeIndex.get(hint.members[i - 1]);
      const b = nodeIndex.get(hint.members[i]);
      if (a !== undefined && b !== undefined) addLink(a, b, row ? 1 : 3, row ? 0 : 2, hintLinks);
    }
  }
  // An edge between two like sides says least about where its ends go, so those come last.
  for (const likeSides of [false, true]) {
    for (const edge of edges) {
      if ((edge.lhsDir === edge.rhsDir) !== likeSides) continue;
      const a = nodeIndex.get(edge.lhsId);
      const b = nodeIndex.get(edge.rhsId);
      if (a !== undefined && b !== undefined) addLink(a, b, sideIndex(edge.lhsDir), sideIndex(edge.rhsDir), edgeLinks);
    }
  }

  // Position of each item in its container, and its size in cells.
  const x = new Int32Array(N + G);
  const y = new Int32Array(N + G);
  const w = new Int32Array(N + G).fill(1);
  const h = new Int32Array(N + G).fill(1);
  const placed = new Uint8Array(N + G);

  const order: number[] = [];
  const cells = new Map<number, number>();
  const large: number[] = [];
  const reach = new Map<string, number>();
  let first = 0;
  let ax = 0;
  let ay = 0;

  const anchor = (node: number, item: number): void => {
    ax = 0;
    ay = 0;
    for (let cur = node; cur !== item; cur = N + parent[cur]) {
      ax += x[cur];
      ay += y[cur];
    }
  };

  const put = (item: number, px: number, py: number): void => {
    x[item] = px;
    y[item] = py;
    placed[item] = 1;
    order.push(item);
    if (w[item] * h[item] > SMALL) {
      large.push(item);
      return;
    }
    for (let cy = py; cy < py + h[item]; cy++) {
      for (let cx = px; cx < px + w[item]; cx++) cells.set((cx + OFFSET) * SPAN + cy + OFFSET, item);
    }
  };

  const overlaps = (item: number, rx: number, ry: number, rw: number, rh: number): boolean =>
    x[item] < rx + rw && rx < x[item] + w[item] && y[item] < ry + rh && ry < y[item] + h[item];

  const collide = (rx: number, ry: number, rw: number, rh: number): number => {
    if (rw * rh > SMALL) {
      for (let i = first; i < order.length; i++) if (overlaps(order[i], rx, ry, rw, rh)) return order[i];
      return -1;
    }
    for (let cy = ry; cy < ry + rh; cy++) {
      for (let cx = rx; cx < rx + rw; cx++) {
        const hit = cells.get((cx + OFFSET) * SPAN + cy + OFFSET);
        if (hit !== undefined) return hit;
      }
    }
    for (const item of large) if (overlaps(item, rx, ry, rw, rh)) return item;
    return -1;
  };

  // How far a block must slide along (sx, sy) to be clear. Each step jumps past what it hit, and
  // the result is kept, because a later block starting from the same place cannot stop sooner.
  const slide = (rx: number, ry: number, rw: number, rh: number, sx: number, sy: number): number => {
    const key = `${rx},${ry},${rw},${rh},${sx},${sy}`;
    let k = reach.get(key) ?? 0;
    for (;;) {
      const hit = collide(rx + k * sx, ry + k * sy, rw, rh);
      if (hit < 0) break;
      k = sx > 0 ? x[hit] + w[hit] - rx : sx < 0 ? rx + rw - x[hit] : sy > 0 ? y[hit] + h[hit] - ry : ry + rh - y[hit];
    }
    reach.set(key, k);
    return k;
  };

  const follow = (from: number, k: number): void => {
    const forward = itemA[k] === from;
    const to = forward ? itemB[k] : itemA[k];
    if (placed[to]) return;
    const sp = forward ? sideA[k] : sideB[k];
    const sq = forward ? sideB[k] : sideA[k];
    anchor(forward ? linkA[k] : linkB[k], from);
    const pax = ax;
    const pay = ay;
    anchor(forward ? linkB[k] : linkA[k], to);
    let dx = DX[sp];
    let dy = DY[sp];
    let shiftX = 0;
    let shiftY = 0;
    const facing = DX[sp] + DX[sq] === 0 && DY[sp] + DY[sq] === 0;
    if (sp === sq) {
      // Like sides: the edge loops around, so the nodes stack across the direction it leaves in.
      dx = DX[sp] === 0 ? 1 : 0;
      dy = 1 - dx;
    } else if (!facing) {
      // A bend: one step along the side the edge leaves, one step back from the side it enters.
      shiftX = -DX[sq];
      shiftY = -DY[sq];
    }
    let bx: number;
    let by: number;
    if (dx !== 0) {
      bx = dx > 0 ? x[from] + w[from] : x[from] - w[to];
      by = y[from] + pay + shiftY - ay;
    } else {
      by = dy > 0 ? y[from] + h[from] : y[from] - h[to];
      bx = x[from] + pax + shiftX - ax;
    }
    if (facing) {
      // When several placed nodes reach this one through the same pair of sides, line it up
      // with the middle one, not with whichever happened to be visited first.
      const list = edgeLinks[to];
      if (list !== undefined && list.length > 1) {
        const wanted: number[] = [];
        for (const j of list) {
          const toIsB = itemB[j] === to;
          const other = toIsB ? itemA[j] : itemB[j];
          if (!placed[other] || (toIsB ? sideA[j] : sideB[j]) !== sp || (toIsB ? sideB[j] : sideA[j]) !== sq) continue;
          anchor(toIsB ? linkA[j] : linkB[j], other);
          const at = dx !== 0 ? y[other] + ay : x[other] + ax;
          anchor(toIsB ? linkB[j] : linkA[j], to);
          wanted.push(at - (dx !== 0 ? ay : ax));
        }
        if (wanted.length > 1) {
          wanted.sort((a, b) => a - b);
          if (dx !== 0) by = wanted[(wanted.length - 1) >> 1];
          else bx = wanted[(wanted.length - 1) >> 1];
        }
      }
      // Taken: move sideways, to whichever side is nearer, so the edge can still leave straight.
      const px = dx === 0 ? 1 : 0;
      const py = 1 - px;
      const forth = slide(bx, by, w[to], h[to], px, py);
      const back = forth === 0 ? 0 : slide(bx, by, w[to], h[to], -px, -py);
      const near = forth <= back ? forth : -back;
      bx += px * near;
      by += py * near;
    } else {
      const far = slide(bx, by, w[to], h[to], dx, dy);
      bx += dx * far;
      by += dy * far;
    }
    put(to, bx, by);
  };

  const layout = (c: number): void => {
    const items = kids[c];
    if (items.length === 0) return;
    order.length = 0;
    const starts: number[] = [];
    for (const start of items) {
      if (placed[start]) continue;
      first = order.length;
      starts.push(first);
      cells.clear();
      large.length = 0;
      reach.clear();
      put(start, 0, 0);
      // Declared alignments are followed as soon as a node is placed, ahead of any edge.
      let hintAt = first;
      let edgeAt = first;
      let linkAt = 0;
      for (;;) {
        if (hintAt < order.length) {
          const from = order[hintAt++];
          const list = hintLinks[from];
          if (list !== undefined) for (const k of list) follow(from, k);
          continue;
        }
        if (edgeAt >= order.length) break;
        const list = edgeLinks[order[edgeAt]];
        if (list === undefined || linkAt >= list.length) {
          edgeAt++;
          linkAt = 0;
          continue;
        }
        follow(order[edgeAt], list[linkAt++]);
      }
    }
    starts.push(order.length);

    // Unconnected parts go side by side, wrapping into rows that keep the whole roughly square.
    const count = starts.length - 1;
    const minX = new Array<number>(count);
    const minY = new Array<number>(count);
    const partW = new Array<number>(count);
    const partH = new Array<number>(count);
    let area = 0;
    let widest = 0;
    for (let p = 0; p < count; p++) {
      let x0 = Infinity;
      let y0 = Infinity;
      let x1 = -Infinity;
      let y1 = -Infinity;
      for (let i = starts[p]; i < starts[p + 1]; i++) {
        const item = order[i];
        x0 = Math.min(x0, x[item]);
        y0 = Math.min(y0, y[item]);
        x1 = Math.max(x1, x[item] + w[item]);
        y1 = Math.max(y1, y[item] + h[item]);
      }
      minX[p] = x0;
      minY[p] = y0;
      partW[p] = x1 - x0;
      partH[p] = y1 - y0;
      area += partW[p] * partH[p];
      widest = Math.max(widest, partW[p]);
    }
    const limit = Math.max(widest, Math.ceil(Math.sqrt(area * 2)));
    let cx = 0;
    let cy = 0;
    let shelf = 0;
    let width = 0;
    for (let p = 0; p < count; p++) {
      if (cx > 0 && cx + partW[p] > limit) {
        cy += shelf;
        cx = 0;
        shelf = 0;
      }
      for (let i = starts[p]; i < starts[p + 1]; i++) {
        x[order[i]] += cx - minX[p];
        y[order[i]] += cy - minY[p];
      }
      cx += partW[p];
      shelf = Math.max(shelf, partH[p]);
      width = Math.max(width, cx);
    }
    if (c < G) {
      w[N + c] = width;
      h[N + c] = cy + shelf;
    }
  };
  for (let c = G - 1; c >= 0; c--) layout(c);
  layout(root);

  const groupCol = new Int32Array(G);
  const groupRow = new Int32Array(G);
  const groupParent = new Int32Array(G);
  const groupEmpty = new Uint8Array(G);
  for (let g = 0; g < G; g++) {
    const p = parent[N + g];
    groupParent[g] = p === root ? -1 : p;
    groupCol[g] = x[N + g] + (p === root ? 0 : groupCol[p]);
    groupRow[g] = y[N + g] + (p === root ? 0 : groupRow[p]);
    if (kids[g].length === 0) groupEmpty[g] = 1;
  }
  const col = new Int32Array(N);
  const row = new Int32Array(N);
  const nodeParent = new Int32Array(N);
  for (let i = 0; i < N; i++) {
    const p = parent[i];
    nodeParent[i] = p === root ? -1 : p;
    col[i] = x[i] + (p === root ? 0 : groupCol[p]);
    row[i] = y[i] + (p === root ? 0 : groupRow[p]);
  }
  return {
    col,
    row,
    groupCol,
    groupRow,
    groupCols: w.slice(N),
    groupRows: h.slice(N),
    groupParent,
    groupEmpty,
    nodeParent,
  };
}
