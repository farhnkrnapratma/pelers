import { num } from '../../svg/builder.js';

// How far a marker reaches back along its line from the box it touches.
export const MARKER_LENGTH = 18;

const BAR = 5;
const FOOT = 6;
const FOOT_LENGTH = 10;
const RING = 3.5;

// Draws a crow's foot cardinality marker over the end of a line that meets a box at (x, y)
// while travelling along the unit vector (dx, dy). Types are Mermaid's lower-cased cardinalities.
export function erMarker(type: string, x: number, y: number, dx: number, dy: number): string {
  const at = (back: number, side: number): string =>
    num(x - dx * back - dy * side) + ',' + num(y - dy * back + dx * side);
  const bar = (back: number): string => `M${at(back, -BAR)}L${at(back, BAR)}`;
  const foot = `M${at(0, -FOOT)}L${at(FOOT_LENGTH, 0)}L${at(0, FOOT)}`;
  const ring = (back: number): string =>
    `<circle class="pele-marker" cx="${num(x - dx * back)}" cy="${num(y - dy * back)}" r="${RING}" fill="var(--_bg)"/>`;
  const path = (d: string): string => `<path class="pele-marker" d="${d}"/>`;

  switch (type) {
    case 'only_one':
      return path(bar(6) + bar(10));
    case 'zero_or_one':
      return path(bar(6)) + ring(13.5);
    case 'one_or_more':
      return path(foot + bar(FOOT_LENGTH + 3.5));
    case 'zero_or_more':
      return path(foot) + ring(FOOT_LENGTH + RING + 0.5);
    case 'md_parent':
      return `<path class="pele-marker" d="M${at(0, 0)}L${at(6, 4)}L${at(12, 0)}L${at(6, -4)}Z" fill="var(--_bg)"/>`;
  }
  return '';
}
