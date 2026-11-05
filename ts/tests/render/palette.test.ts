import { describe, expect, it } from 'vitest';
import { render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const options = { measurer: metricsMeasurer };

const themed = (vars: Record<string, unknown>, body: string): string =>
  `%%{init: ${JSON.stringify({ themeVariables: vars })}}%%\n${body}`;

function series(svg: string): Record<number, string> {
  const style = /^<svg[^>]*\bstyle="([^"]*)"/.exec(svg)![1];
  const found: Record<number, string> = {};
  for (const [, n, color] of style.matchAll(/--pele-series-(\d):([^;]*)/g)) found[Number(n)] = color;
  return found;
}

const PIE = 'pie\n  "A": 1\n  "B": 2';
const XYCHART = 'xychart\n  bar [1, 2]\n  line [2, 1]';

describe('palette theme variables', () => {
  it('colors xychart series from plotColorPalette, repeating a short list', () => {
    const source = `---
config:
  themeVariables:
    xyChart:
      plotColorPalette: '#000000, #0000FF, #00FF00, #FF0000'
---
xychart
title "Different Colors in xyChart"
x-axis "categoriesX" ["Category 1", "Category 2", "Category 3", "Category 4"]
y-axis "valuesY" 0 --> 50
line [10,20,30,40]
bar [20,30,25,35]
bar [15,25,20,30]
line [5,15,25,35]`;
    const { svg } = render(source, options);
    expect(series(svg)).toEqual({
      1: '#000000',
      2: '#0000FF',
      3: '#00FF00',
      4: '#FF0000',
      5: '#000000',
      6: '#0000FF',
      7: '#00FF00',
      8: '#FF0000',
    });
    expect(svg).toContain('stroke="var(--pele-series-1,');
  });

  it('keeps commas inside color functions', () => {
    const { svg } = render(
      themed({ xyChart: { plotColorPalette: 'rgb(0, 0, 0), hsl(0 50% 50% / 0.5)' } }, XYCHART),
      options
    );
    expect(series(svg)[1]).toBe('rgb(0, 0, 0)');
    expect(series(svg)[2]).toBe('hsl(0 50% 50% / 0.5)');
  });

  it('maps each diagram palette onto the series in order', () => {
    const cases: [string, Record<string, string>][] = [
      [PIE, { pie1: 'red', pie2: 'blue' }],
      ['gitGraph\n  commit', { git0: 'red', git1: 'blue' }],
      ['timeline\n  2001 : a\n  2002 : b', { cScale0: 'red', cScale1: 'blue' }],
      ['radar-beta\n  axis a, b, c\n  curve x{1,2,3}', { cScale0: 'red', cScale1: 'blue' }],
      ['treemap\n"a": 1\n"b": 2', { cScale0: 'red', cScale1: 'blue' }],
      ['mindmap\n  root\n    a\n    b', { cScale0: 'green', cScale1: 'red', cScale2: 'blue' }],
    ];
    for (const [body, vars] of cases) {
      expect(series(render(themed(vars, body), options).svg), body).toEqual({ 1: 'red', 2: 'blue' });
    }
  });

  it('repeats the venn colors that are set', () => {
    const { svg } = render(themed({ venn2: 'red', venn4: 'blue' }, 'venn-beta\n  set A\n  set B'), options);
    expect(series(svg)).toEqual({ 1: 'red', 2: 'blue', 3: 'red', 4: 'blue', 5: 'red', 6: 'blue', 7: 'red', 8: 'blue' });
  });

  it('ignores palettes of other diagrams and other theme colors', () => {
    expect(series(render(themed({ cScale0: 'red', primaryColor: 'red' }, PIE), options).svg)).toEqual({});
    expect(
      series(render(themed({ fillType0: 'red', cScale0: 'red' }, 'journey\n  section S\n    Task: 5: Me'), options).svg)
    ).toEqual({});
    expect(series(render(themed({ pie1: 'red' }, 'flowchart LR\n  a --> b'), options).svg)).toEqual({});
  });

  it('reads theme variables from front matter, directives and the config option', () => {
    const front = `---\nconfig:\n  themeVariables:\n    pie1: '#123456'\n---\n${PIE}`;
    expect(series(render(front, options).svg)).toEqual({ 1: '#123456' });
    const fromOption = render(PIE, { ...options, config: { themeVariables: { pie2: 'oklch(70% 0.1 200)' } } }).svg;
    expect(series(fromOption)).toEqual({ 2: 'oklch(70% 0.1 200)' });
  });

  it('accepts theme-aware colors', () => {
    const vars = {
      pie1: 'var(--pele-accent)',
      pie2: 'light-dark(#000, #fff)',
      pie3: 'color-mix(in srgb, red 40%, blue)',
    };
    expect(series(render(themed(vars, PIE), options).svg)).toEqual({
      1: 'var(--pele-accent)',
      2: 'light-dark(#000, #fff)',
      3: 'color-mix(in srgb, red 40%, blue)',
    });
  });

  it('drops values that are not colors and keeps the theme color in their place', () => {
    const unsafe = [
      'url(https://example.com/a.svg)',
      'url(//example.com/a.svg)',
      'image-set(a 1x)',
      'red;stroke:blue',
      '"><script>alert(1)</script>',
      'expression(alert(1))',
      'rgb(0, 0',
      'red)',
      'red}',
      'javascript:alert(1)',
      '/*',
      'reddish',
      '#ff00000',
      'var(--page-color)',
      'color-mix(in srgb, var(--page-color) 50%, red)',
    ];
    for (const value of unsafe) {
      const { svg } = render(themed({ pie1: value, pie2: 'blue' }, PIE), options);
      expect(series(svg), value).toEqual({ 2: 'blue' });
      expect(svg).not.toContain('<script');
      expect(svg).not.toContain('url(');
    }
    const list = render(themed({ xyChart: { plotColorPalette: 'url(a), blue' } }, XYCHART), options).svg;
    expect(series(list)).toEqual({ 2: 'blue', 4: 'blue', 6: 'blue', 8: 'blue' });
  });

  it('picks label colors that contrast with palette colors', () => {
    const ON_LIGHT = 'fill="var(--pele-fg-on-light,#1f1f1f)"';
    const ON_DARK = 'fill="var(--pele-fg-on-dark,#fff)"';
    const pie = render(
      themed({ pie1: '#ffeb3b', pie2: 'navy', pie3: 'var(--pele-accent)' }, 'pie\n  "A": 1\n  "B": 1\n  "C": 1'),
      options
    ).svg;
    const labels = [...pie.matchAll(/<text [^>]*>\d+%</g)].map((m) => m[0]);
    expect(labels[0]).toContain(ON_LIGHT);
    expect(labels[1]).toContain(ON_DARK);
    expect(labels[2]).not.toContain('fill=');
    expect(pie).toContain('class="pele-slice-labels"');

    const git = render(themed({ git0: '#ffeb3b' }, 'gitGraph\n  commit\n  branch dev\n  commit'), options).svg;
    expect(git).toMatch(new RegExp(`class="pele-label" ${ON_LIGHT.replace(/[()]/g, '\\$&')}`));
    expect(git).toContain('class="pele-label" fill="var(--_bg)"');

    const timeline = render(themed({ cScale0: '#ffeb3b' }, 'timeline\n  section S\n    2001 : a'), options).svg;
    expect(timeline.split(ON_LIGHT).length - 1).toBe(2);

    const treemap = render(themed({ cScale0: '#ffeb3b' }, 'treemap\n"a": 1\n"b": 1'), options).svg;
    expect(treemap.split(ON_LIGHT).length - 1).toBe(2);
  });

  it('leaves the output unchanged without a palette', () => {
    expect(render(PIE, options).svg).toBe(render(themed({ primaryColor: 'red' }, PIE), options).svg);
    expect(render(PIE, options).svg).not.toContain('--pele-series-1:');
  });
});
