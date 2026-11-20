import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { peleDir } from "../scripts/oracle.mjs";

const root = join(peleDir, "tests");
const starts =
  /^\s*(---|%%|flowchart|graph|sequenceDiagram|classDiagram|stateDiagram|erDiagram|journey|gantt|pie|mindmap|timeline|quadrantChart|requirementDiagram|gitGraph|C4|block|packet|sankey|xychart|architecture|kanban|ishikawa|treemap|treeView|usecase|agentflow|eventmodeling|cynefin|wardley|swimlane|railroad|venn|radar|info|quadrant|requirement)/;
const literal = /`([^`\\]*)`|"((?:[^"\\\n]|\\.)*)"|'((?:[^'\\\n]|\\.)*)'/g;
const found = new Map();

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      walk(path);
      continue;
    }
    if (!/\.(ts|mmd|md|json)$/.test(name)) continue;
    const text = readFileSync(path, "utf8");
    for (const match of text.matchAll(literal)) {
      const source = (match[1] ?? match[2] ?? match[3] ?? "").trim();
      if (
        !source.includes("\n") ||
        source.includes("${") ||
        !starts.test(source)
      )
        continue;
      if (!found.has(source)) found.set(source, path.slice(root.length + 1));
    }
  }
}

walk(root);
const seeds = [...found].map(([source, file]) => ({ source, file }));
writeFileSync(
  join(process.cwd(), "upstream-seeds.json"),
  JSON.stringify(seeds, null, 1),
);
console.log(`seeds ${seeds.length}`);
