import { execFileSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { root } from "./oracle.mjs";

const rustc = (...args) => execFileSync("rustc", args, { encoding: "utf8" });
const host = /^host: (.+)$/m.exec(rustc("-vV"))[1];
const at = process.argv.indexOf("--target");
const target = at > 0 ? process.argv[at + 1] : host;
const os = (triple) => /apple|windows|freebsd|linux/.exec(triple)?.[0];
const crossFlags = () =>
  os(target) === os(host) && os(host) !== "linux" ? [] : ["--cross-compile"];

const napi = [
  "x",
  "napi",
  "build",
  "--platform",
  "--release",
  "--features",
  "napi",
  "--esm",
  "--js",
  "index.native.js",
  "--dts",
  "index.native.d.ts",
  ...(target === host ? [] : ["--target", target, ...crossFlags()]),
];
const run = (command, args, flags) =>
  execFileSync(command, args, {
    cwd: root,
    stdio: "inherit",
    env: flags
      ? { ...process.env, RUSTFLAGS: `${process.env.RUSTFLAGS ?? ""} ${flags}`.trim() }
      : process.env,
  });
const bun = (args, flags) => run(process.execPath, args, flags);

function profdata() {
  try {
    const tool = join(rustc("--print", "sysroot").trim(), "lib/rustlib", host, "bin/llvm-profdata");
    return existsSync(tool) || existsSync(`${tool}.exe`) ? tool : undefined;
  } catch {
    return undefined;
  }
}

function node() {
  try {
    execFileSync("node", ["--version"]);
    return true;
  } catch {
    return false;
  }
}

const native = target === host;
const tool = !native || process.env.PELERS_PGO === "0" || !node() ? undefined : profdata();
if (tool === undefined) {
  if (native && process.env.CI && process.env.PELERS_PGO !== "0")
    throw new Error("CI builds need node and llvm-profdata (rustup component add llvm-tools)");
  if (native) console.warn("node or llvm-profdata not found; building without profile-guided optimization");
  else console.warn(`cross-compiling ${target} on ${host} without profile-guided optimization`);
  bun(napi);
} else {
  const dir = join(root, "target/pgo", target);
  rmSync(dir, { recursive: true, force: true });
  bun(
    [...napi, "--output-dir", join(dir, "gen"), "--target-dir", "target/pgo-gen"],
    `-Cprofile-generate=${join(dir, "raw")} -Cllvm-args=-vp-counters-per-site=8`,
  );
  run("node", ["scripts/pgo-train.mjs", join(dir, "gen/index.native.js")]);
  run(tool, ["merge", "-o", join(dir, "merged.profdata"), join(dir, "raw")]);
  bun(napi, `-Cprofile-use=${join(dir, "merged.profdata")}`);
}
