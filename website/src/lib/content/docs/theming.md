---
title: Theming
description: Use CSS to apply colors, fonts, and other styles.
---

# Theming

Pelers writes colors and fonts as CSS variables on the SVG root. Set the variables on the SVG, or on any ancestor, and the diagram follows. Changing a variable does not require rendering again.

## Variables

| Variable | Default | Used for |
| --- | --- | --- |
| `--pelers-bg` | `#fff` | Background of nodes and labels |
| `--pelers-fg` | `#1f1f1f` | Text |
| `--pelers-muted` | `#6e6e6e` | Secondary text, such as axis labels |
| `--pelers-line` | `#1f1f1f` | Edges and outlines |
| `--pelers-surface` | `#f3f3f3` | Filled shapes, such as cards and bands |
| `--pelers-surface-alt` | `#e6e6e6` | Alternate fills |
| `--pelers-border` | `#8a8a8a` | Borders |
| `--pelers-accent` | `#5b7bd5` | Highlights and selected items |
| `--pelers-critical` | `#e45756` | Errors and critical items |
| `--pelers-font` | `sans-serif` | Label font |
| `--pelers-font-mono` | `monospace` | Monospace label font |
| `--pelers-radius` | `4px` | Corner radius |
| `--pelers-fg-on-light` | `#1f1f1f` | Text on light fills |
| `--pelers-fg-on-dark` | `#fff` | Text on dark fills |
| `--pelers-series-1` … `--pelers-series-8` | the built-in series colors | Series in charts and pies |

```css
.pelers {
  --pelers-bg: #fffcf0;
  --pelers-fg: #100f0f;
  --pelers-line: #100f0f;
}
```

## Dark mode

Set the variables on a parent that carries your dark-mode class:

```css
.dark .pelers {
  --pelers-bg: #1c1b1a;
  --pelers-fg: #f2f0e5;
  --pelers-line: #f2f0e5;
  --pelers-surface: #2a2926;
}
```

## Fonts

Set `--pelers-font` and `--pelers-font-mono` to the font stacks your page uses. The text is measured with the same font the browser draws, so set the variables before you render, or render again after you change them.

## Diagram styles

Mermaid `classDef` and `style` statements are honored when they set `fill`, `stroke`, `color`, and the other properties the renderer supports. Values that could break out of the SVG are dropped, and the rest of the diagram keeps its theme.
