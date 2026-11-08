// A step of 1, 2 or 5 times a power of ten that divides `span` into about `count` parts.
// A step below one comes back as the negative of its inverse, which is a whole number, so that
// tick values are found by division and print without rounding noise (0.3, not 0.30000000000000004).
export function tickStep(span: number, count: number): number {
  const raw = span / Math.max(count, 1);
  const power = Math.floor(Math.log10(raw));
  const unit = raw / 10 ** power;
  const factor = unit >= 7.07 ? 10 : unit >= 3.16 ? 5 : unit >= 1.41 ? 2 : 1;
  return power < 0 ? -(10 ** -power) / factor : factor * 10 ** power;
}

export function tickIndex(step: number, value: number): number {
  return step > 0 ? value / step : value * -step;
}

export function tickValue(step: number, index: number): number {
  const value = step > 0 ? index * step : index / -step;
  return Number.isFinite(value) ? Number(value.toPrecision(15)) : value;
}

// The multiples of a step from `lo` to `hi`, both included.
export function ticksBetween(lo: number, hi: number, step: number): number[] {
  let first = Math.round(tickIndex(step, lo));
  if (tickValue(step, first) < lo) first++;
  let last = Math.round(tickIndex(step, hi));
  if (tickValue(step, last) > hi) last--;
  const out: number[] = [];
  for (let i = first; i <= last && out.length < 100; i++) out.push(tickValue(step, i));
  return out;
}
