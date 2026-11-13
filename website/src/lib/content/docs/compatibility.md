---
title: Compatibility
description: Compare syntax, layout, and output with Pele and Mermaid.
---

# Compatibility

Pelers accepts the Mermaid syntax that Pele 1.13.0 accepts, and it ports Pele's renderer diagram by diagram. Its output is compared with two references: Pele 1.13.0 (byte for byte) and the official Mermaid 12.1.0 (visually).

## Pele 1.13.0

For the sample inputs in Pelers' test corpus (613 diagrams across 32 of the 33 types, with `info` excluded), Pelers' SVG is byte-identical to Pele 1.13.0 after the product name is changed from `pele` to `pelers`. The renderer's tests also compare each diagram type against fixtures generated from Pele's TypeScript source.

This is evidence for those inputs, not a proof for every possible diagram. The `info` diagram is excluded on purpose: its output contains the version label, which changes the width of the box.

## Mermaid

Pelers draws its own layout and does not run Mermaid, so the SVG is not identical to Mermaid's. Compare the output with Mermaid when you move from Mermaid to Pelers, and check the examples in the [playground](/playground) or the [examples page](/examples) for the types you use.

In the benchmark run in this repository, Pelers renders each diagram type roughly 90 to 900 times faster than Mermaid in headless Chrome. The numbers depend on the machine and the input size; treat them as an order of magnitude.

## Diagram types

Pelers supports these Mermaid diagram types:

Flowchart, swimlane, git graph, Cynefin, entity relationship, block, Gantt, state, class, sequence, use case, agentflow, event modeling, Wardley map, architecture, requirement, railroad, tree view, info, journey, kanban, mindmap, Ishikawa, timeline, radar, treemap, quadrant chart, Venn, Sankey, XY chart, packet, and pie.

## Known differences

- `info` shows the Pelers version label, so its box is wider than Pele's.
- The JavaScript binding does not yet take options. Font size, padding, and the link policy need the Rust API.
- There is no browser build yet. The playground needs one before it can run Pelers in the page.
