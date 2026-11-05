import { layoutLabel, type Label } from '../../text/label.js';
import type { TextMeasurer } from '../../text/measurer.js';

const RE_TAG_START = /<(?!br\s*\/?>)/gi;
const RE_ICON = /:fa-/g;
// A line break, an encoded entity, a surrogate pair, or one character: the pieces a word can be cut between.
const RE_UNIT = /<br\s*\/?>|ﬂ°°?\w+¶ß|&#?\w+;|[\ud800-\udbff][\udc00-\udfff]|[\s\S]/gi;

// Lays out text the way Mermaid's journey and timeline draw it: as plain text in which only
// <br> is markup, so other tags and icon names show as written instead of being read as formatting.
export function plainLabel(
  raw: string | undefined,
  measurer: TextMeasurer,
  size: number,
  max: number,
  base = 0
): Label {
  let text = raw;
  if (text !== undefined) {
    if (text.includes('<')) text = text.replace(RE_TAG_START, '&lt;');
    if (text.includes(':fa-')) text = text.replace(RE_ICON, '&colon;fa-');
  }
  return layoutLabel(text, false, measurer, size, max, base);
}

// layoutLabel wraps at spaces only. For a box of fixed width, this also cuts any word too long to fit.
export function fitLabel(raw: string | undefined, measurer: TextMeasurer, size: number, max: number, base = 0): Label {
  const label = plainLabel(raw, measurer, size, max, base);
  if (raw === undefined || label.width <= max) return label;
  let out = '';
  let width = 0;
  for (const unit of raw.match(RE_UNIT) ?? []) {
    if (unit.trim() === '' || (unit.length > 2 && unit[0] === '<')) {
      width = 0;
    } else {
      const w = measurer.width(unit.length > 2 ? 'x' : unit, size, base);
      if (width > 0 && width + w > max) {
        out += '\n';
        width = 0;
      }
      width += w;
    }
    out += unit;
  }
  return plainLabel(out, measurer, size, max, base);
}
