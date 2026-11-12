import { describe, expect, it } from 'vitest';
import { PelersError, detectType, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus } from '../support/corpus.js';
import { assertWellFormed } from '../support/xml.js';

const corpus = loadCorpus('usecase', /usecase-beta/);
const options = { measurer: metricsMeasurer };

function tryRender(src: string): string | undefined {
  try {
    return render(src, options).svg;
  } catch (error) {
    if (error instanceof PelersError) return undefined;
    throw error;
  }
}

const svgOf = (src: string): string => render('usecase-beta\n' + src, options).svg;
const count = (svg: string, needle: string): number => svg.split(needle).length - 1;

// Where a node was put, from the translate of its group.
function at(svg: string, id: string): { x: number; y: number } {
  const m = new RegExp(
    `<g class="pelers-node [^"]*" data-id="${id}" transform="translate\\(([-\\d.]+),([-\\d.]+)\\)"`
  ).exec(svg);
  if (!m) throw new Error(`no node ${id}`);
  return { x: Number(m[1]), y: Number(m[2]) };
}

function nodeGroup(svg: string, id: string): string {
  const start = svg.indexOf(`data-id="${id}" transform=`);
  if (start === -1) throw new Error(`no node ${id}`);
  return svg.slice(svg.lastIndexOf('<g class="pelers-node', start), svg.indexOf('<g class="pelers-node', start + 1) >>> 0);
}

function ellipse(svg: string, id: string): { x: number; y: number; rx: number; ry: number } {
  const m = /<ellipse rx="([\d.]+)" ry="([\d.]+)"/.exec(nodeGroup(svg, id));
  if (!m) throw new Error(`no ellipse in ${id}`);
  return { ...at(svg, id), rx: Number(m[1]), ry: Number(m[2]) };
}

// The rectangles of a boundary: its body, and before it the title tab of a package.
function boundary(svg: string, id: string): { x: number; y: number; w: number; h: number }[] {
  const tag = svg.indexOf(`data-id="${id}">`);
  const group = svg.slice(svg.lastIndexOf('<g class="pelers-cluster', tag), svg.indexOf('</g>', tag));
  return [...group.matchAll(/<rect[^>]*? x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="([\d.]+)"/g)].map(
    (m) => ({
      x: Number(m[1]),
      y: Number(m[2]),
      w: Number(m[3]),
      h: Number(m[4]),
    })
  );
}

function edges(svg: string): { id: string; classes: string; d: string; body: string }[] {
  return [
    ...svg.matchAll(/<g class="pelers-edge ([^"]*)" data-id="([^"]*)"[^>]*><path d="([^"]+)"([^>]*)\/>(.*?)<\/g>/g),
  ].map((m) => ({
    classes: m[1],
    id: m[2],
    d: m[3],
    body: m[4] + m[5],
  }));
}

function ends(d: string): [number, number, number, number] {
  const nums = d.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
  return [nums[0], nums[1], nums[nums.length - 2], nums[nums.length - 1]];
}

