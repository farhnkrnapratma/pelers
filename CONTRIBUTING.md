# Contributing to pelers

## Setup

You need Rust (stable), [Bun](https://bun.sh) and Node.js 20 or newer. The WebAssembly builds also need the `wasm32-unknown-unknown` target, `wasm-bindgen` 0.2.130 and `wasm-opt`.

```sh
bun install    # JavaScript dependencies, commitlint and the git hooks
```

## Build and test

```sh
cargo test --locked                                      # core, oracle and golden tests
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
cargo deny check
bun run build              # debug native addon: pelers.<platform>.node and index.native.js
bun run build:release      # profile-guided release addon (needs rustup's llvm-tools)
bun run build:wasm         # wasm/fast, wasm/small and dist/*.global.js
bun run test:bindings      # the JavaScript API against the addon, both WebAssembly builds and Pele
bun run test:native        # build:release, then test:bindings
bun run test:dom           # mount and zoom in a simulated DOM
bun run check              # Biome format and lint for the JavaScript sources
```

`build:release` builds an instrumented addon, renders the corpus with it and rebuilds with the collected profile. `bun run build:release --target <triple>` cross-compiles for another platform through `napi build --cross-compile`, which needs [zig](https://ziglang.org), and skips profile-guided optimization because the profile has to be collected on the target itself.

## Pele as the reference

pelers must render exactly what Pele 1.13.0 renders, with `pelers` names. The fixtures and golden SVGs are generated from Pele's source, checked out next to this repository (or wherever `PELE_DIR` points):

```sh
git clone --branch 1.13.0 https://github.com/obsidianmd/pele ../pele
bun scripts/gen-golden.mjs            # tests/golden from ../pele/tests/corpus
bun scripts/gen-fixtures-render.mjs   # and the other scripts/gen-fixtures-*.mjs
```

Regenerate them only when the reference version changes. `scripts/oracle.mjs` is the only place that maps Pele's names to pelers names. Rendering code must keep JavaScript number semantics (for example, `Math.round` rounds ties toward positive infinity), so do not use `f64::round` or hash-map iteration order there.

The benchmarks in `bench/` compare pelers with the official `pele` and `mermaid` npm packages. `bun bench/run.mjs` runs them and rewrites `bench/results.md`.

## Repository layout

```
src/                  the Rust core: parsing, layout and SVG for every diagram type
src/bindings/node.rs  the napi binding (feature `napi`)
src/bindings/wasm.rs  the WebAssembly binding (feature `wasm`)
src/bin/pelers.rs     the command-line renderer
src/tests/            oracle and golden tests, compiled only under cfg(test)
js/                   the JavaScript API shared by the native and WebAssembly builds
index.js, browser.js  package entries for Node.js and browsers
scripts/              build scripts, fixture generators and binding tests
tests/                golden SVGs, fixtures and DOM tests
bench/                benchmarks and differential fuzzing against pele and mermaid
```

## Submitting changes

- Branch from `develop` as `feature/<name>` or `bugfix/<name>`. The repository uses [git-flow-next](https://git-flow.sh/), and `main` only receives releases and hotfixes.
- Write commit messages in the [Conventional Commits](https://www.conventionalcommits.org) format and sign them off with `git commit -s`. The `commit-msg` hook and CI check both.
- Keep `cargo test`, clippy, `cargo fmt --check` and `bun run check` passing, and add a regression test with every fix.
- Do not write comments in code.

CI runs on CircleCI: `reuse lint`, formatting and lint, tests, `cargo deny`, a secret scan and commitlint on every push. On `main` and `release/*` it also builds the native addon for every published platform.

## Reporting security issues

Do not open a public issue for a vulnerability. Report it privately through [GitHub's security advisories](https://github.com/farhnkrnapratma/pelers/security/advisories/new).
