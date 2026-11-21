# pelers

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/stZqvYiNAK17m5zy7ERCb/3MyzjfBswYtP68e4e1e9hk/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/stZqvYiNAK17m5zy7ERCb/3MyzjfBswYtP68e4e1e9hk/tree/main) [![npm](https://img.shields.io/npm/v/pelers?label=npm)](https://www.npmjs.com/package/pelers) [![crates.io](https://img.shields.io/crates/v/pelers?label=crates.io)](https://crates.io/crates/pelers) ![Language](https://img.shields.io/badge/language-Rust-orange?logo=rust) ![Tests](https://img.shields.io/badge/tests-cargo%20test-orange?logo=rust) ![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) [![Coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/farhnkrnapratma/pelers/develop/.github/badges/coverage.json)](https://github.com/farhnkrnapratma/pelers/actions)

A fast, lightweight Mermaid renderer that outputs themeable SVG. `pelers` is a Rust rewrite of [Pele](https://github.com/obsidianmd/pele), a Mermaid renderer by [kepano](https://github.com/kepano) (Obsidian) released under the MIT License. See [NOTICE](NOTICE) for credits and the original license text. It is being ported from TypeScript to Rust, and will expose only a JavaScript/TypeScript binding (native addon for Node, Bun and Deno, WebAssembly for browsers), like [`markers`](https://github.com/farhnkrnapratma/markers) and [`mdrs`](https://github.com/farhnkrnapratma/mdrs).

## Status

Alpha. All 33 diagram types of Pele 1.13.0 are ported and match Pele's output byte for byte on its test corpus (with `pelers` names), and a differential fuzz run against the official `pele` package is clean.

| Registry  | Name                                             | State                                                                                                   |
| --------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| npm       | [`pelers`](https://www.npmjs.com/package/pelers) | `1.0.0-alpha1` is the old TypeScript build. The next release is the Rust binding with Pele's full API. |
| crates.io | [`pelers`](https://crates.io/crates/pelers)      | The Rust core and a `pelers` command-line renderer.                                                    |

The npm API follows Pele's: `render(source | model, options)` returns `{ svg, width, height, links, type, mermaidType, title? }`, and `parse`, `detectType`, `supports`, `register`, `registered`, `forgetTextWidths` and `PelersError` (with `code`, `type`, `mermaidType`, `line`, `column` and `snippet`) are exported. Every `RenderOptions` field is accepted, including the `icons`, `math` and `measurer` callbacks. CSS classes and variables use the `pelers-` prefix (`--pelers-bg` and so on).

## Install

```sh
bun add pelers          # JavaScript / TypeScript (or: npm install pelers)
cargo add pelers        # Rust
```

## Usage

JavaScript / TypeScript (Rust-backed version):

```ts
import { parse, render } from "pelers";

const { svg, width, height, links } = render("flowchart LR\n  A --> B", {
  fontSize: 14,
  maxWidth: 640,
});
const model = parse("pie\n  \"Dogs\": 3\n  \"Cats\": 1");
const again = render(model, { idPrefix: "pets" });
```

In browsers `pelers` resolves to a WebAssembly build. Two builds ship with the package:

| Import                                        | Build                         | Module size (gzip) |
| --------------------------------------------- | ----------------------------- | ------------------ |
| `pelers`, `pelers/lazy`, `pelers/global`      | fast (`opt-level 3`)          | 1.5 MB             |
| `pelers/small`, `pelers/small/lazy`, `pelers/small/global` | small (`opt-level z`) | 0.96 MB            |

Both render the same SVG; the fast build renders faster, the small one downloads less. `pelers/lazy` and `pelers/small/lazy` load the module on first use (`await load()`, `renderAsync`, `mountAsync`), and the `global` builds define `globalThis.pelers` for a plain `<script>` tag.

Rust:

```rust
use pelers::{ParseOptions, RenderOptions, parse_with, render_with};

let model = parse_with("flowchart LR\n  A --> B", &ParseOptions::default())?;
let rendered = render_with(&model, &RenderOptions::default())?;
println!("{}", rendered.svg);
```

## Repository layout

```
src/*.rs                 the Rust core: api, detect, preprocess, flowchart, render, errors
src/bindings/node.rs     the napi binding (feature `napi`)
src/bindings/wasm.rs     the WebAssembly binding (feature `wasm`)
index.js, index.d.ts     Node entry over the generated index.native.js / index.native.d.ts
browser.js, small.js     browser entries over wasm/fast and wasm/small
js/                      the shared JavaScript API (render, parse, mount, zoom, lazy loading)
scripts/                 binding checks
scripts/oracle.mjs       loads Pele's source from ../pele (or PELE_DIR) as the oracle for fixtures and goldens
bench/                   benchmarks and differential fuzzing against the npm pele and mermaid releases
```

## Development

Requirements: Rust (stable), [Bun](https://bun.sh) and Node.js 20 or newer.

```sh
bun install                # JS dependencies and git hooks
bun run build              # napi build (debug) -> pelers.<platform>.node, index.native.js
bun run build:release      # profile-guided release addon (needs node and rustup's llvm-tools)
bun run build:wasm         # wasm/fast, wasm/small and dist/*.global.js (needs wasm-bindgen 0.2.130, wasm-opt)
bun run test:native        # release build + scripts/test-bindings.mjs
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
cargo deny check
```

The oracle is Pele's own source. Check it out next to this repository (or point `PELE_DIR` at it) before regenerating fixtures:

```sh
git clone --branch 1.13.0 https://github.com/obsidianmd/pele ../pele
bun scripts/gen-golden.mjs          # tests/golden from ../pele/tests/corpus
bun scripts/gen-fixtures-render.mjs # and the other scripts/gen-fixtures-*.mjs
```

`build:release` builds an instrumented addon, renders the corpus with it and rebuilds with the collected profile. Without node or `llvm-profdata` it falls back to a plain release build, except under CI, where the job fails instead so the tested and stored builds are always the profile-guided ones.

Releases are cut with git-flow (`git flow release start vX.Y.Z`), so the tag carries the `v`. The npm package is published with `--tag alpha` while the version is a prerelease, and the crate with `cargo publish`.

## Upstream and attribution

Pele is © kepano and distributed under the MIT License. pelers ports that work to Rust, together with material from Mermaid and Lucide. Their notices are reproduced in [`NOTICE`](NOTICE) and must stay with every copy of this code.

## Contributing

This repository uses [git-flow-next](https://git-flow.sh/) (classic preset): `main` is release-only, work happens on `feature/*` and `bugfix/*` branches off `develop`. Commits are signed off (`git commit -s`) and follow [Conventional Commits](https://www.conventionalcommits.org), enforced by commitlint in a `commit-msg` hook and in CI. Install the hooks with `bash .githooks/install.sh` (`bun install` does it through the `prepare` script). CI runs on CircleCI: `reuse lint`, format and lint, tests, `cargo deny`, a TruffleHog secret scan and commitlint on every push. The branch rules are written out in [`CLAUDE.md`](CLAUDE.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

Copyright © 2026 Farhan Kurnia Pratama. Portions are derived from Pelers (© kepano, MIT); see [NOTICE](NOTICE).
