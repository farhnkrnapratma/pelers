import { PeleError } from '../../errors.js';
import type { SankeyModel } from './db.js';

export type Alignment = 'justify' | 'left' | 'right' | 'center';

export interface LayoutOptions {
  width: number;
  height: number;
  nodeWidth: number;
  nodePadding: number;
  // Height of a label line; thin nodes are kept at least this far apart where there is room.
  labelHeight: number;
  alignment: Alignment;
}

export interface PlacedNode {
  layer: number;
  // Total flow through the node, in the diagram's units.
  value: number;
  x: number;
  y: number;
  height: number;
}

export interface PlacedLink {
  source: number;
  target: number;
  value: number;
  thickness: number;
  // Vertical centers of the band where it leaves its source and where it enters its target.
  sourceY: number;
  targetY: number;
}

export interface SankeyLayout {
  nodes: PlacedNode[];
  links: PlacedLink[];
}

const PASSES = 6;

// Most nodes have a handful of bands, and the built-in sort is slow to start on short lists.
function sort(list: number[], compare: (a: number, b: number) => number): void {
  if (list.length > 8) {
    list.sort(compare);
    return;
  }
  for (let i = 1; i < list.length; i++) {
    const item = list[i];
    let j = i - 1;
    while (j >= 0 && compare(list[j], item) > 0) {
      list[j + 1] = list[j];
      j--;
    }
    list[j + 1] = item;
  }
}

