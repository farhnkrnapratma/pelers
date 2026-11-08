import type { TextMeasurer } from '../../text/measurer.js';

export interface Fitted {
  text: string;
  size: number;
  width: number;
}

// Fits one line of text into a width: first by shrinking the font down to `min`, then by cutting
// the text and adding an ellipsis. Returns undefined when not even one character fits.
export function fitText(
  text: string,
  max: number,
  size: number,
  min: number,
  measurer: TextMeasurer,
  style = 0
): Fitted | undefined {
  if (text === '' || max <= 0) return undefined;
  let width = measurer.width(text, size, style);
  if (width <= max) return { text, size, width };

  // Width grows with font size, so one division lands on the right size or just above it.
  let fitted = Math.max(min, Math.floor((size * max) / width));
  while (fitted >= min) {
    width = measurer.width(text, fitted, style);
    if (width <= max) return { text, size: fitted, width };
    fitted--;
  }

  const chars = Array.from(text);
  let low = 0;
  let high = chars.length - 1;
  while (low < high) {
    const mid = (low + high + 1) >> 1;
    if (measurer.width(chars.slice(0, mid).join('').trimEnd() + '…', min, style) <= max) low = mid;
    else high = mid - 1;
  }
  if (low === 0) return undefined;
  const cut = chars.slice(0, low).join('').trimEnd() + '…';
  width = measurer.width(cut, min, style);
  return width <= max ? { text: cut, size: min, width } : undefined;
}
