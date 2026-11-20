import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { root } from "./oracle.mjs";

const run = (command, args) => execFileSync(command, args, { cwd: root, stdio: "inherit" });

run("cargo", [
  "build",
  "--lib",
  "--locked",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
  "--features",
  "wasm",
  "--config",
  'profile.release.opt-level="z"',
  "--config",
  'profile.release.panic="abort"',
  "--target-dir",
  "target/wasm",
]);

const out = join(root, "wasm");
rmSync(out, { recursive: true, force: true });
run("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  out,
  "--out-name",
  "pelers",
  "target/wasm/wasm32-unknown-unknown/release/pelers.wasm",
]);

function wasmOpt() {
  const cache = join(homedir(), ".cache/.wasm-pack");
  if (existsSync(cache))
    for (const entry of readdirSync(cache).filter((name) => name.startsWith("wasm-opt-"))) {
      const binary = join(cache, entry, "bin/wasm-opt");
      if (existsSync(binary)) return binary;
    }
  return "wasm-opt";
}

const module = join(out, "pelers_bg.wasm");
try {
  run(wasmOpt(), ["-Oz", "--all-features", "--strip-debug", module, "-o", module]);
} catch {
  console.warn("wasm-opt not found; the module is left unoptimized");
}

const entry = join(out, "global-entry.js");
writeFileSync(
  entry,
  `import * as engine from "./pelers.js";
import * as api from "../js/core.js";
import { bind } from "../js/native.js";
const bytes = Uint8Array.from(atob(${JSON.stringify(readFileSync(module).toString("base64"))}), (c) => c.charCodeAt(0));
engine.initSync({ module: bytes });
bind(engine);
globalThis.pelers = { ...api };
`,
);
mkdirSync(join(root, "dist"), { recursive: true });
const built = await Bun.build({
  entrypoints: [entry],
  format: "iife",
  minify: true,
  target: "browser",
});
if (!built.success) throw new AggregateError(built.logs, "global build failed");
writeFileSync(join(root, "dist/pelers.global.js"), await built.outputs[0].text());
rmSync(entry);
console.log(
  `wasm ${readFileSync(module).length} bytes, global ${readFileSync(join(root, "dist/pelers.global.js")).length} bytes`,
);
