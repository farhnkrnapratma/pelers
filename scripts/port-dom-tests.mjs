import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { peleDir, root, toPelers } from "./oracle.mjs";

const out = join(root, "tests/dom");
mkdirSync(out, { recursive: true });
for (const name of ["mount.test.ts", "zoom.test.ts"]) {
  const text = readFileSync(join(peleDir, "tests", name), "utf8")
    .replace("from 'vitest'", "from 'bun:test'")
    .replace("import { setFlagsFromString } from 'node:v8';\n", "")
    .replace("import { runInNewContext } from 'node:vm';\n", "")
    .replace("setFlagsFromString('--expose-gc');\n", "")
    .replace(
      "const collect = runInNewContext('gc') as () => void;",
      "const collect = (): void => Bun.gc(true);",
    )
    .replace("from '../src/index.js'", "from '../../index.js'")
    .replace("from '../src/mount.js'", "from '../../js/mount.js'")
    .replace("from '../src/zoom.js'", "from '../../js/zoom.js'")
    .replace(
      "import { metricsMeasurer } from '../src/text/measurer.js';",
      "const metricsMeasurer = undefined;",
    );
  writeFileSync(join(out, name), toPelers(text));
}
