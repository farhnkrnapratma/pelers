import { describe, expect, it } from 'vitest';
import { PelersError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const options = { measurer: metricsMeasurer };
const count = (svg: string, re: RegExp): number => svg.match(re)?.length ?? 0;

const BASIC =
  'cynefin-beta\n  title Incident Response\n  complex\n    "Investigate root cause"\n    "Run chaos experiment"\n  complicated\n    "Expert review needed"\n' +
  '  clear\n    "Restart service"\n  chaotic\n    "Page on-call immediately"\n  confusion\n    "Unknown failure mode"\n';

describe('cynefin rendering', () => {
  it('draws the five domains, their items, and the boundaries', () => {
    const { svg, type } = render(BASIC, options);
    expect(type).toBe('cynefin');
    expect(supports(BASIC)).toBe(true);
    expect(count(svg, /class="pelers-cynefin-region"/g)).toBe(5);
    expect(count(svg, /class="pelers-cynefin-domain"/g)).toBe(5);
    expect(count(svg, /class="pelers-node"/g)).toBe(6);
    for (const name of ['Complex', 'Complicated', 'Clear', 'Chaotic', 'Confusion']) expect(svg).toContain(`>${name}<`);
    expect(svg).toContain('Probe → Sense → Respond');
    expect(svg).toContain('>Disorder<');
    expect(svg).toContain('class="pelers-cynefin-cliff"');
    expect(svg).toContain('>Incident Response<');
    for (let i = 1; i <= 5; i++) expect(svg).toContain(`var(--pelers-series-${i},`);
    expect(svg).not.toContain('NaN');
  });

  it('draws every domain even when none is declared', () => {
    const { svg } = render('cynefin-beta', options);
    expect(count(svg, /class="pelers-cynefin-domain"/g)).toBe(5);
    expect(count(svg, /class="pelers-node/g)).toBe(0);
  });

  it('draws transitions with arrowheads and labels, and drops self loops', () => {
    const src =
      'cynefin-beta\ncomplex --> complicated : "Pattern identified"\nclear --> chaotic\nclear --> clear : "ignored"\ncomplex --> clear : "diagonal"\nconfusion --> chaotic\n';
    const { svg } = render(src, options);
    expect(count(svg, /class="pelers-edge"/g)).toBe(4);
    expect(count(svg, /class="pelers-marker"/g)).toBe(4);
    expect(svg).toContain('data-id="complex-complicated"');
    expect(svg).toContain('>Pattern identified</text>');
    expect(svg).not.toContain('ignored');
    const model = parse(src);
    if (model.type === 'cynefin')
      expect(model.transitions.map((t) => `${t.from}>${t.to}`)).toEqual([
        'complex>complicated',
        'clear>chaotic',
        'complex>clear',
        'confusion>chaotic',
      ]);
  });

  it('caps the confusion list at three items', () => {
    const { svg } = render('cynefin-beta\nconfusion\n"a"\n"b"\n"c"\n"d"\n"e"\n', options);
    expect(count(svg, /class="pelers-node"/g)).toBe(3);
    expect(svg).toContain('class="pelers-node pelers-cynefin-overflow"');
    expect(svg).toContain('>+2 more</text>');
  });

  it('keeps the last block when a domain is declared twice', () => {
    const model = parse('cynefin-beta\ncomplex\n"first"\ncomplex\n"second"\n');
    expect(model.type).toBe('cynefin');
    if (model.type === 'cynefin') expect(model.domains.get('complex')!.items).toEqual([{ label: 'second' }]);
  });

  it('grows to fit long lists and wraps long labels', () => {
    const base = render('cynefin-beta\ncomplex\n"a"\n', options);
    const tall = render('cynefin-beta\ncomplex\n' + '"an item"\n'.repeat(20), options);
    const wide = render('cynefin-beta\ncomplex\n"' + 'word '.repeat(30) + '"\n', options);
    expect(tall.height).toBeGreaterThan(base.height);
    expect(wide.width).toBeLessThan(base.width + 200);
    expect(count(wide.svg, /<tspan/g)).toBeGreaterThan(3);
  });

  it('honours the options', () => {
    const base = render(BASIC, options);
    const front = (body: string): string => `---\nconfig:\n  cynefin:\n${body}---\n${BASIC}`;
    expect(render(front('    showDomainDescriptions: false\n'), options).svg).not.toContain('Probe');
    expect(render(front('    width: 1200\n    height: 900\n'), options).width).toBeGreaterThan(base.width);
    const straight = render(front('    boundaryAmplitude: 0\n'), options).svg;
    expect(straight).not.toBe(base.svg);
    expect(render(front('    seed: 5\n'), options).svg).not.toBe(render(front('    seed: 6\n'), options).svg);
    expect(render(BASIC, { ...options, idPrefix: 'other' }).svg).not.toBe(base.svg);
    expect(render(BASIC, options).svg).toBe(base.svg);
  });

  it('draws titles and accessible names', () => {
    const { svg } = render(
      '---\ntitle: Front\n---\ncynefin-beta\naccTitle: Acc\naccDescr: Described\nclear\n"a"\n',
      options
    );
    expect(svg).toContain('>Front<');
    expect(svg).toContain('<title id="pelers-title">Acc</title>');
    expect(svg).toContain('<desc id="pelers-desc">Described</desc>');
    expect(render('---\ntitle: Front\n---\ncynefin-beta\ntitle Own\n', options).svg).toContain('>Own<');
  });

  it('rejects what Mermaid rejects', () => {
    for (const src of [
      'cynefin-beta\n"orphan"',
      'cynefin-beta\ncomplexity',
      'cynefin-beta\ncomplex --> nowhere',
      'cynefin-beta\ncomplex --> clear clear',
      'cynefin-betax',
    ]) {
      expect(() => render(src, options), src).toThrow(PelersError);
    }
  });
});
