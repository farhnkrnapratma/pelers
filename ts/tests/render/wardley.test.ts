import { describe, expect, it } from 'vitest';
import { PelersError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const options = { measurer: metricsMeasurer };
const count = (svg: string, re: RegExp): number => svg.match(re)?.length ?? 0;

const TEA =
  'wardley-beta\ntitle Tea Shop\nanchor Business [0.95, 0.63]\ncomponent Cup of Tea [0.79, 0.61]\ncomponent Tea [0.63, 0.81]\n' +
  'component Kettle [0.43, 0.35]\ncomponent Power [0.10, 0.70]\nBusiness -> Cup of Tea\nCup of Tea -> Tea\nCup of Tea -> Kettle\nKettle -> Power\n' +
  'evolve Kettle 0.62\nevolve Power 0.89\nnote "Standardising power" [0.30, 0.49]\n';

describe('wardley rendering', () => {
  it('draws axes, stages, components, links, evolution, and notes', () => {
    const { svg, type } = render(TEA, options);
    expect(type).toBe('wardley');
    expect(supports(TEA)).toBe(true);
    expect(svg).toContain('class="pelers-wardley-axis"');
    expect(count(svg, /class="pelers-wardley-stage"/g)).toBe(3);
    for (const stage of ['Genesis', 'Custom Built', 'Product', 'Commodity', 'Evolution', 'Visibility'])
      expect(svg).toContain(`>${stage}<`);
    expect(count(svg, /class="pelers-node pelers-wardley-component"/g)).toBe(4);
    expect(count(svg, /class="pelers-node pelers-wardley-anchor"/g)).toBe(1);
    expect(count(svg, /class="pelers-edge"/g)).toBe(4);
    expect(count(svg, /class="pelers-wardley-trend"/g)).toBe(2);
    expect(svg).toContain('class="pelers-wardley-note"');
    expect(svg).toContain('>Tea Shop<');
    expect(svg).toContain('data-id="Cup of Tea"');
    expect(svg).not.toContain('NaN');
  });

  it('places components by evolution across and visibility up', () => {
    const at = (src: string, id: string): [number, number] => {
      const m = new RegExp(`data-id="${id}"><circle cx="([\\d.]+)" cy="([\\d.]+)"`).exec(render(src, options).svg)!;
      return [Number(m[1]), Number(m[2])];
    };
    const src = 'wardley-beta\ncomponent Low [0.1, 0.2]\ncomponent High [0.9, 0.8]\ncomponent Percent [90.0, 80.0]\n';
    const low = at(src, 'Low');
    const high = at(src, 'High');
    expect(high[0]).toBeGreaterThan(low[0]);
    expect(high[1]).toBeLessThan(low[1]);
    expect(at(src, 'Percent')).toEqual(high);
  });

  it('exposes the model in percentages', () => {
    const model = parse(TEA);
    expect(model.type).toBe('wardley');
    if (model.type !== 'wardley') return;
    expect(model.nodes.find((n) => n.id === 'Kettle')).toMatchObject({ x: 35, y: 43, className: 'component' });
    expect(model.nodes[0]).toMatchObject({ id: 'Business', className: 'anchor' });
    expect(model.links).toHaveLength(4);
    expect(model.trends).toEqual([
      { nodeId: 'Kettle', targetX: 62, targetY: 43 },
      { nodeId: 'Power', targetX: 89, targetY: 10 },
    ]);
  });

  it('draws custom stages, dual labels, and stage widths', () => {
    const { svg } = render(
      'wardley-beta\nevolution Genesis@0.3 / Concept -> Custom@0.5 -> Product@0.9 -> Utility@1.0\ncomponent A [0.5, 0.5]\n',
      options
    );
    expect(svg).toContain('Genesis / Concept');
    expect(svg).toContain('>Utility<');
    const xs = [...svg.matchAll(/class="pelers-wardley-stage" d="M([\d.]+),/g)].map((m) => Number(m[1]));
    expect(xs).toHaveLength(3);
    expect(xs[0] / xs[1]).toBeCloseTo(0.6, 5);
  });

  it('draws sourcing strategies and inertia', () => {
    const { svg } = render(
      'wardley-beta\ncomponent A [0.9, 0.1] (build)\ncomponent B [0.7, 0.4] (buy) inertia\ncomponent C [0.5, 0.7] (outsource)\ncomponent D [0.3, 0.9] (market) (inertia)\n',
      options
    );
    for (const strategy of ['build', 'buy', 'outsource', 'market'])
      expect(svg).toContain(`class="pelers-wardley-${strategy}"`);
    expect(count(svg, /class="pelers-wardley-inertia"/g)).toBe(2);
  });

  it('draws flows, dashed links, and link labels', () => {
    const src =
      "wardley-beta\ncomponent A [0.9, 0.1]\ncomponent B [0.5, 0.5]\ncomponent C [0.1, 0.9]\nA +> B\nB +< C\nA +<> C\nA -.-> B; note here\nB +'cash'> C\nA -> Missing\nA -> A\n";
    const { svg } = render(src, options);
    expect(count(svg, /class="pelers-edge[ "]/g)).toBe(5);
    expect(count(svg, /class="pelers-marker"/g)).toBe(5);
    expect(count(svg, /class="pelers-edge pelers-wardley-dashed"/g)).toBe(1);
    expect(svg).toContain('>note here</text>');
    expect(svg).toContain('>cash</text>');
  });

  it('draws pipelines around their components and moves the parent onto the box', () => {
    const src =
      'wardley-beta\ncomponent Kettle [0.45, 0.57]\ncomponent Water [0.8, 0.8]\npipeline Kettle {\n  component Campfire Kettle [0.35]\n  component Electric Kettle [0.53]\n}\nWater -> Electric Kettle\nCampfire Kettle -> Kettle\n';
    const { svg } = render(src, options);
    expect(count(svg, /class="pelers-wardley-pipeline"/g)).toBe(1);
    expect(count(svg, /class="pelers-node pelers-wardley-pipeline-component"/g)).toBe(2);
    expect(count(svg, /class="pelers-node pelers-wardley-pipeline-parent"/g)).toBe(1);
    expect(svg).toContain('data-id="Water-Kettle_Electric Kettle"');
    expect(count(svg, /class="pelers-edge"/g)).toBe(1);
    const model = parse(src);
    if (model.type === 'wardley')
      expect(model.pipelines).toEqual([
        { nodeId: 'Kettle', componentIds: ['Kettle_Campfire Kettle', 'Kettle_Electric Kettle'] },
      ]);
  });

  it('draws annotations, listing their texts in the box or under the map', () => {
    const body = 'annotation 2,[0.5, 0.5] "Second"\nannotation 1,[0.6, 0.65] "First"\n';
    const boxed = render('wardley-beta\nannotations [0.9, 0.1]\n' + body, options);
    const below = render('wardley-beta\n' + body, options);
    expect(count(boxed.svg, /class="pelers-wardley-annotation"/g)).toBe(2);
    expect(boxed.svg.indexOf('1. First')).toBeLessThan(boxed.svg.indexOf('2. Second'));
    expect(below.svg).toContain('2. Second');
    expect(below.height).toBeGreaterThan(boxed.height);
  });

  it('moves a label out of the way of a link that leaves up and to the right', () => {
    const labelX = (src: string): number =>
      Number(/data-id="A">.*?<text[^>]*><tspan x="([\d.-]+)"/.exec(render(src, options).svg)![1]);
    const alone = labelX('wardley-beta\ncomponent A [0.5, 0.5]\ncomponent B [0.2, 0.2]\nA -> B\n');
    const blocked = labelX(
      'wardley-beta\ncomponent A [0.5, 0.5]\ncomponent B [0.8, 0.8]\ncomponent C [0.2, 0.8]\nA -> B\nA -> C\n'
    );
    const moved = labelX('wardley-beta\ncomponent A [0.5, 0.5] label [30, 30]\ncomponent B [0.8, 0.8]\nA -> B\n');
    expect(blocked).toBeLessThan(alone);
    expect(moved).toBe(alone + 22);
  });

  it('draws accelerators and deaccelerators', () => {
    const { svg } = render(
      'wardley-beta\naccelerator "AI Adoption" [0.55, 0.25]\ndeaccelerator Legacy [0.15, 0.75]\n',
      options
    );
    expect(svg).toContain('class="pelers-wardley-accelerator" data-id="AI Adoption"');
    expect(svg).toContain('class="pelers-wardley-deaccelerator" data-id="Legacy"');
  });

  it('honours size, label offsets, and the grid option, and keeps labels inside the picture', () => {
    const base = render('wardley-beta\ncomponent A [0.5, 0.5]\n', options);
    const sized = render('wardley-beta\nsize [1100, 800]\ncomponent A [0.5, 0.5]\n', options);
    expect(sized.width - base.width).toBe(200);
    expect(sized.height - base.height).toBe(200);
    const edge = render('wardley-beta\ncomponent A long label at the right edge [0.5, 1.0]\n', options);
    expect(edge.width).toBeGreaterThan(base.width + 100);
    const moved = render('wardley-beta\ncomponent A [0.5, 0.5] label [-40, 30]\n', options).svg;
    expect(moved).not.toBe(base.svg);
    expect(
      render('---\nconfig:\n  wardley-beta:\n    showGrid: true\n---\nwardley-beta\ncomponent A [0.5, 0.5]\n', options)
        .svg
    ).toContain('pelers-wardley-grid');
    expect(base.svg).not.toContain('pelers-wardley-grid');
  });

  it('draws titles and accessible names', () => {
    const { svg } = render('---\ntitle: Front\n---\nwardley-beta\naccTitle: Acc\naccDescr: Described\n', options);
    expect(svg).toContain('>Front<');
    expect(svg).toContain('<title id="pelers-title">Acc</title>');
    expect(svg).toContain('<desc id="pelers-desc">Described</desc>');
    expect(render('---\ntitle: Front\n---\nwardley-beta\ntitle Own\n', options).svg).toContain('>Own<');
  });

  it("rejects what Mermaid rejects, with Mermaid's messages", () => {
    expect(() => render('wardley-beta\ncomponent A [150.5, 0.5]\n', options)).toThrow(
      'Component "A" visibility must be between 0-1 (decimal) or 0-100 (percentage). Received: 150.5'
    );
    expect(() => render('wardley-beta\npipeline Nope {\n component A [0.5]\n}\n', options)).toThrow(
      'Pipeline "Nope" must reference an existing component with coordinates.'
    );
    for (const src of [
      'wardley-beta\ncomponent market [0.1, 0.2]',
      'wardley-beta\ncomponent A [1, 2]',
      'wardley-beta\nA B',
      'wardley-beta\nnote text [0.1, 0.2]',
    ]) {
      expect(() => render(src, options), src).toThrow(PelersError);
    }
  });
});
