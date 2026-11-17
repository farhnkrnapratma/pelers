import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const report = JSON.parse(
  execFileSync("cargo", ["llvm-cov", "--locked", "--json", "--summary-only"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1 << 28,
    stdio: ["ignore", "pipe", "inherit"],
  }),
);
const lines = report.data[0].totals.lines.percent;
const functions = report.data[0].totals.functions.percent;
const percent = Math.floor(lines * 100) / 100;
const color =
  percent >= 100
    ? "brightgreen"
    : percent >= 99.999
      ? "green"
      : percent >= 90
        ? "yellowgreen"
        : percent >= 80
          ? "yellow"
          : "red";
const badge = {
  schemaVersion: 1,
  label: "coverage",
  message: `${percent.toFixed(2)}%`,
  color,
  functions: `${functions.toFixed(2)}%`,
};
const dir = join(root, ".github", "badges");
mkdirSync(dir, { recursive: true });
writeFileSync(
  join(dir, "coverage.json"),
  `${JSON.stringify(badge, null, 2)}\n`,
);
console.log(
  `coverage lines ${percent.toFixed(2)}% functions ${functions.toFixed(2)}%`,
);
