import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const theme = await import(join(root, 'ts/src/svg/theme.ts'));

const POOL = ['fill:#f00', 'stroke:red', 'color:blue', 'font-size:20px', 'font-size:1.2.3px', 'font-size: 300', 'font-weight:bold', 'font-weight:600', 'font-weight:400',
  'font-style:italic', 'stroke-width:2px !important', 'FILL: #abc', 'url(x)', 'fill:javascript:alert', 'bad:val', ':nothing', 'stroke-dasharray: 4 2', 'opacity:0.5',
  'color:<script>', 'rx:4', 'font-family:Arial', 'stroke-opacity:0.3', 'fill-opacity:0.2', '', 'no colon here', 'text-decoration:underline', 'letter-spacing:1px', 'font-size:0px', 'font-size:500px', 'stroke:"quoted"'];
const CLASSES = ['', 'default', 'default node', 'a b c', 'my-class_1', 'x!!y', '  spaced  ', 'emoji🙂class', 'a.b:c'];

let seed = 424242;
const random = () => {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  return seed / 0x7fffffff;
};
const pick = list => list[Math.floor(random() * list.length)];

const styles = [];
for (let i = 0; i < 300 * SCALE; i++) {
  const count = Math.floor(random() * 5);
  const declarations = [];
  for (let k = 0; k < count; k++) declarations.push(pick(POOL));
  styles.push({ declarations, resolved: theme.resolveStyle(declarations) });
}
const classes = CLASSES.map(raw => ({ raw, out: theme.classNames(raw) }));
const fills = ['fill:x', ';fill:y', '"fill:z', 'stroke:1', 'x fill:y', 'fill:', 'a;fill:b'].map(shape => ({ shape, out: theme.setsFill(shape) }));
writeFileSync(join(OUT, 'theme.json'), `${JSON.stringify({ styles, classes, fills })}\n`);
console.log(`theme: ${styles.length} styles, ${classes.length} classes, ${fills.length} fills`);
