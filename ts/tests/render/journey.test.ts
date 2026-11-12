import { describe, expect, it } from 'vitest';
import { PelersError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { elements } from '../support/xml.js';

const options = { measurer: metricsMeasurer };
const count = (svg: string, cls: string): number => svg.split(`class="${cls}"`).length - 1;
const texts = (svg: string): string[] => [...svg.matchAll(/>([^<>]+)</g)].map((m) => m[1]);
const faceY = (svg: string): number[] =>
  [...svg.matchAll(/class="pelers-face pelers-face-\w+" transform="translate\([-\d.]+,([-\d.]+)\)"/g)].map((m) =>
    Number(m[1])
  );
const moods = (svg: string): string[] => [...svg.matchAll(/class="pelers-face pelers-face-(\w+)"/g)].map((m) => m[1]);

const DAY =
  'journey\n  title My working day\n  section Go to work\n    Make tea: 5: Me\n    Go upstairs: 3: Me\n    Do work: 1: Me, Cat\n  section Go home\n    Go downstairs: 5: Me\n    Sit down: 5: Me\n';

describe('journey rendering', () => {
  it('draws a title, section bands, a card and a face per task, and a legend of actors', () => {
    const { svg, type, links } = render(DAY, options);
    expect(type).toBe('journey');
    expect(supports(DAY)).toBe(true);
    expect(links).toEqual([]);
    expect(svg).toContain('class="pelers pelers-journey"');
    expect(svg).toContain('>My working day<');
    expect(count(svg, 'pelers-cluster pelers-section')).toBe(2);
    expect(count(svg, 'pelers-node pelers-task')).toBe(5);
    expect(moods(svg)).toEqual(['happy', 'neutral', 'sad', 'happy', 'happy']);
    expect(count(svg, 'pelers-actor')).toBe(6);
    expect(count(svg, 'pelers-legend-item')).toBe(2);
    expect(svg).toContain('data-id="Go to work"');
    expect(svg).toContain('data-id="Make tea"');
    expect(svg).toContain('>Do work<');
  });

  it('places each face by its score', () => {
    const ys = faceY(render('journey\n  a: 5\n  b: 4\n  c: 3\n  d: 2\n  e: 1\n', options).svg);
    expect(ys.length).toBe(5);
    for (let i = 1; i < 5; i++) expect(ys[i] - ys[i - 1]).toBe(ys[1] - ys[0]);
    expect(ys[1]).toBeGreaterThan(ys[0]);
  });

  it('keeps odd scores on the scale: out of range is clamped, not a number is neutral', () => {
    const { svg } = render(
      'journey\n  top: 5\n  mid: 3\n  low: 1\n  big: 100\n  neg: -7\n  zero: 0\n  word: abc\n  blank: \n  half: 3.5\n  inf: Infinity\n',
      options
    );
    const ys = faceY(svg);
    const [top, mid, low] = ys;
    expect(ys.slice(3)).toEqual([top, low, low, mid, low, (top + mid) / 2 + (mid - top) / 4, top]);
    expect(moods(svg)).toEqual(['happy', 'neutral', 'sad', 'happy', 'sad', 'sad', 'neutral', 'sad', 'happy', 'happy']);
    expect(svg).not.toMatch(/NaN|Infinity/);
  });

  it('draws faces from circles and paths only', () => {
    const { svg } = render('journey\n  a: 5\n  b: 3\n  c: 1\n', options);
    const names = new Set(elements(svg).map((el) => el.name));
    expect([...names].sort()).toEqual(['circle', 'g', 'path', 'rect', 'svg', 'text']);
    const mouths = [
      ...svg.matchAll(/<path d="([^"]+)" fill="none" stroke="var\(--_l\)" stroke-linecap="round"\/>/g),
    ].map((m) => m[1]);
    expect(new Set(mouths).size).toBe(3);
  });

  it('colors actors by their place in the sorted list and marks them on each task', () => {
    const { svg } = render(DAY, options);
    const legend = [
      ...svg.matchAll(/class="pelers-legend-item" data-id="(\w+)"><circle [^>]*fill="var\(--pelers-series-(\d)/g),
    ].map((m) => [m[1], m[2]]);
    expect(legend).toEqual([
      ['Cat', '1'],
      ['Me', '2'],
    ]);
    const dots = [
      ...svg.matchAll(/class="pelers-actor"[^>]*fill="var\(--pelers-series-(\d),[^>]*><title>(\w+)<\/title>/g),
    ].map((m) => [m[2], m[1]]);
    expect(dots).toEqual([
      ['Me', '2'],
      ['Me', '2'],
      ['Me', '2'],
      ['Cat', '1'],
      ['Me', '2'],
      ['Me', '2'],
    ]);
  });

  it('leaves out the nameless actor of a task that ends in a colon', () => {
    const { svg } = render('journey\n  a: 5:\n  b: 3: , Real,\n', options);
    expect(count(svg, 'pelers-actor')).toBe(1);
    expect(count(svg, 'pelers-legend-item')).toBe(1);
    expect(render('journey\n  a: 5:\n', options).svg).not.toContain('pelers-legend');
  });

  it('spans each section band over its run of tasks, and draws no band for empty or missing sections', () => {
    const { svg } = render(
      'journey\n  first: 3\n  section A\n  section B\n    b1: 3\n    b2: 3\n  section C\n    c1: 3\n  section D\n',
      options
    );
    expect(count(svg, 'pelers-cluster pelers-section')).toBe(2);
    expect(svg).not.toContain('>A<');
    expect(svg).not.toContain('>D<');
    const rects = elements(svg).filter((el) => el.name === 'rect');
    // first, band B, b1, b2, band C, c1
    const [first, band, b1, b2] = rects;
    expect(Number(band.attrs.get('x'))).toBe(Number(b1.attrs.get('x')));
    expect(Number(band.attrs.get('x')) + Number(band.attrs.get('width'))).toBe(
      Number(b2.attrs.get('x')) + Number(b2.attrs.get('width'))
    );
    expect(Number(band.attrs.get('y'))).toBeLessThan(Number(b1.attrs.get('y')));
    expect(Number(first.attrs.get('y'))).toBe(Number(b1.attrs.get('y')));
  });

  it('merges consecutive sections of the same name into one band, as Mermaid does', () => {
    const { svg } = render(
      'journey\n  section A\n    a: 3\n  section A\n    b: 3\n  section B\n    c: 3\n  section A\n    d: 3\n',
      options
    );
    expect(count(svg, 'pelers-cluster pelers-section')).toBe(3);
  });

  it('breaks lines at <br>, wraps long task names, and cuts long words', () => {
    const one = render('journey\n  first second: 3\n', options);
    const two = render('journey\n  first<br>second: 3\n', options);
    expect(two.height).toBeGreaterThan(one.height);
    expect(texts(two.svg).slice(5)).toEqual(['first', 'second']);
    const long = render('journey\n  ' + 'several words that go on '.repeat(4) + ': 3\n', options);
    expect(long.width).toBe(one.width);
    const cut = render('journey\n  ' + 'x'.repeat(200) + ': 3\n', options);
    expect(cut.width).toBeLessThanOrEqual(one.width + 80);
    const card = elements(cut.svg).filter((el) => el.name === 'rect')[0];
    const lines = texts(cut.svg).filter((line) => line.startsWith('x'));
    expect(lines.join('')).toBe('x'.repeat(200));
    for (const line of lines)
      expect(metricsMeasurer.width(line, 14, 0)).toBeLessThanOrEqual(Number(card.attrs.get('width')) - 20);
  });

  it('shows tags and icon names as written', () => {
    const { svg } = render('journey\n  title fa:fa-check <u>T</u>\n  <b>bold</b> a<T>: 3: <i>Me</i>\n', options);
    expect(svg).toContain('&lt;u&gt;T&lt;/u&gt;');
    expect(svg).toContain('&lt;b&gt;bold&lt;/b&gt;');
    expect(svg).toContain('fa:fa-check');
    expect(svg).toContain('a&lt;T&gt;');
    expect(svg).toContain('>&lt;i&gt;Me&lt;/i&gt;<');
    expect(svg).not.toContain('pelers-icon');
  });

  it('keeps many actors inside their card', () => {
    const { svg } = render(
      'journey\n  crowd: 3: ' + Array.from({ length: 60 }, (_, i) => `p${i}`).join(',') + '\n',
      options
    );
    const card = elements(svg).filter((el) => el.name === 'rect')[0];
    const x0 = Number(card.attrs.get('x'));
    const x1 = x0 + Number(card.attrs.get('width'));
    const dots = elements(svg).filter((el) => el.attrs.get('class') === 'pelers-actor');
    expect(dots.length).toBe(60);
    for (const dot of dots) {
      expect(Number(dot.attrs.get('cx')) - 5).toBeGreaterThanOrEqual(x0);
      expect(Number(dot.attrs.get('cx')) + 5).toBeLessThanOrEqual(x1);
    }
  });

  it('keeps the legend inside the drawing', () => {
    const { svg, width } = render(
      'journey\n  one: 3: ' + Array.from({ length: 40 }, (_, i) => `Person number ${i}`).join(',') + '\n',
      options
    );
    const rows = new Set<string>();
    for (const el of elements(svg)) {
      if (el.name !== 'circle' || el.attrs.has('class') || el.attrs.get('r') !== '5') continue;
      rows.add(el.attrs.get('cy')!);
      expect(Number(el.attrs.get('cx'))).toBeGreaterThan(0);
      expect(Number(el.attrs.get('cx'))).toBeLessThan(width);
    }
    expect(rows.size).toBeGreaterThan(5);
  });

  it('takes the title from front matter unless the diagram sets one', () => {
    expect(render('---\ntitle: From front matter\n---\njourney\n  a: 3\n', options).svg).toContain(
      '>From front matter<'
    );
    const own = render('---\ntitle: From front matter\n---\njourney\n  title Own\n  a: 3\n', options).svg;
    expect(own).toContain('>Own<');
    expect(own).not.toContain('From front matter');
  });

  it('writes accessible names', () => {
    const { svg } = render(
      'journey\n  accTitle: Short name\n  accDescr {\n    A longer\n    description\n  }\n  a: 3\n',
      options
    );
    expect(svg).toContain('<title id="pelers-title">Short name</title>');
    expect(svg).toContain('<desc id="pelers-desc">A longer\ndescription</desc>');
  });

  it('draws an empty journey, a lone title, and sections without tasks', () => {
    for (const src of ['journey', 'journey\n  title Only', 'journey\n  section S\n  section T']) {
      const { svg, width, height } = render(src, options);
      expect(svg).not.toMatch(/NaN|Infinity|undefined/);
      expect(width).toBeGreaterThan(0);
      expect(height).toBeGreaterThan(0);
      expect(count(svg, 'pelers-grid')).toBe(0);
    }
  });

  it('rejects what Mermaid rejects', () => {
    expect(() => render('journey\n  A task\n', options)).toThrow(/Expecting 'taskData', got 'NEWLINE'/);
    expect(() => render('journey\n  A task:\n', options)).toThrow(/got ':'/);
    expect(() => render('journey\n  A; B: 3\n', options)).toThrow(/got 'INVALID'/);
    expect(() => render('journey\n  : 5\n', options)).toThrow(PelersError);
    expect(() => render('journey\n  accDescr {\n  never closed\n', options)).toThrow(PelersError);
  });

  it('treats a bare title as no title', () => {
    const model = parse('journey\ntitle\nsection Work\nTask: 5: Me');
    expect(model.type).toBe('journey');
    if (model.type === 'journey') {
      expect(model.title).toBeUndefined();
      expect(model.sections).toEqual(['Work']);
    }
    expect(parse('journey\n  title\u00a0Kept\n  Task: 5: Me').title).toBe('Kept');
  });

  it('keeps the character before a comment', () => {
    const model = parse('journey\nA%% label: 5: Me');
    expect(model.type).toBe('journey');
    if (model.type === 'journey') expect(model.tasks.map((t) => t.task)).toEqual(['A%% label']);
    expect(() => render('journey\n  Task: 5: Me;%% c\n', options)).toThrow(/got 'INVALID'/);
  });

  it('exposes the model', () => {
    const model = parse('journey\n  title T\n  First: 2\n  section S\n    A task: 5: Bob, Alice\n    B task: x:\n');
    expect(model.type).toBe('journey');
    if (model.type === 'journey') {
      expect(model.title).toBe('T');
      expect(model.sections).toEqual(['S']);
      expect(model.tasks).toEqual([
        { task: 'First', score: 2, people: [], section: '' },
        { task: 'A task', score: 5, people: ['Bob', 'Alice'], section: 'S' },
        { task: 'B task', score: NaN, people: [''], section: 'S' },
      ]);
    }
  });

  it('is deterministic', () => {
    expect(render(DAY, options).svg).toBe(render(DAY, options).svg);
  });
});