describe('use case diagram rendering', () => {
  it('renders every corpus input that parses, as well-formed SVG', () => {
    let rendered = 0;
    for (const src of corpus) {
      const svg = tryRender(src);
      if (svg === undefined) continue;
      rendered++;
      const where = JSON.stringify(src).slice(0, 120);
      expect(() => assertWellFormed(svg), where).not.toThrow();
      expect(svg, where).not.toMatch(/<script|<style|<foreignObject|<defs|<marker|\son\w+=|javascript:/i);
      expect(svg, where).not.toContain('NaN');
      expect(svg, where).not.toContain('undefined');
      expect(tryRender(src), where).toBe(svg);
    }
    expect(rendered).toBeGreaterThan(60);
  });

  it('is detected, supported and typed', () => {
    const src = 'usecase-beta\nactor User\nUser --> Login';
    expect(detectType(src)).toBe('usecase');
    expect(supports(src)).toBe(true);
    const result = render(src, options);
    expect(result.type).toBe('usecase');
    expect(result.svg).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" class="pelers pelers-usecase"/);
    expect(result.svg).toContain(`viewBox="0 0 ${result.width} ${result.height}"`);
    expect(result.links).toEqual([]);
    const model = parse(src);
    expect(model.type).toBe('usecase');
    if (model.type === 'usecase') {
      expect([...model.actors.keys()]).toEqual(['User']);
      expect([...model.useCases.keys()]).toEqual(['Login']);
      expect(model.statements.map((s) => s.kind)).toEqual(['node', 'edge']);
    }
  });

  it('runs left to right unless told otherwise', () => {
    const chain = 'actor A\nA --> B\nB --> C';
    const wide = render('usecase-beta\n' + chain, options);
    expect(wide.width).toBeGreaterThan(wide.height);
    expect(at(wide.svg, 'A').x).toBeLessThan(at(wide.svg, 'B').x);
    const tall = render('usecase-beta\ndirection TB\n' + chain, options);
    expect(tall.height).toBeGreaterThan(tall.width);
    expect(at(tall.svg, 'A').y).toBeLessThan(at(tall.svg, 'B').y);
    expect(render('usecase-beta\ndirection TD\n' + chain, options).svg).toBe(tall.svg);
    const up = svgOf('direction BT\n' + chain);
    expect(at(up, 'A').y).toBeGreaterThan(at(up, 'B').y);
    const back = svgOf('direction RL\n' + chain);
    expect(at(back, 'A').x).toBeGreaterThan(at(back, 'B').x);
  });

  it('draws each kind of actor as a figure with its name below', () => {
    const svg = svgOf(
      'actor N("Normal one")\nactor H@{ type: hollow }\nactor W@{ type: awesome }\nactor I@{ icon: "fa:user" }'
    );
    const normal = nodeGroup(svg, 'N');
    expect(normal).toContain('class="pelers-node pelers-actor usecase-actor usecase-actor-normal"');
    expect(normal).toContain('<circle cy="-21" r="9"');
    expect(normal).toContain('M0,-12V10M-14,-3H14M0,10L-12,30M0,10L12,30');
    expect(normal).toContain('>Normal one</text>');
    const figureY = Number(/pelers-actor-figure" transform="translate\(0,([-\d.]+)\)/.exec(normal)![1]);
    const labelY = Number(/<text class="pelers-label" x="0" y="([-\d.]+)"/.exec(normal)![1]);
    expect(labelY).toBeGreaterThan(figureY + 30);

    const hollow = nodeGroup(svg, 'H');
    expect(hollow).toContain('usecase-actor-hollow');
    expect(count(hollow, 'fill="var(--_bg)"')).toBe(2);
    const awesome = nodeGroup(svg, 'W');
    expect(awesome).toContain('fill="var(--_b)"');
    expect(awesome).not.toContain('<circle');
    const icon = nodeGroup(svg, 'I');
    expect(icon).toContain('<svg class="pelers-icon" data-icon="fa:user"');
    expect(icon).toContain('<rect x="-22" y="-26" width="44" height="44"');
  });

  it('fills an icon actor from the icon resolver', () => {
    const { svg } = render('usecase-beta\nactor I@{ icon: "fa:user" }', {
      ...options,
      icons: (name) => (name === 'fa:user' ? '<path d="M1,1"/>' : null),
    });
    expect(svg).toContain('data-icon="fa:user"');
    expect(svg).toContain('stroke-linejoin="round"><path d="M1,1"/></svg>');
  });

  it('marks business actors and business use cases with a slash', () => {
    const svg = svgOf(
      'actor A@{ business: true }\nactor B@{ type: hollow, business: true }\nU(Use)@{ business: true }\nV(Plain)'
    );
    expect(count(nodeGroup(svg, 'A'), 'class="pelers-business"')).toBe(1);
    expect(count(nodeGroup(svg, 'B'), 'class="pelers-business"')).toBe(1);
    expect(count(nodeGroup(svg, 'U'), 'class="pelers-business"')).toBe(1);
    expect(count(nodeGroup(svg, 'V'), 'class="pelers-business"')).toBe(0);
    // The slash of a use case runs between two points on its oval.
    const { rx, ry } = ellipse(svg, 'U');
    const [x1, y1, x2, y2] = ends(/class="pelers-business" d="([^"]+)"/.exec(nodeGroup(svg, 'U'))![1]);
    for (const [x, y] of [
      [x1, y1],
      [x2, y2],
    ])
      expect((x / rx) ** 2 + (y / ry) ** 2).toBeCloseTo(1, 1);
    expect(y1).toBeGreaterThan(0);
    expect(y2).toBeLessThan(0);
  });

  it('draws use cases as ovals, and as boxes when written with brackets', () => {
    const svg = svgOf('Login("Sign in")\nReport[Generate report]\n"Reset password"');
    const oval = nodeGroup(svg, 'Login');
    expect(oval).toContain('class="pelers-node pelers-usecase usecase-element usecase-ellipse"');
    expect(count(oval, '<ellipse')).toBe(1);
    expect(oval).toContain('>Sign in</text>');
    const box = nodeGroup(svg, 'Report');
    expect(box).toContain('usecase-rect');
    expect(count(box, '<rect')).toBe(1);
    expect(count(box, '<ellipse')).toBe(0);
    expect(nodeGroup(svg, 'Reset_password')).toContain('>Reset password</text>');
    // The oval is wider than its text by more than the box is.
    const { rx } = ellipse(svg, 'Login');
    expect(rx * 2).toBeGreaterThan(metricsMeasurer.width('Sign in', 16, 0) + 30);
  });

  it('shows a stereotype above the name of a use case and between figure and name of an actor', () => {
    const svg = svgOf('actor A <<Human>>\nU(Use) <<Core>>');
    const useCase = nodeGroup(svg, 'U');
    const y = (group: string, cls: string): number =>
      Number(new RegExp(`<text class="${cls}"[^>]* y="([-\\d.]+)"`).exec(group)![1]);
    expect(useCase).toContain('>«Core»</text>');
    expect(y(useCase, 'pelers-stereotype')).toBeLessThan(y(useCase, 'pelers-label'));
    const actor = nodeGroup(svg, 'A');
    expect(actor).toContain('>«Human»</text>');
    expect(y(actor, 'pelers-stereotype')).toBeLessThan(y(actor, 'pelers-label'));
    expect(y(actor, 'pelers-stereotype')).toBeGreaterThan(0);
  });

  it('draws a boundary around its members and leaves the rest outside', () => {
    const svg = svgOf('actor User\nsystemBoundary Shop["The shop"]\n  Browse\n  actor Clerk\nend\nUser --> Browse');
    const [box] = boundary(svg, 'Shop');
    expect(svg).toContain('<g class="pelers-cluster pelers-boundary system-boundary system-boundary-rect" data-id="Shop">');
    expect(svg).toContain('>The shop</text>');
    const inside = (id: string): boolean => {
      const p = at(svg, id);
      return p.x > box.x && p.x < box.x + box.w && p.y > box.y && p.y < box.y + box.h;
    };
    expect(inside('Browse')).toBe(true);
    expect(inside('Clerk')).toBe(true);
    expect(inside('User')).toBe(false);
    // The actor outside comes before the boundary, on the side its relationship starts from.
    expect(at(svg, 'User').x).toBeLessThan(box.x);
  });

  it('draws a boundary fill the author sets at full opacity', () => {
    const svg = svgOf('systemBoundary S\n  U\nend\nsystemBoundary T\n  V\nend\nstyle S fill:#fee');
    const body = (id: string) =>
      svg
        .slice(svg.indexOf(`data-id="${id}">`))
        .match(/<rect [^>]*>/g)!
        .find((r) => r.includes('fill="var(--_a)"'))!;
    expect(body('S')).not.toContain('fill-opacity');
    expect(body('T')).toContain('fill-opacity="0.5"');
  });

  it('picks a contrasting text color on fills a style sets', () => {
    const svg = svgOf(
      'actor A <<Human>>\nU(Use) <<Core>>\nW(Own)\nX(Plain)\njson J@{ "k": "v" }\nsystemBoundary S\n  V\nend\nstyle U fill:#1f3a5f\nstyle A fill:#1f3a5f\nstyle W fill:#1f3a5f,color:#eee\nstyle J fill:navy\nstyle S fill:#FFF9C4'
    );
    const node = (id: string): string => new RegExp(`data-id="${id}"[^>]*>(.*?)</g>(?:<g|</g>)`).exec(svg)![1];
    expect(node('U')).toMatch(/<text class="pelers-stereotype"[^>]* fill="var\(--pelers-fg-on-dark,#fff\)"/);
    expect(node('U')).toMatch(/<text class="pelers-label" fill="var\(--pelers-fg-on-dark,#fff\)"[^>]*>Use</);
    expect(svg).toMatch(/data-id="A"[^>]*>.*?<text class="pelers-stereotype"[^>]* fill="var\(--_m\)"/);
    expect(svg).toMatch(/data-id="A"[^>]*>.*?<text class="pelers-label" x=[^>]*>A</);
    expect(node('W')).not.toContain('--pelers-fg-on');
    expect(node('J')).toMatch(/<text class="pelers-json-key" fill="var\(--pelers-fg-on-dark,#fff\)"/);
    expect(node('J')).toMatch(/<text class="pelers-json-value" fill="var\(--pelers-fg-on-dark,#fff\)"/);
    expect(svg).toMatch(/data-id="S">.*?<text class="pelers-cluster-label" fill="var\(--pelers-fg-on-light,#1f1f1f\)"/);
    expect(node('X')).not.toContain('--pelers-fg-on');
  });

  it('cases edges that cross a boundary fill a style sets and inks those inside it', () => {
    const src =
      'actor User\nsystemBoundary S\n  Pay\n  Ship\nend\nPay --> Ship\nUser --> Pay\nUser --> Other\nstyle S fill:#BBDEFB';
    const svg = svgOf(src);
    const casings = /<g class="pelers-casings"[^>]*>(.*?)<\/g>/.exec(svg)![1];
    const edges = [...svg.matchAll(/class="pelers-edge [^"]*" data-id="[^"]+"([^>]*)><path d="([^"]+)"/g)].map((m) => [
      m[1],
      m[2],
    ]);
    expect(edges).toHaveLength(3);
    // Pay --> Ship lies inside the boundary and takes its ink; User --> Pay crosses into it.
    expect(edges.map(([, d]) => casings.includes(`<path d="${d}"/>`))).toEqual([false, true, false]);
    expect(edges.map(([attrs]) => attrs)).toEqual([' stroke="var(--pelers-fg-on-light,#1f1f1f)"', '', '']);
    expect(svgOf(src.replace('\nstyle S fill:#BBDEFB', ''))).not.toMatch(/pelers-casings|stroke="var\(--pelers-fg-on/);
    // A hollow triangle keeps the fill's color inside it.
    const hollow = svgOf('systemBoundary S\n  Pay\n  Ship\nend\nPay --|> Ship\nstyle S fill:black');
    expect(hollow).toMatch(/class="pelers-marker"[^>]* fill="black" stroke="var\(--pelers-fg-on-dark,#fff\)"/);
  });

  it('puts an actor that is pointed at on the far side of the boundary', () => {
    const svg = svgOf('actor User\nactor Bank\nsystemBoundary Shop\n  Pay\nend\nUser --> Pay\nPay --> Bank');
    const [box] = boundary(svg, 'Shop');
    expect(at(svg, 'User').x).toBeLessThan(box.x);
    expect(at(svg, 'Bank').x).toBeGreaterThan(box.x + box.w);
  });

  it('gives a package boundary a tab that holds its title', () => {
    const svg = svgOf('systemBoundary P["Payments"]@{ type: package }\n  Pay\nend');
    const [tab, body] = boundary(svg, 'P');
    expect(svg).toContain('system-boundary-package');
    expect(svg).toContain('class="pelers-boundary-tab"');
    expect(tab.x).toBe(body.x);
    expect(tab.y + tab.h).toBeCloseTo(body.y, 1);
    expect(tab.w).toBeLessThan(body.w);
    const title = /<text class="pelers-cluster-label"[^>]* x="([-\d.]+)" y="([-\d.]+)"/.exec(svg)!;
    expect(Number(title[1])).toBeGreaterThan(tab.x);
    expect(Number(title[1])).toBeLessThan(tab.x + tab.w);
    expect(Number(title[2])).toBeLessThan(body.y);
    expect(at(svg, 'Pay').y).toBeGreaterThan(body.y);
  });

  it('draws the marker each association asks for', () => {
    const svg = svgOf('A a@--> B\nA b@<-- B\nA c@-- B\nA d@--o B\nA e@o-- B\nA f@--x B\nA g@x-- B');
    const byId = new Map(edges(svg).map((e) => [e.id, e.body]));
    const kinds = (id: string): string[] =>
      [...byId.get(id)!.matchAll(/<(path|circle) class="pelers-marker"/g)].map((m) => m[1]);
    expect(kinds('a')).toEqual(['path']);
    expect(kinds('b')).toEqual(['path']);
    expect(kinds('c')).toEqual([]);
    expect(kinds('d')).toEqual(['circle']);
    expect(kinds('e')).toEqual(['circle']);
    expect(kinds('f')).toEqual(['path']);
    expect(kinds('g')).toEqual(['path']);
    expect(byId.get('a')).toContain('stroke="none"');
    // A cross is two strokes, not a filled head.
    expect(byId.get('f')).not.toContain('stroke="none"');
    for (const edge of edges(svg)) expect(edge.body).not.toContain('stroke-dasharray');
  });

  it('puts a forward marker at the target and a backward one at the source', () => {
    const svg = svgOf('A a@--> B\nC b@<-- D');
    const tip = (id: string): number => {
      const body = edges(svg).find((e) => e.id === id)!.body;
      return Number(/class="pelers-marker" d="M([-\d.]+),/.exec(body)![1]);
    };
    expect(Math.abs(tip('a') - at(svg, 'B').x)).toBeLessThan(Math.abs(tip('a') - at(svg, 'A').x));
    expect(Math.abs(tip('b') - at(svg, 'C').x)).toBeLessThan(Math.abs(tip('b') - at(svg, 'D').x));
  });

  it('draws include and extend dashed, with an arrow and their name in guillemets', () => {
    const svg = svgOf('A i@..> : include B\nC e@..> : EXTEND D');
    const [include, extend] = edges(svg);
    expect(include.classes).toBe('pelers-include');
    expect(extend.classes).toBe('pelers-extend');
    for (const edge of [include, extend]) {
      expect(edge.body).toContain('stroke-dasharray="5 4"');
      expect(count(edge.body, 'class="pelers-marker"')).toBe(1);
    }
    expect(svg).toContain('>«include»</text>');
    expect(svg).toContain('>«extend»</text>');
    expect(count(svg, 'class="pelers-edge-label"')).toBe(2);
  });

  it('draws a generalization as a solid line with a hollow triangle', () => {
    const svg = svgOf('actor Admin\nactor Person\nAdmin --|> Person');
    const [edge] = edges(svg);
    expect(edge.classes).toBe('pelers-generalization');
    expect(edge.body).not.toContain('stroke-dasharray');
    expect(edge.body).toMatch(
      /<path class="pelers-marker" d="M[-\d.]+,[-\d.]+L[-\d.]+,[-\d.]+L[-\d.]+,[-\d.]+Z" fill="var\(--_bg\)" stroke="var\(--_l\)"/
    );
    // The line stops at the base of the triangle, not at its tip.
    const [, , ex] = ends(edge.d);
    const tip = Number(/class="pelers-marker" d="M([-\d.]+),/.exec(edge.body)![1]);
    expect(tip - ex).toBeCloseTo(12, 0);
  });

  it('labels an association and keeps the label off the nodes', () => {
    const svg = svgOf('actor User\nUser -- "starts a session" --> Login');
    expect(svg).toContain('>starts a session</text>');
    const m = /<g class="pelers-edge-label" data-id="edge-0"><rect x="([-\d.]+)" y="[-\d.]+" width="([\d.]+)"/.exec(svg)!;
    const left = Number(m[1]);
    const right = left + Number(m[2]);
    const login = ellipse(svg, 'Login');
    expect(left).toBeGreaterThan(at(svg, 'User').x + 20);
    expect(right).toBeLessThan(login.x - login.rx);
  });

  it('ends relationships on the outline of an oval and beside the figure of an actor', () => {
    const svg = svgOf('actor User\nUser --> Login\nUser --> Logout\nOther -- Login');
    for (const edge of edges(svg)) {
      const [sx, sy, ex, ey] = ends(edge.d);
      const target = edge.id === 'edge-1' ? 'Logout' : 'Login';
      const { x, y, rx, ry } = ellipse(svg, target);
      // An arrowhead takes up the last 7 units before the outline.
      const reach = edge.id === 'edge-2' ? 0 : 7;
      const onOutline = ((ex + reach - x) / rx) ** 2 + ((ey - y) / ry) ** 2;
      expect(onOutline, edge.id).toBeGreaterThan(0.85);
      expect(onOutline, edge.id).toBeLessThan(1.15);
      if (edge.id !== 'edge-2') {
        const user = at(svg, 'User');
        expect(sx - user.x).toBeCloseTo(23, 0);
        // At the height of the figure, above the middle of figure and name together.
        expect(sy).toBeLessThan(user.y);
      }
    }
  });

  it('spreads the relationships that meet on one side of a use case', () => {
    const svg = svgOf('actor A\nactor B\nactor C\nA --> U\nB --> U\nC --> U');
    const ys = edges(svg).map((e) => ends(e.d)[3]);
    expect(new Set(ys.map((y) => Math.round(y))).size).toBe(3);
    const sorted = [...ys].sort((a, b) => a - b);
    expect(sorted[1] - sorted[0]).toBeGreaterThan(6);
  });

  it('gives longer operators more ranks', () => {
    const near = svgOf('A --> B');
    const far = svgOf('A ----> B');
    expect(at(far, 'B').x - at(far, 'A').x).toBeGreaterThan(at(near, 'B').x - at(near, 'A').x + 100);
  });

  it('draws a relationship from a use case to itself as a loop with its label beside it', () => {
    for (const dir of ['LR', 'TB']) {
      const svg = svgOf(`direction ${dir}\nLogin loop@..> : include Login`);
      const [edge] = edges(svg);
      expect(edge.d).toMatch(/^M[-\d.,]+C[-\d., ]+$/);
      const { x, y, rx, ry } = ellipse(svg, 'Login');
      const label = /<g class="pelers-edge-label" data-id="loop"><rect x="([-\d.]+)" y="([-\d.]+)"/.exec(svg)!;
      if (dir === 'LR') expect(Number(label[2])).toBeGreaterThan(y + ry);
      else expect(Number(label[1])).toBeGreaterThan(x + rx);
      expect(svg).not.toContain('NaN');
    }
  });

  it('attaches a note to its target with a dotted line and no marker', () => {
    const svg = svgOf(
      'actor User\nnote for User "Starts the workflow"\nnote for Login "`Needs a **session**`"\nUser --> Login'
    );
    expect(count(svg, 'class="pelers-node pelers-note usecase-note"')).toBe(2);
    expect(svg).toContain('>Starts the workflow</text>');
    expect(svg).toContain('<tspan font-weight="var(--_w)">session</tspan>');
    const connectors = edges(svg).filter((e) => e.classes === 'pelers-note');
    expect(connectors.map((e) => e.id)).toEqual(['note-0-edge', 'note-1-edge']);
    for (const connector of connectors) {
      expect(connector.body).toContain('stroke-dasharray="2 3"');
      expect(connector.body).not.toContain('pelers-marker');
    }
  });

  it('draws a JSON node as a table of its leaves in written order', () => {
    const svg = svgOf(
      'json Payload@{\n  "2": "second",\n  "1": "first",\n  "colors": ["Red", "Green"],\n  "address": { "city": "Oslo" },\n  "none": null,\n  "empty": {}\n}'
    );
    const table = nodeGroup(svg, 'Payload');
    expect(table).toContain('class="pelers-node pelers-json usecase-json-table"');
    expect(table).toMatch(
      /<text class="pelers-label" font-weight="var\(--_hw\)"[^>]*><tspan x="0" y="-95.9">Payload<\/tspan>/
    );
    const keys = [...table.matchAll(/<text class="pelers-json-key"[^>]*>([^<]*)<\/text>/g)].map((m) => m[1]);
    const values = [...table.matchAll(/<text class="pelers-json-value"[^>]*>([^<]*)<\/text>/g)].map((m) => m[1]);
    // The second value of an array has no key of its own.
    expect(keys).toEqual(['2', '1', 'colors', 'address.city', 'none', 'empty']);
    expect(values).toEqual(['second', 'first', 'Red', 'Green', 'Oslo', 'null', '{}']);
    // Keys line up in one column and values in another.
    const xs = (cls: string): number[] =>
      [...table.matchAll(new RegExp(`<text class="${cls}"[^>]* x="([-\\d.]+)"`, 'g'))].map((m) => Number(m[1]));
    const valueX = xs('pelers-json-value');
    const lefts = valueX.map((x, i) => x - metricsMeasurer.width(values[i], 14, 0) / 2);
    for (const left of lefts) expect(left).toBeCloseTo(lefts[0], 1);
  });

  it('shows plain labels as written and formats markdown labels', () => {
    const svg = svgOf(
      'A("*n* <b>x</b> \\n &amp;")\nQ("a #quot;q#quot; #40;b#41;")\nB("`**bold** and *italic*`")\nC(*literal*)'
    );
    expect(nodeGroup(svg, 'A')).toContain('>*n* &lt;b&gt;x&lt;/b&gt; \\n &amp;amp;</text>');
    expect(nodeGroup(svg, 'Q')).toContain('>a &quot;q&quot; (b)</text>');
    const formatted = nodeGroup(svg, 'B');
    expect(formatted).toContain(' font-weight="var(--_w)">bold</tspan>');
    expect(formatted).toContain('<tspan font-style="italic">italic</tspan>');
    expect(nodeGroup(svg, 'C')).toContain('>*literal*</text>');
  });

  it('breaks a markdown label where the source breaks', () => {
    const svg = svgOf('A("`first\nsecond`")');
    const label = nodeGroup(svg, 'A');
    expect(count(label, '<tspan')).toBe(2);
    expect(label).toContain('>first</tspan>');
    expect(label).toContain('>second</tspan>');
  });

  it('applies classes and styles to nodes, boundaries and explicit edges', () => {
    const svg = svgOf(
      'actor A:::hot\nU(Use)\nsystemBoundary S\n  V\nend\nA e@--> U\nclassDef hot fill:#fee,stroke:#c33,color:#900\nclassDef default stroke-width:2px\nclass U,S,e hot\nstyle U stroke:#06c\nstyle e stroke-width:4px'
    );
    const actor = nodeGroup(svg, 'A');
    expect(actor).toContain('usecase-actor-normal hot"');
    expect(actor).toContain('style="stroke-width:2px;fill:#fee;stroke:#c33;"');
    expect(actor).toContain('style="fill:#900;"');
    // A direct style beats the class, which beats the default class.
    expect(nodeGroup(svg, 'U')).toContain(
      '<ellipse rx="44" ry="26" fill="var(--_s)" stroke="var(--_b)" style="stroke-width:2px;fill:#fee;stroke:#06c;"/>'
    );
    expect(svg).toMatch(
      /<g class="pelers-cluster pelers-boundary system-boundary system-boundary-rect hot" data-id="S"><rect[^>]* style="rx:var\(--_r\);stroke-width:2px;fill:#fee;stroke:#c33;"/
    );
    expect(svg).toContain(
      '<g class="pelers-edge pelers-association hot" data-id="e" style="stroke-width:4px;stroke:#c33;">'
    );
    expect(edges(svg)[0].body).toContain('fill="#c33" stroke="none"');
  });

  it('keeps the dashes of an animated relationship, without motion', () => {
    const svg = svgOf('A e@--> B\ne@{ animation: slow }\nA f@--> B\nf@{ animate: false }');
    const [animated, still] = edges(svg);
    expect(animated.classes).toBe('pelers-association edge-animation-slow');
    expect(animated.body).toContain('stroke-dasharray="9 5"');
    expect(still.body).not.toContain('stroke-dasharray');
  });

  it('draws the front matter title and the accessible names', () => {
    const { svg } = render(
      '---\ntitle: Ordering\n---\nusecase-beta\naccTitle: Online ordering\naccDescr {\n  A customer orders.\n   Staff review.\n}\nactor A',
      options
    );
    expect(svg).toContain('<text class="pelers-title" font-weight="var(--_tw)"');
    expect(svg).toContain('>Ordering</tspan>');
    expect(svg).toContain('<title id="pelers-title">Online ordering</title>');
    expect(svg).toContain('<desc id="pelers-desc">A customer orders.\nStaff review.</desc>');
  });

  it('draws nothing but the frame for a diagram with no statements', () => {
    const result = render('usecase-beta', options);
    expect(result.svg).toContain('<g class="pelers-nodes"></g>');
    expect(result.width).toBeGreaterThan(0);
  });

  it('reports syntax, lexer and semantic errors as Mermaid words them', () => {
    const message = (src: string): string => {
      try {
        render(src, options);
      } catch (error) {
        expect(error).toBeInstanceOf(PelersError);
        return (error as PelersError).code + ': ' + (error as Error).message;
      }
      return 'no error';
    };
    expect(message('usecase-beta\nactor A actor B')).toBe(
      "syntax: Error parsing usecase diagram: Expecting: one of these possible Token sequences:\n  1. [NEWLINE]\n  2. [EOF]\nbut found: 'actor' at line 2, column 9 [21,26)"
    );
    expect(message('usecase-beta\nsystemBoundary S\nA --> B\nend')).toBe(
      "syntax: Error parsing usecase diagram: Expecting: one of these possible Token sequences:\n  1. [NEWLINE]\n  2. [EOF]\nbut found: '-->' at line 3, column 3 [32,35)"
    );
    expect(message('usecase-beta\nsystemBoundary S\nA')).toBe(
      "syntax: Error parsing usecase diagram: Expecting token of type --> END <-- but found --> '' <-- at line 4, column 1 [32,32)"
    );
    expect(message('usecase-beta\nA "b')).toBe(
      'syntax: Error lexing usecase diagram: unexpected character: ->"<- at offset: 15, skipped 1 characters. at line 2, column 3 [15,16)'
    );
    expect(message('usecase-beta\nnote for Ghost "x"')).toBe(
      "semantic: Note target 'Ghost' is unresolved at line 2, column 10 [22,27)"
    );
    expect(message('usecase-beta\njson P@{"a":}')).toMatch(/^syntax: Invalid JSON: .* \(line 2, column \d+\)$/);
    expect(() => render('usecase-beta\njson P@{"a":"#35;", "b":}', options)).toThrow(
      expect.objectContaining({ line: 2, column: 25, message: expect.not.stringMatching(/[ﬂ°¶ß]/) })
    );
    const located = (() => {
      try {
        render('usecase-beta\nactor A\nA --> B\nstyle Z fill:red', options);
      } catch (error) {
        return error as PelersError;
      }
      return undefined;
    })();
    expect(located).toMatchObject({ code: 'semantic', type: 'usecase', line: 4, column: 7 });
  });
});
