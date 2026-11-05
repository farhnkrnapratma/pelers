# pelers

A fast, lightweight Mermaid renderer that outputs themeable SVG. `pelers` is derived from [Pele](https://github.com/obsidianmd/pele) by kepano: the npm package is the TypeScript renderer, and the Rust crate is a port that is being built up one diagram type at a time.

## Status

Alpha (`v1.0.0-alpha1`).

| Registry  | Name                                             | State                                                                                                                                                      |
| --------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| npm       | [`pelers`](https://www.npmjs.com/package/pelers) | The TypeScript renderer: Mermaid 12.1.0 syntax, themeable SVG, browser mounting, lazy loading and per-diagram registration.                                |
| crates.io | [`pelers`](https://crates.io/crates/pelers)      | The Rust port: diagram type detection, preprocessing, flowchart parsing and a basic SVG renderer. Other diagram types and the full CLI are not ported yet. |

## Install

```sh
bun add pelers          # TypeScript / JavaScript (or: npm install pelers)
cargo add pelers        # Rust
```

## Usage

TypeScript / JavaScript:

```ts
import { render } from 'pelers';

const { svg, width, height } = render('flowchart LR\n  A --> B');
```

Smaller bundles: `pelers/core` plus the diagram types you need (`pelers/diagrams/<type>`), or `pelers/lazy` to load a type the first time it is rendered. `pelers/global` is a browser build that exposes a global.

Rust:

```rust
use pelers::{detect_diagram, parse_flowchart, preprocess, render_flowchart};

let prepared = preprocess("flowchart LR\n  A --> B")?;
if detect_diagram(&prepared.text).is_some() {
    let model = parse_flowchart(&prepared.text)?;
    let svg = render_flowchart(&model).svg;
}
```

Command line (reads Mermaid source on standard input, prints the detected diagram and, for flowcharts, the SVG):

```sh
printf 'flowchart LR\n  A --> B\n' | cargo run
```

## Repository layout

```
src/*.rs          the Rust crate: api, detect, preprocess, flowchart, render, errors
src/diagrams/     the TypeScript renderer, one directory per diagram type
src/entries/      generated per-diagram entry points
scripts/          build, benchmarks and generators (registry, metrics, unicode, specs)
tests/            vitest suites, compatibility specs and corpus
```

## Development

Requirements: Rust (stable), [Bun](https://bun.sh) and Node.js 20 or newer.

```sh
bun install                # JS dependencies and git hooks
bun run build              # bundles dist/ with esbuild
bun run typecheck          # tsc --noEmit (TypeScript 7)
bun run lint               # eslint (typescript-eslint on the TypeScript 6 API)
bun run fmt:check          # prettier
bun run test               # vitest
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
cargo deny check
```

TypeScript 7 does not ship a programmatic API yet, so the repository installs both compilers side by side: `typescript` is an alias for `@typescript/typescript6` (used by typescript-eslint) and `@typescript/native` provides the `tsc` 7 binary. See the [TypeScript 7.0 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/).

Releases are cut with git-flow (`git-flow release start/finish`), tagged `vX.Y.Z`. The npm package is published with `npm publish --tag alpha` while the version is a prerelease, and the crate with `cargo publish`.

## Upstream and attribution

Pele is © kepano and distributed under the MIT License. The TypeScript implementation and test corpus here carry over that work, together with material from Mermaid and Lucide. Their notices are reproduced in [`NOTICE`](NOTICE) and must stay with every copy of this code.

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

Copyright © 2026 Farhan Kurnia Pratama. Portions are derived from Pele (© kepano, MIT); see [NOTICE](NOTICE).
