import { num } from '../../svg/builder.js';
import { marker, markerTrim } from '../../svg/edges.js';
import type { EndMarker } from './graph.js';

const TRIANGLE = 12;
const DIAMOND = 16;
const HALF = 6;
const RING = 5;

// How far the line stops short of the node so that it meets the back of the marker.
export function classMarkerTrim(type: EndMarker): number {
  switch (type) {
    case 'extension':
      return TRIANGLE;
    case 'composition':
    case 'aggregation':
      return DIAMOND;
    case 'lollipop':
      return 2 * RING;
    case 'dependency':
      return markerTrim('arrow_point');
  }
  return 0;
}

// Draws an end marker whose tip is at (x, y), pointing along (dx, dy).
export function classMarker(type: EndMarker, x: number, y: number, dx: number, dy: number, color: string): string {
  const nx = -dy;
  const ny = dx;
  switch (type) {
    case 'extension': {
      const bx = x - dx * TRIANGLE;
      const by = y - dy * TRIANGLE;
      return `<path class="pele-marker" d="M${num(x)},${num(y)}L${num(bx + nx * HALF)},${num(by + ny * HALF)}L${num(
        bx - nx * HALF
      )},${num(by - ny * HALF)}Z" stroke-linejoin="round"/>`;
    }
    case 'composition':
    case 'aggregation': {
      const mx = x - (dx * DIAMOND) / 2;
      const my = y - (dy * DIAMOND) / 2;
      const half = HALF - 1;
      return `<path class="pele-marker" d="M${num(x)},${num(y)}L${num(mx + nx * half)},${num(my + ny * half)}L${num(
        x - dx * DIAMOND
      )},${num(y - dy * DIAMOND)}L${num(mx - nx * half)},${num(my - ny * half)}Z" stroke-linejoin="round"${
        type === 'composition' ? ` fill="${color}"` : ''
      }/>`;
    }
    case 'lollipop':
      return `<circle class="pele-marker" cx="${num(x - dx * RING)}" cy="${num(y - dy * RING)}" r="${RING}"/>`;
    case 'dependency':
      return marker('arrow_point', x, y, dx, dy, color);
  }
  return '';
}
