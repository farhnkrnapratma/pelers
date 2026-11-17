import { describe, expect, it } from 'vitest';
import { render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { assertInert, inertProblem } from '../support/inert.js';

// The check that the other security tests rely on. If it stopped objecting, they would all pass.
const svg = (inner: string, attrs = ''): string =>
  `<svg xmlns="http://www.w3.org/2000/svg" class="pelers"${attrs}>${inner}</svg>`;

describe('the inertness check', () => {
  it('accepts what Pelers draws', () => {
    const drawn = render(
      'flowchart LR\n  A[One] --> B[Two]\n  click A href "https://example.com/a?b=1#c"\n  style B fill:#eee,stroke-width:2px',
      { measurer: metricsMeasurer }
    ).svg;
    expect(inertProblem(drawn)).toBeUndefined();
    expect(() => assertInert(drawn, 'flowchart')).not.toThrow();
    for (const href of [
      'https://example.com',
      'http://example.com/a',
      'mailto:a@example.com',
      'tel:+15550100',
      './a/b',
      '/a',
      'a.html',
      '#top',
      '?q=1',
    ]) {
      expect(inertProblem(svg(`<a href="${href}"><text>x</text></a>`)), href).toBeUndefined();
    }
  });

  it('objects to anything else', () => {
    const bad: [string, string, RegExp][] = [
      ['script element', svg('<script>alert(1)</script>'), /element <script>/],
      ['style element', svg('<style>a{}</style>'), /element <style>/],
      ['foreign object', svg('<foreignObject></foreignObject>'), /element <foreignObject>/],
      ['event handler', svg('<rect onclick="alert(1)"/>'), /attribute onclick/],
      ['unknown attribute', svg('<rect xlink:href="x"/>'), /attribute xlink:href/],
      ['unquoted markup', svg('<rect width=1/>'), /Malformed tag/],
      ['unclosed element', svg('<g><rect/>'), /Unclosed <g>|closes/],
      ['raw ampersand', svg('<text>a & b</text>'), /Unescaped "&"/],
      ['size that is not a number', svg('<rect width="1;x"/>'), /width="1;x" is not a number/],
      ['radius that loads something', svg('<rect rx="url(#a)"/>'), /rx=/],
      ['href on a shape', svg('<rect href="https://example.com"/>'), /href on <rect>/],
      ['script link', svg('<a href="javascript:alert(1)"></a>'), /can run script/],
      ['script link in capitals and spaces', svg('<a href=" JaVaScRiPt :alert(1)"></a>'), /can run script/],
      ['data link', svg('<a href="data:text/html,x"></a>'), /can run script/],
      ['file link', svg('<a href="file:///etc/passwd"></a>'), /scheme that is not allowed/],
      [
        'file link behind a tab',
        svg('<a href="fi&#9;le:///etc/passwd"></a>'),
        /scheme that is not allowed|not allowed/,
      ],
      ['network path', svg('<a href="//evil.example/x"></a>'), /scheme that is not allowed/],
      ['backslash network path', svg('<image href="\\\\evil.example\\x"/>'), /scheme that is not allowed/],
      ['app link', svg('<a href="obsidian://open"></a>'), /scheme that is not allowed/],
      ['blank page', svg('<a href="about:blank"></a>'), /scheme that is not allowed/],
      ['target', svg('<a href="/a" target="evil"></a>'), /target="evil"/],
      ['style that loads something', svg('<rect style="fill:url(//evil.example/x)"/>'), /style=/],
      ['style property', svg('<rect style="position:fixed"/>'), /style property position/],
      ['root sizing on another element', svg('<rect style="height:auto"/>'), /style property height/],
      ['fill that loads something', svg('<rect fill="url(#a)"/>'), /fill=/],
      ['font that loads something', svg('<text font-family="x, url(y)">a</text>'), /font-family=/],
    ];
    for (const [name, markup, message] of bad) {
      expect(inertProblem(markup) ?? 'no objection', name).toMatch(message);
      expect(() => assertInert(markup, name), name).toThrow(message);
    }
  });

  it('names where it was when it objects', () => {
    expect(() => assertInert(svg('<script/>'), 'timeline: mutated')).toThrow('timeline: mutated: element <script>');
  });
});
