import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { root } from "./oracle.mjs";

const run = (command, args) => execFileSync(command, args, { cwd: root, stdio: "inherit" });

const variants = [
  { name: "fast", level: "3", flag: "-O3", features: "wasm,regex-perf", global: "pelers.global.js" },
  { name: "small", level: '"z"', flag: "-Oz", features: "wasm", global: "pelers.small.global.js" },
];

function wasmOpt() {
  const cache = join(homedir(), ".cache/.wasm-pack");
  if (existsSync(cache))
    for (const entry of readdirSync(cache).filter((name) => name.startsWith("wasm-opt-"))) {
      const binary = join(cache, entry, "bin/wasm-opt");
      if (existsSync(binary)) return binary;
    }
  return "wasm-opt";
}

rmSync(join(root, "wasm"), { recursive: true, force: true });
mkdirSync(join(root, "dist"), { recursive: true });
for (const { name, level, flag, features, global } of variants) {
  const target = `target/wasm-${name}`;
  run("cargo", [
    "build",
    "--lib",
    "--locked",
    "--release",
    "--target",
    "wasm32-unknown-unknown",
    "--features",
    features,
    "--config",
    `profile.release.opt-level=${level}`,
    "--config",
    'profile.release.panic="abort"',
    "--target-dir",
    target,
  ]);
  const out = join(root, "wasm", name);
  run("wasm-bindgen", [
    "--target",
    "web",
    "--out-dir",
    out,
    "--out-name",
    "pelers",
    `${target}/wasm32-unknown-unknown/release/pelers.wasm`,
  ]);
  const module = join(out, "pelers_bg.wasm");
  try {
    run(wasmOpt(), [flag, "--all-features", "--strip-debug", module, "-o", module]);
  } catch (error) {
    if (process.env.CI) throw error;
    console.warn("wasm-opt not found; the module is left unoptimized");
  }
  const entry = join(out, "global-entry.js");
  writeFileSync(
    entry,
    `import * as engine from "./pelers.js";
import * as api from "../../js/core.js";
import { bind } from "../../js/native.js";
const bytes = Uint8Array.from(atob(${JSON.stringify(readFileSync(module).toString("base64"))}), (c) => c.charCodeAt(0));
engine.initSync({ module: bytes });
bind(engine);
globalThis.pelers = { ...api };
`,
  );
  const built = await Bun.build({
    entrypoints: [entry],
    format: "iife",
    minify: true,
    target: "browser",
  });
  if (!built.success) throw new AggregateError(built.logs, "global build failed");
  writeFileSync(join(root, "dist", global), await built.outputs[0].text());
  rmSync(entry);
  console.log(
    `${name}: wasm ${readFileSync(module).length} bytes, global ${readFileSync(join(root, "dist", global)).length} bytes`,
  );
}
