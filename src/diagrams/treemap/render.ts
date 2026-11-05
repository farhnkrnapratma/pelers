import type { Config } from '../../preprocess.js';
import { esc, escText, labelSvg, num } from '../../svg/builder.js';
import { inkFill } from '../../svg/ink.js';
import { svgDocument } from '../../svg/root.js';
import { seriesInkFill } from '../../svg/palette.js';
import { classNames, resolveStyle, rounded, seriesColor, setsFill } from '../../svg/theme.js';
import { decodeEntities } from '../../text/entities.js';
import { layoutLabel } from '../../text/label.js';
import { Style, defaultMeasurer } from '../../text/measurer.js';
import type { RenderOptions, Rendered } from '../../types.js';
import { capWidth, titleRoom } from '../common/fit-width.js';
import { fitText } from '../common/fit-text.js';
import { formatter } from './format.js';
import { layoutTreemap, type Box } from './layout.js';
import type { TreemapModel, TreemapNode } from './model.js';

const HEADER = 28;
const SHORT_HEADER = 18;
const SIDE = 6;
const LINE = 1.25;

const titlePart = (raw: string): string => esc(decodeEntities(raw).trim()).trim();

function leafCount(roots: TreemapNode[]): number {
  let count = 0;
  const stack = roots.slice();
  while (stack.length > 0) {
    const node = stack.pop()!;
    if (node.children !== undefined) stack.push(...node.children);
    else if (Number.isFinite(node.value) && node.value! > 0) count++;
  }
  return count;
}

// Caps corners at a quarter of a small box's shorter side; 4px is the default radius.
function corners(attrs: string, b: Box): string {
  const cap = Math.min(b.w, b.h) / 4;
  return cap < 4 ? rounded(attrs).replace('rx:var(--_r)', `rx:min(var(--_r),${num(cap)}px)`) : rounded(attrs);
}

function option(config: Config, key: string, fallback: number, min: number, max: number): number {
  const value = (config.treemap as Config | undefined)?.[key];
  return typeof value === 'number' && value >= min ? Math.min(value, max) : fallback;
}

