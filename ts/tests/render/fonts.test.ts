import { describe, expect, it } from 'vitest';
import { render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { assertInert } from '../support/inert.js';

const FLOW = 'flowchart LR\n  A --> B';
const root = (options: object = {}): string =>
  /^<svg[^>]*>/.exec(render(FLOW, { measurer: metricsMeasurer, ...options }).svg)![0];

describe('fonts', () => {
  it('draws in generic fonts unless told what was measured', () => {
    expect(root()).toContain(';--_fm:var(--pelers-font-mono,monospace);font:16px sans-serif;');
    expect(root()).toMatch(/;font-family:var\(--pelers-font,sans-serif\)"/);
    expect(root()).not.toContain(' font-family=');
  });

  it('draws in the fonts it measured with, where the page names no others', () => {
    const drawn = root({ fontFamily: '"Segoe UI", system-ui, sans-serif', fontFamilyMono: "'SF Mono', ui-monospace" });
    expect(drawn).toContain('font-family:var(--pelers-font,&quot;Segoe UI&quot;, system-ui, sans-serif)"');
    expect(drawn).toContain("--_fm:var(--pelers-font-mono,'SF Mono', ui-monospace);");
    expect(root({ fontFamily: '思源黑体, sans-serif' })).toContain('var(--pelers-font,思源黑体, sans-serif)');
    // Obsidian's lists start with placeholders for the fonts a user or a theme has not set.
    expect(root({ fontFamily: '"??", "??", ui-sans-serif, -apple-system, system-ui' })).toContain(
      'var(--pelers-font,&quot;??&quot;, &quot;??&quot;, ui-sans-serif, -apple-system, system-ui)'
    );
  });

  it('pins what text would inherit from the page and was not measured', () => {
    for (const pinned of ['letter-spacing:normal', 'word-spacing:normal', 'text-transform:none', 'direction:ltr']) {
      expect(root()).toContain(`;${pinned};`);
    }
  });

  it('takes the weight of bold text from variables, each defined only where it is used', () => {
    const plain = render(FLOW, { measurer: metricsMeasurer }).svg;
    expect(plain).not.toContain('font-weight');
    expect(plain).not.toMatch(/--_t?h?w:/);

    const titled = render(`---\ntitle: My title\n---\n${FLOW}\n  B --> C["\`**Bold** word\`"]`, {
      measurer: metricsMeasurer,
    }).svg;
    expect(titled).toContain(';--_tw:var(--pelers-title-weight,bold);--_w:var(--pelers-bold-weight,bold);');
    expect(titled).not.toContain('--_hw:');
    // Written once, on the text, so that a rule for .pelers-title is enough to change it.
    expect(titled).toMatch(
      /<text class="pelers-title" font-weight="var\(--_tw\)"[^>]*><tspan x="[\d.]+" y="[\d.]+">My title<\/tspan><\/text>/
    );
    expect(titled).toContain('<tspan x="0" y="5.6" font-weight="var(--_w)">Bold</tspan><tspan> word</tspan>');
  });

  it('draws the names of classes, columns, and other headings in the heading weight', () => {
    const sources = [
      'classDiagram\n  class Duck {\n    +swim()\n  }',
      'kanban\n  Todo\n    [Write docs]',
      'treeView-beta\n    src/\n        index.js\n',
      'sequenceDiagram\n  loop Daily\n    A->>B: x\n  end',
      'timeline\n  section Early\n    2001 : First',
    ];
    for (const source of sources) {
      const { svg } = render(source, { measurer: metricsMeasurer });
      expect(svg, source).toContain(';--_hw:var(--pelers-heading-weight,bold);');
      expect(svg, source).toMatch(/<text[^>]* font-weight="var\(--_hw\)"/);
      expect(svg, source).not.toMatch(/font-weight="(?!var\(--_hw\)")/);
    }
  });

  it('writes the corner radius in a style, which every browser reads a variable from', () => {
    const { svg } = render('flowchart LR\n  A[One] --> B[Two]\n  style B rx:10,fill:#eee', {
      measurer: metricsMeasurer,
    });
    expect(svg).toContain(';--_r:var(--pelers-radius,4px)');
    expect(svg).not.toContain('rx="var(');
    expect(svg).toMatch(/<rect[^>]* style="rx:var\(--_r\)"\/>/);
    // A radius the diagram sets for a node comes after the theme's, and so wins.
    expect(svg).toContain('style="rx:var(--_r);rx:10;fill:#eee;"');
    expect(render('pie\n  "a": 1', { measurer: metricsMeasurer }).svg).not.toContain('--_r');
  });

  it('does not write a font list that is more than names', () => {
    for (const family of [
      'x;color:red',
      'x) ; y',
      'url(//evil.example/f)',
      '"url(x)"',
      '"open',
      "it's",
      'a\nb',
      'x{}',
      '<b>',
      '"a";"b"',
      'a,,b',
      ', a',
      '',
    ]) {
      const svg = render(FLOW, { measurer: metricsMeasurer, fontFamily: family, fontFamilyMono: family }).svg;
      expect(svg, family).toContain('font-family:var(--pelers-font,sans-serif)"');
      expect(svg, family).toContain('--_fm:var(--pelers-font-mono,monospace);');
      assertInert(svg, family);
    }
  });
});
