---
title: API
description: Render Mermaid diagrams from JavaScript, TypeScript, or Rust.
---

# API

Pelers ships one native binding for Node and Bun, and a Rust crate with the same renderer.

## JavaScript and TypeScript

```ts
import { detectType, PelersError, render, renderFull, supports } from 'pelers';

const svg = render('pie title Pets\n  "Dogs": 3\n  "Cats": 1');
const { svg: sized, width, height, title } = renderFull('pie\n  "Dogs": 3');
```

### `render(source: string): string`

Returns the SVG for one Mermaid diagram. The SVG has a `class="pelers pelers-<type>"` root and styles itself through [CSS variables](/docs/theming).

### `renderFull(source: string): RenderResult`

Returns `{ svg, width, height, title }`. Use it when you need the size of the diagram or its accessibility title. `title` is `undefined` when the diagram has no title.

### `detectType(source: string): string | null`

Returns the diagram type that Pelers detects (for example `flowchart` or `pie`), or `null` when the text is not a Mermaid diagram Pelers knows.

### `supports(source: string): boolean`

Returns `true` when `detectType` finds a diagram type. Pelers supports every type it detects; a `false` result means the text is not recognized as Mermaid.

### `PelersError`

Thrown by `render` and `renderFull`. Its `code` is one of:

| Code | Meaning |
| --- | --- |
| `unsupported-diagram` | The text does not start with a recognized Mermaid diagram header. |
| `syntax` | The diagram text is not valid for its type. The message names the line and the expected token. |
| `semantic` | The text parses but describes something the renderer cannot draw. |
| `limit` | The source or the output is longer than the limit below. |

```ts
try {
  render('hello');
} catch (error) {
  if (error instanceof PelersError && error.code === 'unsupported-diagram') {
    // show a message to the user
  }
}
```

### Limits

- Source: 50,000 UTF-16 code units (`limit`).
- Output: 4,000,000 characters.
- Flowcharts and agentflow diagrams: 5,000 edges (`max_edges`).

The JavaScript binding uses these defaults. Changing them needs the Rust API.

## Rust

```rust
use pelers::{ParseOptions, RenderOptions, parse_with, render_with};

let model = parse_with("flowchart LR\n  A --> B", &ParseOptions::default())?;
let rendered = render_with(&model, &RenderOptions::default())?;
println!("{}", rendered.svg);
```

- `pelers::parse` and `pelers::render` use the defaults.
- `ParseOptions` has `limit` (source length) and `max_edges`.
- `RenderOptions` has the font size, padding, `max_width`, `link_schemes`, `image_schemes`, and the `id_prefix` that keeps ids unique when several diagrams share a page.
- `pelers::api::render_batch(&sources, threads)` renders many diagrams in parallel and returns the results in input order.

The JavaScript binding does not yet expose `RenderOptions` or `ParseOptions`.