export function renderTreemap(model: TreemapModel, config: Config, options: RenderOptions): Rendered {
  const size = options.fontSize ?? 16;
  const small = Math.round(size * 0.875);
  const tiny = Math.round(size * 0.75);
  const least = Math.round(size * 0.5625);
  const measurer = options.measurer ?? defaultMeasurer(options.fontFamily, options.fontFamilyMono);
  const treemap = (config.treemap ?? {}) as Config;
  const pad = options.padding ?? option(config, 'diagramPadding', 8, 0, 2000);

  // Mermaid sizes the diagram as ten times `nodeWidth` by ten times `nodeHeight`.
  const width = Math.round(capWidth(options, option(config, 'nodeWidth', 64, 1, 400) * 10, 200, 2 * pad));
  const height = Math.round(option(config, 'nodeHeight', 40, 1, 400) * 10);
  const showValues = treemap.showValues !== false;
  const format = formatter(typeof treemap.valueFormat === 'string' ? treemap.valueFormat : ',');
  // Reduce gaps and insets as average leaf area shrinks.
  const crowd = Math.min(1, Math.max(0, Math.log10((width * height) / Math.max(1, leafCount(model.roots)) / 800)));
  const boxes = layoutTreemap(model.roots, width, height, {
    gap: option(config, 'padding', Math.round(2 + 2 * crowd), 0, 200),
    header: HEADER,
    short: SHORT_HEADER,
    side: Math.round(3 + 3 * crowd),
  });

  const title = layoutLabel(model.title, false, measurer, size, titleRoom(options), Style.Bold);
  const titleHeight = title.height > 0 ? title.height + 8 : 0;
  const left = pad + Math.max(0, (title.width - width) / 2);
  const top = pad + titleHeight;

  let sections = '';
  let leaves = '';
  for (const b of boxes) {
    const node = b.node;
    const style = resolveStyle(node.cssCompiledStyles ?? []);
    const color = seriesColor(b.series);
    const x = left + b.x;
    const y = top + b.y;
    const name = decodeEntities(node.name).trim();
    const value = showValues ? format(b.value) : '';
    const group = `data-id="${escText(node.name)}">`;
    // Include headerless ancestors in tooltips because their children cover them.
    const own = titlePart(node.name);
    const path = own === '' ? [] : [own];
    for (let p = b.parent; p !== undefined && !p.headed; p = p.parent) {
      const section = titlePart(p.node.name);
      if (section !== '') path.unshift(section);
    }
    const full = path.join(' › ');
    const tooltip = (drawn: string | undefined): string =>
      full === '' || (drawn === name && full === own) ? '' : `<title>${full}</title>`;
    const rect = `<rect x="${num(x)}" y="${num(y)}" width="${num(b.w)}" height="${num(b.h)}" fill="${color}"`;
    // Inline font sizes override SVG attributes, so truncate instead of shrinking.
    const fixed = style.fontSize;
    const textStyle = (style.bold ? Style.Bold : 0) | (style.italic ? Style.Italic : 0);
    const ink = setsFill(style.shape) ? inkFill(style) : seriesInkFill(b.series);

    if (node.children !== undefined) {
      let text = '';
      let drawn: string | undefined;
      if (b.headed) {
        const short = b.header < HEADER;
        const labelSize = fixed ?? (short ? tiny : small);
        const valueSize = fixed ?? tiny;
        const room = b.w - 2 * SIDE - 4;
        const valueWidth = value ? measurer.width(value, valueSize, textStyle) : 0;
        const withValue = !short && valueWidth > 0 && room - valueWidth - 12 >= 3 * labelSize;
        const label =
          labelSize * LINE < b.header
            ? fitText(
                name,
                withValue ? room - valueWidth - 12 : room,
                labelSize,
                labelSize,
                measurer,
                textStyle | Style.Bold
              )
            : undefined;
        const baseline = y + b.header / 2 + 1;
        drawn = label?.text;
        if (label) {
          text += `<text x="${num(x + SIDE + 2)}" y="${num(baseline + labelSize * 0.35)}" font-weight="var(--_hw)"${short ? ` font-size="${labelSize}"` : ''}${ink}${style.text}>${esc(label.text)}</text>`;
        }
        if (withValue && valueSize * LINE <= HEADER) {
          text += `<text class="pele-value" x="${num(x + b.w - SIDE - 2)}" y="${num(baseline + valueSize * 0.35)}" font-size="${tiny}" text-anchor="end"${ink || ' fill="var(--_m)"'}${style.text}>${esc(value)}</text>`;
        }
      }
      // A section is a tint of its color, unless a class gives it a fill of its own. Headerless
      // sections draw no box, so their children blend into the grid.
      const tint = /[";]fill:/.test(style.shape) ? '' : ' fill-opacity="0.12"';
      const shape = b.headed
        ? `${rect}${corners(`${tint} stroke="${color}"${style.shape}`, b)}/>`
        : `<rect x="${num(x)}" y="${num(y)}" width="${num(b.w)}" height="${num(b.h)}" fill="none"/>`;
      sections += `<g class="pele-node pele-section${classNames(node.classSelector ?? '')}" ${group}${tooltip(drawn)}${shape}${text}</g>`;
      continue;
    }

    let text = '';
    const room = b.w - 8;
    const smallest = fixed ?? least;
    const tallest =
      fixed === undefined ? Math.min(size, Math.floor((b.h - 4) / LINE)) : fixed * LINE <= b.h - 4 ? fixed : 0;
    const label = tallest >= smallest ? fitText(name, room, tallest, smallest, measurer, textStyle) : undefined;
    if (label) {
      const cx = num(x + b.w / 2);
      const cy = y + b.h / 2;
      const valueSize = fixed ?? Math.max(least, Math.round(label.size * 0.8));
      const lines = (label.size + valueSize) * LINE;
      if (value && lines <= b.h - 4 && measurer.width(value, valueSize, textStyle) <= room) {
        const first = cy - lines / 2 + (label.size * LINE) / 2;
        const second = cy + lines / 2 - (valueSize * LINE) / 2;
        text =
          `<text x="${cx}" y="${num(first + label.size * 0.35)}" font-size="${num(label.size)}"${ink}${style.text}>${esc(label.text)}</text>` +
          `<text class="pele-value" x="${cx}" y="${num(second + valueSize * 0.35)}" font-size="${num(valueSize)}"${ink}${style.text}>${esc(value)}</text>`;
      } else {
        text = `<text x="${cx}" y="${num(cy + label.size * 0.35)}" font-size="${num(label.size)}"${ink}${style.text}>${esc(label.text)}</text>`;
      }
    }
    leaves += `<g class="pele-node pele-leaf${classNames(node.classSelector ?? '')}" ${group}${tooltip(label?.text)}${rect}${corners(style.shape, b)}/>${text}</g>`;
  }

  const totalWidth = Math.ceil(Math.max(width, title.width) + 2 * pad);
  const totalHeight = Math.ceil(height + titleHeight + 2 * pad);
  const content =
    labelSvg(title, totalWidth / 2, pad + title.height / 2, ' class="pele-title" font-weight="var(--_tw)"') +
    (sections ? `<g class="pele-sections" font-size="${small}">${sections}</g>` : '') +
    (leaves ? `<g class="pele-leaves" text-anchor="middle" fill="var(--_bg)">${leaves}</g>` : '');

  return {
    svg: svgDocument('treemap', totalWidth, totalHeight, size, options, model, content),
    width: totalWidth,
    height: totalHeight,
    links: [],
  };
}
