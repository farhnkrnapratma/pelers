# pelers

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/stZqvYiNAK17m5zy7ERCb/3MyzjfBswYtP68e4e1e9hk/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/stZqvYiNAK17m5zy7ERCb/3MyzjfBswYtP68e4e1e9hk/tree/main) [![npm](https://img.shields.io/npm/v/pelers?label=npm)](https://www.npmjs.com/package/pelers) [![crates.io](https://img.shields.io/crates/v/pelers?label=crates.io)](https://crates.io/crates/pelers) ![Language](https://img.shields.io/badge/language-Rust-orange?logo=rust) ![Tests](https://img.shields.io/badge/tests-cargo%20test-orange?logo=rust) ![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)


A fast, lightweight Mermaid renderer that outputs themeable SVG. `pelers` is derived from [Pelers](https://github.com/obsidianmd/pelers) by kepano. It is being ported from TypeScript to Rust, and will expose only a JavaScript/TypeScript binding (native addon for Node, Bun and Deno, WebAssembly for browsers), like [`markers`](https://github.com/farhnkrnapratma/markers) and [`mdrs`](https://github.com/farhnkrnapratma/mdrs).

## Status

Alpha. The port is in progress; the TypeScript renderer in [`ts/`](ts) is the reference until each diagram type matches it byte for byte.

| Registry  | Name                                             | State                                                                                                                                                                    |
| --------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| npm       | [`pelers`](https://www.npmjs.com/package/pelers) | `1.0.0-alpha1` is the TypeScript renderer (33 diagram types). The next version is the Rust binding and starts with flowcharts only; other types return `unsupported-diagram` until ported. |
| crates.io | [`pelers`](https://crates.io/crates/pelers)      | The Rust core: diagram type detection, preprocessing, flowchart parsing and a basic SVG renderer. There is no command-line interface.                                    |

The npm API of the Rust-backed version is `render`, `detectType`, `supports` and `PelersError`. `register()` and the per-diagram `pelers/diagrams/*` entry points of the TypeScript package go away.

## Install

```sh
bun add pelers          # JavaScript / TypeScript (or: npm install pelers)
cargo add pelers        # Rust
```

## Usage

JavaScript / TypeScript (Rust-backed version):

```ts
import { render } from 'pelers';

const { svg, width, height } = render('flowchart LR\n  A --> B');
```

Rust:

```rust
use pelers::{detect_diagram, parse_flowchart, preprocess, render_flowchart};

let prepared = preprocess("flowchart LR\n  A --> B")?;
if detect_diagram(&prepared.text).is_some() {
    let model = parse_flowchart(&prepared.text)?;
    let svg = render_flowchart(&model).svg;
}
```

## Repository layout

```
src/*.rs                 the Rust core: api, detect, preprocess, flowchart, render, errors
src/bindings/node.rs     the napi binding (feature `napi`)
index.js, index.d.ts     hand-written wrapper over the generated index.native.js / index.native.d.ts
scripts/                 binding checks
ts/                      the TypeScript renderer, kept as the oracle for the port (private, not published)
```

## Development

Requirements: Rust (stable), [Bun](https://bun.sh) and Node.js 20 or newer.

```sh
bun install                # JS dependencies and git hooks
bun run build              # napi build (debug) -> pelers.<platform>.node, index.native.js
bun run test:native        # release build + scripts/test-bindings.mjs
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
cargo deny check
```

The TypeScript oracle lives in `ts/` with its own dependencies:

```sh
cd ts
bun install
bun run typecheck          # tsc --noEmit (TypeScript 7)
bun run lint               # eslint (typescript-eslint on the TypeScript 6 API)
bun run fmt:check          # prettier
bun run test               # vitest
```

TypeScript 7 does not ship a programmatic API yet, so `ts/` installs both compilers side by side: `typescript` is an alias for `@typescript/typescript6` (used by typescript-eslint) and `@typescript/native` provides the `tsc` 7 binary. See the [TypeScript 7.0 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/).

Releases are cut with git-flow (`git flow release start vX.Y.Z`), so the tag carries the `v`. The npm package is published with `--tag alpha` while the version is a prerelease, and the crate with `cargo publish`.

## Upstream and attribution

Pelers is © kepano and distributed under the MIT License. The TypeScript implementation and test corpus in `ts/` carry over that work, together with material from Mermaid and Lucide. Their notices are reproduced in [`NOTICE`](NOTICE) and must stay with every copy of this code.

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
