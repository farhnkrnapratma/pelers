import { escText, num, type IconResolver } from '../../svg/builder.js';

// A square for a named icon, in the form labels use. The host's resolver supplies the drawing, if any.
export function iconSvg(name: string, x: number, y: number, side: number, icons: IconResolver | undefined): string {
  return `<svg class="pele-icon" data-icon="${escText(name)}" x="${num(x)}" y="${num(y)}" width="${num(side)}" height="${num(
    side
  )}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${
    icons?.(name) ?? ''
  }</svg>`;
}
