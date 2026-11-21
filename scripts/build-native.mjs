import { execFileSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { root } from "./oracle.mjs";

const napi = [
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
];
const run = (command, args, env = {}) =>
  execFileSync(command, args, { cwd: root, stdio: "inherit", env: { ...process.env, ...env } });

function profdata() {
  try {
    const sysroot = execFileSync("rustc", ["--print", "sysroot"], { encoding: "utf8" }).trim();
    const host = /^host: (.+)$/m.exec(execFileSync("rustc", ["-vV"], { encoding: "utf8" }))[1];
    const tool = join(sysroot, "lib/rustlib", host, "bin/llvm-profdata");
    return existsSync(tool) ? tool : undefined;
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

const tool = process.env.PELERS_PGO === "0" || !node() ? undefined : profdata();
if (tool === undefined) {
  console.warn("node or llvm-profdata not found; building without profile-guided optimization");
  run("bunx", napi);
} else {
  const dir = join(root, "target/pgo");
  rmSync(dir, { recursive: true, force: true });
  run("bunx", [...napi, "--output-dir", join(dir, "gen"), "--target-dir", "target/pgo-gen"], {
    RUSTFLAGS: `-Cprofile-generate=${join(dir, "raw")} -Cllvm-args=-vp-counters-per-site=8`,
  });
  run("node", ["scripts/pgo-train.mjs", join(dir, "gen/index.native.js")]);
  run(tool, ["merge", "-o", join(dir, "merged.profdata"), join(dir, "raw")]);
  run("bunx", napi, {
    RUSTFLAGS: `-Cprofile-use=${join(dir, "merged.profdata")}`,
  });
}
