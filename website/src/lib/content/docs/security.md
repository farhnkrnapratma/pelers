---
title: Security
description: Render and display untrusted diagrams.
---

# Security

Mermaid text can come from users. Pelers escapes what it writes into the SVG and limits how much work one diagram can cause.

## Text and markup

- Labels, titles, and ids are XML-escaped before they reach the SVG.
- Characters that XML cannot hold are dropped.
- Diagram source is never copied into the SVG as markup; it is only read.

Display the SVG with the browser's image or inline SVG handling, and do not run scripts from the diagram text. Pelers does not emit `<script>` elements.

## Links

Mermaid `click` links are kept only when the scheme is on the allow-list:

- Links: `http`, `https`, `mailto`, `tel`.
- Images: `http`, `https`.

Set `link_schemes` and `image_schemes` in the Rust `RenderOptions` to change the lists. The JavaScript binding uses the defaults.

## Limits

These limits stop one diagram from using too much time or memory:

- Source: 50,000 UTF-16 code units.
- Output: 4,000,000 characters.
- Flowchart and agentflow edges: 5,000.

When a limit is exceeded, Pelers returns a `limit` error and renders nothing.

## Reporting

Report a security problem privately through [GitHub](https://github.com/farhnkrnapratma/pelers/security/advisories/new). Do not open a public issue for it.
