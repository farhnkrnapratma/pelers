import { peleCore, peleDir, writeFixture } from "./oracle.mjs";
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

process.env.TZ = "UTC";

const root = fileURLToPath(new URL("../", import.meta.url));
const corpus = join(peleDir, "tests/corpus");
const out = join(root, "tests/golden");
const { render } = await peleCore();

mkdirSync(out, { recursive: true });
for (const entry of readdirSync(out, { withFileTypes: true })) {
  if (entry.isDirectory()) rmSync(join(out, entry.name), { recursive: true });
}
let svgs = 0;
let errors = 0;

for (const file of readdirSync(corpus)
  .filter((name) => name.endsWith("-docs.json"))
  .sort()) {
  const type = file.replace(/-docs\.json$/, "");
  const sources = JSON.parse(readFileSync(join(corpus, file), "utf8"));
  const dir = join(out, type);
  mkdirSync(dir, { recursive: true });
  sources.forEach((original, index) => {
    const source =
      type === "gantt" &&
      original.includes("dateFormat HH:mm") &&
      !original.includes("todayMarker")
        ? original.replace(/(axisFormat[^\n]*\n)/, "$1    todayMarker off\n")
        : original;
    const name = String(index).padStart(3, "0");
    writeFileSync(join(dir, `${name}.mmd`), source);
    try {
      writeFixture(join(dir, `${name}.svg`), render(source).svg);
      svgs += 1;
    } catch (error) {
      writeFixture(
        join(dir, `${name}.error.json`),
        `${JSON.stringify({ code: error.code, message: error.message }, null, 2)}\n`,
      );
      errors += 1;
    }
  });
}
console.log(`golden: ${svgs} svg, ${errors} error cases`);