// Places nodes in columns by their distance from a source, sizes them by their flow, and
// then lets each column settle next to its neighbours so that the bands cross less.
export function layoutSankey(model: SankeyModel, options: LayoutOptions): SankeyLayout {
  const n = model.nodes.length;
  const links: PlacedLink[] = model.links.map((link) => ({
    source: link.source.index,
    target: link.target.index,
    value: Number.isFinite(link.value) && link.value > 0 ? link.value : 0,
    thickness: 0,
    sourceY: 0,
    targetY: 0,
  }));
  const outgoing: number[][] = [];
  const incoming: number[][] = [];
  for (let i = 0; i < n; i++) {
    outgoing.push([]);
    incoming.push([]);
  }
  links.forEach((link, k) => {
    outgoing[link.source].push(k);
    incoming[link.target].push(k);
  });

  // Longest distance from a source, in topological order; what is left over lies on a cycle.
  const depth = new Int32Array(n);
  const pending = new Int32Array(n);
  const order: number[] = [];
  for (let i = 0; i < n; i++) {
    pending[i] = incoming[i].length;
    if (pending[i] === 0) order.push(i);
  }
  for (let q = 0; q < order.length; q++) {
    const i = order[q];
    for (const k of outgoing[i]) {
      const t = links[k].target;
      if (depth[t] < depth[i] + 1) depth[t] = depth[i] + 1;
      if (--pending[t] === 0) order.push(t);
    }
  }
  if (order.length < n) {
    // Every node left over has a predecessor that is left over too; walking back finds the loop.
    let stuck = 0;
    while (pending[stuck] === 0) stuck++;
    const seen = new Uint8Array(n);
    while (seen[stuck] === 0) {
      seen[stuck] = 1;
      for (const k of incoming[stuck]) {
        if (pending[links[k].source] > 0) {
          stuck = links[k].source;
          break;
        }
      }
    }
    throw new PeleError(
      `Sankey diagrams cannot have circular links, and "${model.nodes[stuck].id}" is part of one.`,
      'semantic',
      { type: 'sankey' }
    );
  }
  let layers = 1;
  for (let i = 0; i < n; i++) if (depth[i] + 1 > layers) layers = depth[i] + 1;
  const reach = new Int32Array(n);
  for (let q = n - 1; q >= 0; q--) {
    const i = order[q];
    for (const k of outgoing[i]) if (reach[i] < reach[links[k].target] + 1) reach[i] = reach[links[k].target] + 1;
  }

  // Flows are scaled to the largest one, so that sums of huge values stay finite.
  let largest = 0;
  for (const link of links) if (link.value > largest) largest = link.value;
  const unit = largest > 0 ? 1 / largest : 0;

  const nodes: PlacedNode[] = [];
  const flow = new Float64Array(n);
  const columns: number[][] = [];
  for (let l = 0; l < layers; l++) columns.push([]);
  const step = layers > 1 ? (options.width - options.nodeWidth) / (layers - 1) : 0;
  for (let i = 0; i < n; i++) {
    let layer: number;
    if (options.alignment === 'left') {
      layer = depth[i];
    } else if (options.alignment === 'right') {
      layer = layers - 1 - reach[i];
    } else if (options.alignment === 'center') {
      layer = depth[i];
      if (incoming[i].length === 0 && outgoing[i].length > 0) {
        layer = layers;
        for (const k of outgoing[i]) if (depth[links[k].target] - 1 < layer) layer = depth[links[k].target] - 1;
      }
    } else {
      layer = outgoing[i].length > 0 ? depth[i] : layers - 1;
    }
    let into = 0;
    let out = 0;
    let value = 0;
    let valueOut = 0;
    for (const k of incoming[i]) {
      into += links[k].value * unit;
      value += links[k].value;
    }
    for (const k of outgoing[i]) {
      out += links[k].value * unit;
      valueOut += links[k].value;
    }
    flow[i] = Math.max(into, out);
    nodes.push({ layer, value: Math.max(value, valueOut), x: layer * step, y: 0, height: 0 });
    columns[layer].push(i);
  }

  let tallest = 1;
  for (const column of columns) if (column.length > tallest) tallest = column.length;
  const gap = tallest > 1 ? Math.min(options.nodePadding, options.height / (tallest - 1)) : options.nodePadding;
  let scale = Infinity;
  for (const column of columns) {
    let sum = 0;
    for (const i of column) sum += flow[i];
    if (sum > 0) scale = Math.min(scale, (options.height - (column.length - 1) * gap) / sum);
  }
  if (!Number.isFinite(scale) || scale < 0) scale = 0;

  // A thin node still needs room for its label. Where every column can afford it, each node
  // claims at least a label's height, and the flows are scaled down to make that fit.
  let claim = Math.max(0, options.labelHeight - gap);
  const fits = (s: number): boolean => {
    for (const column of columns) {
      let extent = (column.length - 1) * gap;
      for (const i of column) extent += Math.max(flow[i] * s, claim);
      if (extent > options.height + 1e-6) return false;
    }
    return true;
  };
  if (claim > 0 && !fits(scale)) {
    if (fits(0)) {
      let low = 0;
      for (let k = 0; k < 40; k++) {
        const mid = (low + scale) / 2;
        if (fits(mid)) low = mid;
        else scale = mid;
      }
      scale = low;
    } else {
      claim = 0;
    }
  }

  // The room a node keeps clear above and below itself.
  const margin = new Float64Array(n);
  for (const link of links) link.thickness = link.value * unit * scale;
  for (const column of columns) {
    let y = 0;
    for (const i of column) {
      nodes[i].height = flow[i] * scale;
      margin[i] = Math.max(0, claim - nodes[i].height) / 2;
      nodes[i].y = y + margin[i];
      y += nodes[i].height + 2 * margin[i] + gap;
    }
    // Spare room is shared out between the nodes of a column.
    const spare = (options.height - y + gap) / (column.length + 1);
    column.forEach((i, k) => (nodes[i].y += spare * (k + 1)));
  }

  const center = (i: number): number => nodes[i].y + nodes[i].height / 2;

  // Bands leave and enter a node in the order of where they go, so they do not cross at it.
  // These are the offsets of each band's middle from the top of the node at either end.
  const sourceOffset = new Float64Array(links.length);
  const targetOffset = new Float64Array(links.length);
  const byTarget = (a: number, b: number): number => center(links[a].target) - center(links[b].target) || a - b;
  const bySource = (a: number, b: number): number => center(links[a].source) - center(links[b].source) || a - b;
  const byCenter = (a: number, b: number): number => center(a) - center(b) || a - b;
  const stackOut = (i: number): void => {
    let y = 0;
    sort(outgoing[i], byTarget);
    for (const k of outgoing[i]) {
      sourceOffset[k] = y + links[k].thickness / 2;
      y += links[k].thickness;
    }
  };
  const stackIn = (i: number): void => {
    let y = 0;
    sort(incoming[i], bySource);
    for (const k of incoming[i]) {
      targetOffset[k] = y + links[k].thickness / 2;
      y += links[k].thickness;
    }
  };
  const stackAll = (): void => {
    for (let i = 0; i < n; i++) {
      stackOut(i);
      stackIn(i);
    }
  };

  // Moves each node of a column towards where its bands would run level, then restores the
  // order and the gaps. `forward` looks at the bands coming in, otherwise at those going out.
  const settle = (column: number[], pull: number, forward: boolean): void => {
    for (const i of column) {
      const list = forward ? incoming[i] : outgoing[i];
      if (list.length === 0) continue;
      if (forward) stackIn(i);
      else stackOut(i);
      let weighted = 0;
      let weight = 0;
      let plain = 0;
      for (const k of list) {
        const level = forward
          ? nodes[links[k].source].y + sourceOffset[k] - targetOffset[k]
          : nodes[links[k].target].y + targetOffset[k] - sourceOffset[k];
        weighted += level * links[k].thickness;
        weight += links[k].thickness;
        plain += center(forward ? links[k].source : links[k].target) - nodes[i].height / 2;
      }
      const wanted = weight > 0 ? weighted / weight : plain / list.length;
      nodes[i].y += (wanted - nodes[i].y) * pull;
    }
    sort(column, byCenter);
    let y = 0;
    for (const i of column) {
      if (nodes[i].y - margin[i] < y) nodes[i].y = y + margin[i];
      y = nodes[i].y + nodes[i].height + margin[i] + gap;
    }
    y = options.height;
    for (let k = column.length - 1; k >= 0; k--) {
      const i = column[k];
      if (nodes[i].y + nodes[i].height + margin[i] > y) nodes[i].y = y - nodes[i].height - margin[i];
      y = nodes[i].y - margin[i] - gap;
    }
  };

  stackAll();
  for (let pass = 0; pass < PASSES; pass++) {
    const pull = Math.pow(0.99, pass);
    for (let l = layers - 2; l >= 0; l--) settle(columns[l], pull, false);
    stackAll();
    for (let l = 1; l < layers; l++) settle(columns[l], pull, true);
    stackAll();
  }
  for (let k = 0; k < links.length; k++) {
    links[k].sourceY = nodes[links[k].source].y + sourceOffset[k];
    links[k].targetY = nodes[links[k].target].y + targetOffset[k];
  }

  return { nodes, links };
}
