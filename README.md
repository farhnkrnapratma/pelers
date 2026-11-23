# pelers

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/stZqvYiNAK17m5zy7ERCb/3MyzjfBswYtP68e4e1e9hk/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/stZqvYiNAK17m5zy7ERCb/3MyzjfBswYtP68e4e1e9hk/tree/main) [![codecov](https://codecov.io/gh/farhnkrnapratma/pelers/graph/badge.svg)](https://codecov.io/gh/farhnkrnapratma/pelers) [![npm](https://img.shields.io/npm/v/@farhnkrnapratma/pelers?label=npm)](https://www.npmjs.com/package/@farhnkrnapratma/pelers) [![crates.io](https://img.shields.io/crates/v/pelers?label=crates.io)](https://crates.io/crates/pelers) [![docs.rs](https://img.shields.io/docsrs/pelers)](https://docs.rs/pelers) ![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)

A fast Mermaid renderer that turns diagram source into themeable SVG, without a browser, a DOM or headless Chrome. The core is written in Rust and runs as a native addon in Node.js and Bun, as WebAssembly in browsers, as a Rust library and as a command-line tool.

pelers is a Rust port of [Pele](https://github.com/obsidianmd/pele) 1.13.0 by kepano (Obsidian). For all 33 diagram types it produces the same SVG as Pele on Pele's own test corpus, apart from the naming described in [Differences from Pele](#differences-from-pele).

> **Alpha.** The API follows Pele's and may still change before 1.0.0. The npm package moved from `pelers` to `@farhnkrnapratma/pelers` in 1.0.0-alpha4.

## Features

- **33 diagram types**: flowchart, sequence, class, state, ER, pie, gantt, gitGraph, mindmap, timeline, journey, xychart, quadrantChart, requirement, C4, sankey, block, packet, kanban, architecture, radar, treemap, info, agentflow, usecase, railroad, treeView, venn, cynefin, wardley, swimlane, ishikawa and eventmodeling.
- **Themeable output**: colours, fonts and corner radius come from `--pelers-*` CSS variables, so the same SVG follows a light or dark page.
- **Safe input handling**: link and image URLs are checked against scheme allow-lists, and input size, output size and edge count are capped.
- **Fast**: about 2.2 to 2.4 times faster than Pele on the server, and two orders of magnitude faster than Mermaid in the browser (see [Performance](#performance)).

## Install

```sh
npm install @farhnkrnapratma/pelers   # or: bun add, pnpm add
cargo add pelers                      # Rust library
cargo install pelers                  # command-line renderer
```

The npm package ships prebuilt native addons as optional dependencies, so nothing is compiled on install:

| OS      | Architectures                                                   |
| ------- | --------------------------------------------------------------- |
| Linux   | x64 and arm64 (glibc and musl), armv7, ppc64le, s390x, riscv64 |
| macOS   | arm64, x64                                                      |
| Windows | x64, arm64, ia32                                                |

Browsers get the WebAssembly build through the package's `browser` export.

## Usage

### Node.js and Bun

```ts
import { render } from "@farhnkrnapratma/pelers";

const { svg, width, height } = render("flowchart LR\n  A[Start] --> B[End]");
```

`render` accepts diagram source or a model returned by `parse`, so a diagram can be parsed once and rendered with different options:

```ts
import { parse, render } from "@farhnkrnapratma/pelers";

const model = parse('pie\n  "Dogs": 3\n  "Cats": 1');
const small = render(model, { fontSize: 12, maxWidth: 320 });
const large = render(model, { fontSize: 18, padding: 16 });
```

`render` returns `{ svg, width, height, links, type, mermaidType, title? }`. Invalid source throws a `PelersError` with `code`, `type`, `mermaidType`, `line`, `column` and `snippet`. `detectType` and `supports` tell whether a text is a diagram pelers can render.

### Browsers

With a bundler, import the package as above: the `browser` export resolves to WebAssembly. `mount` renders into an element and can add pan and zoom:

```ts
import { mount } from "@farhnkrnapratma/pelers";

mount(document.querySelector("#diagram"), source, { zoom: "auto" });
```

To keep the WebAssembly module out of the initial bundle, import from `@farhnkrnapratma/pelers/lazy` and use `renderAsync` or `mountAsync`, which load the module on first use. Without a bundler, load `@farhnkrnapratma/pelers/global` with a `<script>` tag and call `globalThis.pelers.render`.

| Import (from `@farhnkrnapratma/pelers`) | Build | Size (gzip) |
| ---------------------------------------- | ----- | ----------- |
| `.`, `/lazy`, `/global`                  | fast  | 1.5 MB      |
| `/small`, `/small/lazy`, `/small/global` | small | 0.96 MB     |

Both builds render the same SVG. The fast build renders faster; the small build downloads less.

### Rust

```rust
use pelers::{ParseOptions, RenderOptions, parse_with, render_with};

let model = parse_with("flowchart LR\n  A --> B", &ParseOptions::default())?;
let rendered = render_with(&model, &RenderOptions::default())?;
println!("{}", rendered.svg);
```

### Command line

```sh
pelers diagram.mmd > diagram.svg     # one file to stdout
pelers --out svg/ docs/*.mmd         # one SVG per file, rendered in parallel
cat diagram.mmd | pelers             # from stdin
```

`--threads N` sets the number of worker threads. The exit status is non-zero if any input fails to render.

## Options

The most used `RenderOptions`:

| Option                                     | Purpose                                                                   |
| ------------------------------------------ | ------------------------------------------------------------------------- |
| `fontSize`, `fontFamily`, `fontFamilyMono` | Text size and font stacks                                                 |
| `maxWidth`, `padding`, `responsive`        | Layout width, outer padding, and whether the SVG scales to its container |
| `autoDirection`, `directionBreakpoint`     | Turn wide diagrams vertical on narrow widths                              |
| `idPrefix`                                 | Prefix for element ids, for several diagrams on one page                  |
| `links`, `linkSchemes`, `linkRel`          | Whether links are kept, which URL schemes they may use, and their `rel`   |
| `images`, `imageSchemes`                   | The same for images                                                       |
| `limit`, `outputLimit`, `maxEdges`         | Caps on input size, output size and edge count                            |
| `icons`, `math`, `measurer`                | Callbacks for icons, math markup and text measurement                     |
| `config`                                   | Mermaid configuration, as in a `%%{init: …}%%` directive                  |
| `now`                                      | The current time, for gantt's today marker                                |

Every option and type is declared in [`index.d.ts`](https://github.com/farhnkrnapratma/pelers/blob/main/index.d.ts).

## Theming

The SVG reads its colours from CSS variables and falls back to a light theme. Set them on the page or on a container:

```css
.diagram {
  --pelers-bg: #0d1117;
  --pelers-fg: #e6edf3;
  --pelers-line: #8b949e;
  --pelers-accent: #58a6ff;
}
```

The variables are `--pelers-bg`, `--pelers-fg`, `--pelers-muted`, `--pelers-line`, `--pelers-surface`, `--pelers-surface-alt`, `--pelers-border`, `--pelers-accent`, `--pelers-critical`, `--pelers-series-1` to `--pelers-series-8`, `--pelers-font`, `--pelers-font-mono` and `--pelers-radius`.

## Performance

Total render time over a 631-diagram corpus, compared with the official `pele` 1.13.0 and `mermaid` 12.3.0 packages (higher is faster):

| Runtime                    | vs Pele | vs Mermaid |
| -------------------------- | ------- | ---------- |
| Bun, native addon          | 2.4×    |            |
| Node.js, native addon      | 2.2×    |            |
| Chrome, WebAssembly fast   | 1.0×    | 265×       |
| Chrome, WebAssembly small  | 0.7×    | 180×       |

Per-diagram numbers and the method are in [`bench/results.md`](https://github.com/farhnkrnapratma/pelers/blob/main/bench/results.md).

## Differences from Pele

- Names use `pelers`: CSS classes are `pelers-*`, CSS variables are `--pelers-*` and the error class is `PelersError`.
- The `info` diagram shows the pelers version.
- xychart rounds tick values to 15 significant digits, which keeps floating-point noise such as `0.30000000000000004` out of axis labels.

## Contributing

Bug reports and pull requests are welcome. [`CONTRIBUTING.md`](https://github.com/farhnkrnapratma/pelers/blob/main/CONTRIBUTING.md) explains how to build, test and submit changes.

## License

Licensed under either of the [Apache License, Version 2.0](https://github.com/farhnkrnapratma/pelers/blob/main/LICENSE-APACHE) or the [MIT License](https://github.com/farhnkrnapratma/pelers/blob/main/LICENSE-MIT), at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

pelers is derived from Pele (© kepano, MIT License) and includes material from Mermaid and Lucide. Their notices are in [`NOTICE`](https://github.com/farhnkrnapratma/pelers/blob/main/NOTICE) and must stay with every copy of this code.
