import { readFileSync, writeFileSync } from 'node:fs';

const src = readFileSync(process.argv[2], 'utf8');
const start = src.indexOf('[\\u00AA');
const end = src.indexOf("return 'UNICODE_TEXT'");
const block = src.slice(start, end);
const ranges = [];
const re = /\\u([0-9A-F]{4})(?:-\\u([0-9A-F]{4}))?/g;
let m;
while ((m = re.exec(block))) {
  const a = parseInt(m[1], 16);
  ranges.push([a, m[2] ? parseInt(m[2], 16) : a]);
}
ranges.sort((x, y) => x[0] - y[0]);
const merged = [];
for (const r of ranges) {
  const last = merged[merged.length - 1];
  if (last && r[0] <= last[1] + 1) last[1] = Math.max(last[1], r[1]);
  else merged.push([...r]);
}
let prev = 0;
const parts = [];
for (const [a, b] of merged) {
  parts.push((a - prev).toString(36), (b - a).toString(36));
  prev = b;
}
const packed = parts.join(' ');
writeFileSync(
  process.argv[3],
  `const PACKED =\n  '${packed}';\n\n` +
    `let table: Uint16Array | undefined;\n\n` +
    `function build(): Uint16Array {\n` +
    `  const parts = PACKED.split(' ');\n` +
    `  const out = new Uint16Array(parts.length);\n` +
    `  let prev = 0;\n` +
    `  for (let i = 0; i < parts.length; i += 2) {\n` +
    `    const start = prev + parseInt(parts[i], 36);\n` +
    `    prev = start + parseInt(parts[i + 1], 36);\n` +
    `    out[i] = start;\n` +
    `    out[i + 1] = prev;\n` +
    `  }\n` +
    `  return out;\n` +
    `}\n\n` +
    `export function isUnicodeLetter(code: number): boolean {\n` +
    `  if (code < 0xaa || code > 0xffdc) return false;\n` +
    `  const t = (table ??= build());\n` +
    `  let lo = 0;\n` +
    `  let hi = (t.length >> 1) - 1;\n` +
    `  while (lo <= hi) {\n` +
    `    const mid = (lo + hi) >> 1;\n` +
    `    if (code < t[mid * 2]) hi = mid - 1;\n` +
    `    else if (code > t[mid * 2 + 1]) lo = mid + 1;\n` +
    `    else return true;\n` +
    `  }\n` +
    `  return false;\n` +
    `}\n`
);
console.log(merged.length, 'ranges,', packed.length, 'chars');
