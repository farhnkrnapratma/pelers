import { describe, expect, it } from 'vitest';
import { PelersError, parse, render, supports } from '../../src/index.js';
import type { KanbanModel } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const options = { measurer: metricsMeasurer };
const BASE = "---\nconfig:\n  kanban:\n    ticketBaseUrl: 'https://example.com/browse/#TICKET#'\n---\n";

function model(src: string): KanbanModel {
  return parse(src) as KanbanModel;
}

describe('kanban rendering', () => {
  it('draws a column per section and a card per item, with counts', () => {
    const { svg, type } = render(
      'kanban\n  todo[To do]\n    a[First]\n    b[Second]\n  done[Done]\n    c[Third]\n  later[Later]',
      options
    );
    expect(type).toBe('kanban');
    expect(supports('kanban\n  a')).toBe(true);
    expect(svg.match(/class="pelers-cluster pelers-column"/g)?.length).toBe(3);
    expect(svg.match(/class="pelers-node pelers-card"/g)?.length).toBe(3);
    for (const text of ['To do', 'First', 'Second', 'Done', 'Third', 'Later']) expect(svg).toContain(`>${text}</text>`);
    expect([...svg.matchAll(/class="pelers-count"[^>]*>(\d+)</g)].map((m) => m[1])).toEqual(['2', '1', '0']);
    expect(svg).toContain('data-id="todo"');
    expect(svg).toContain('data-id="a"');
  });

  it('gives every column the width of the others and the height of the tallest', () => {
    const { svg } = render('kanban\n  a\n    one\n    two\n    three\n  b\n    four\n  c', options);
    const columns = [
      ...svg.matchAll(
        /pelers-column" data-id="[^"]*" transform="translate\(([\d.]+),[\d.]+\)"><rect width="(\d+)" height="([\d.]+)"/g
      ),
    ];
    expect(columns.length).toBe(3);
    expect(new Set(columns.map((m) => m[2]))).toEqual(new Set(['200']));
    expect(new Set(columns.map((m) => m[3])).size).toBe(1);
    expect(+columns[1][1] - +columns[0][1]).toBe(+columns[2][1] - +columns[1][1]);
    const wide = render('---\nconfig:\n  kanban:\n    sectionWidth: 320\n---\nkanban\n  a\n    one\n  b', options);
    expect(wide.svg).toContain('<rect width="320"');
    expect(wide.width).toBeGreaterThan(640);
  });

  it('treats every deeper line as a card of the last column', () => {
    const { sections } = model('kanban\n  root\n    child1\n      leaf1\n    child2\n  second\n      deep');
    expect(sections.map((section) => section.items.map((item) => item.id))).toEqual([
      ['child1', 'leaf1', 'child2'],
      ['deep'],
    ]);
  });

  it('rejects a line after one indented less than the first column', () => {
    expect(model('kanban\n    a\n  shallow').sections[0].items[0].id).toBe('shallow');
    const bad = (): unknown => render('kanban\n    a\n  shallow\n    b', options);
    expect(bad).toThrow(PelersError);
    expect(bad).toThrow('Items without section detected, found section ("shallow")');
  });

  it('reads metadata on one line and on several', () => {
    const one = model("kanban\n  todo\n    a[Task]@{ ticket: MC-2037, assigned: 'knsv', priority: 'High', icon: star }")
      .sections[0].items[0];
    expect(one).toMatchObject({
      id: 'a',
      label: 'Task',
      ticket: 'MC-2037',
      assigned: 'knsv',
      priority: 'High',
      icon: 'star',
    });
    const many = model(
      'kanban\n  todo\n    a@{\n      assigned: knsv\n      label: "A new\n        label"\n      ticket: 12\n    }'
    ).sections[0].items[0];
    expect(many).toMatchObject({ assigned: 'knsv', label: 'A new<br/>label', ticket: '12' });
  });

  it('shows the assignee and the ticket', () => {
    const { svg, links } = render("kanban\n  todo\n    a[Task]@{ ticket: MC-1, assigned: 'knsv' }", options);
    expect(svg).toMatch(/class="pelers-ticket" fill="var\(--_m\)"[^>]*>MC-1</);
    expect(svg).toMatch(/class="pelers-assigned" fill="var\(--_m\)"[^>]*text-anchor="end">knsv</);
    expect(svg).not.toContain('<a ');
    expect(links).toEqual([]);
  });

  it('links the ticket through ticketBaseUrl', () => {
    const { svg, links } = render(
      BASE + 'kanban\n  todo\n    a[Task]@{ ticket: MC-1 }\n    b[Other]@{ ticket: "$&" }',
      options
    );
    expect(svg).toContain('<a href="https://example.com/browse/MC-1" target="_blank" rel="noopener">');
    expect(links).toEqual([
      { id: 'a', href: 'https://example.com/browse/MC-1', internal: false },
      { id: 'b', href: 'https://example.com/browse/$&', internal: false },
    ]);
  });

  it('drops a ticket link that is not a safe URL', () => {
    const src =
      "---\nconfig:\n  kanban:\n    ticketBaseUrl: 'javascript:alert(#TICKET#)'\n---\nkanban\n  todo\n    a[Task]@{ ticket: MC-1 }";
    const { svg, links } = render(src, options);
    expect(svg).not.toContain('<a ');
    expect(svg).toContain('>MC-1<');
    expect(links).toEqual([]);
    const whole = render(
      "---\nconfig:\n  kanban:\n    ticketBaseUrl: '#TICKET#'\n---\nkanban\n  todo\n    a[Task]@{ ticket: 'javascript:alert(1)' }",
      options
    );
    expect(whole.svg).not.toContain('<a ');
  });

  it('marks priority with a colored edge', () => {
    const src =
      "kanban\n  todo\n    a[A]@{ priority: 'Very High' }\n    b[B]@{ priority: High }\n    c[C]@{ priority: Medium }\n" +
      "    d[D]@{ priority: 'Low' }\n    e[E]@{ priority: 'Very Low' }\n    f[F]@{ priority: high }\n    g[G]@{ priority: whenever }";
    const { svg } = render(src, options);
    const edge = (id: string): string | undefined =>
      new RegExp(
        `data-id="${id}"[^>]*><rect[^>]*/><path class="pelers-priority"[^>]*fill="none" (.*?) stroke-width`
      ).exec(svg)?.[1];
    const red = 'stroke="var(--pelers-critical,#e45756)"';
    const grey = 'stroke="var(--_m)"';
    const faint = ' opacity="0.5"';
    expect(['a', 'b', 'c', 'd', 'e', 'f', 'g'].map(edge)).toEqual([
      red,
      red + faint,
      undefined,
      grey + faint,
      grey,
      red + faint,
      undefined,
    ]);
    expect(svg).not.toContain('--pelers-series-');
    expect(svg).toContain('class="pelers-node pelers-card pelers-priority-very-high" data-id="a"');
    expect(svg).toContain('class="pelers-node pelers-card" data-id="g"');
  });

  it('applies classes and icons to columns and cards', () => {
    const src = 'kanban\n  todo[To do]\n  :::wide\n  ::icon(inbox)\n    a[Task]@{ icon: star }\n    :::urgent large';
    const { svg } = render(src, { ...options, icons: (name) => `<path d="M0,0" data-icon="${name}"/>` });
    expect(svg).toContain('class="pelers-cluster pelers-column wide" data-id="todo"');
    expect(svg).toContain('class="pelers-node pelers-card urgent large" data-id="a"');
    expect(svg).toContain('<svg class="pelers-icon" data-icon="inbox"');
    expect(svg).toContain('<path d="M0,0" data-icon="star"/></svg>');
  });

  it('wraps card text and breaks a word that is longer than the card', () => {
    const short = render('kanban\n  todo\n    a[Short]', options);
    const long = render('kanban\n  todo\n    a[' + 'several words that wrap '.repeat(4) + ']', options);
    const word = render('kanban\n  todo\n    a[' + 'x'.repeat(120) + ']', options);
    expect(long.height).toBeGreaterThan(short.height);
    expect(long.width).toBe(short.width);
    expect(word.width).toBe(short.width);
    const pieces = [...word.svg.matchAll(/class="pelers-label"[^>]*>(x+)</g)].map((m) => m[1]);
    expect(pieces.length).toBeGreaterThan(3);
    expect(pieces.join('')).toBe('x'.repeat(120));
    for (const piece of pieces) expect(metricsMeasurer.width(piece, 14, 0)).toBeLessThanOrEqual(164);
  });

  it('uses a label from metadata and formats markdown', () => {
    const { svg } = render('kanban\n  todo\n    a[Old]@{ label: "New **bold** text" }', options);
    expect(svg).not.toContain('Old');
    expect(svg).toContain('font-weight="var(--_w)">bold</tspan>');
  });

  it('reports metadata that is not valid YAML, and ignores metadata that is not a mapping', () => {
    expect(() => render('kanban\n  todo\n    a@{ label: "x }', options)).toThrow(PelersError);
    expect(() => render('kanban\n  todo\n    a@{ a: [ }', options)).toThrow(/not valid YAML/);
    expect(render('kanban\n  todo\n    a@{\n}', options).svg).toContain('>a</text>');
    expect(render('kanban\n  todo\n    a@{\n - x\n}', options).svg).toContain('>a</text>');
    expect(render('kanban\n  todo\n    a@{}', options).svg).toContain('>a</text>');
  });

  it('rejects shape names Mermaid rejects', () => {
    expect(() => render('kanban\n  todo\n    a@{ shape: Rect }', options)).toThrow(
      'No such shape: Rect. Shape names should be lowercase.'
    );
    expect(() => render('kanban\n  todo\n    a@{ shape: kanban_item }', options)).toThrow(/No such shape/);
    expect(render('kanban\n  todo\n    a@{ shape: rect }', options).svg).toContain('>a</text>');
  });

  it('ignores a decoration that comes before any node', () => {
    expect(render('kanban\n::icon(star)\n:::big\ntodo', options).svg).toContain('>todo</text>');
  });

  it('draws the front matter title', () => {
    const plain = render('kanban\n  todo\n    a', options);
    const titled = render('---\ntitle: Sprint 12\n---\nkanban\n  todo\n    a', options);
    expect(titled.svg).toContain('class="pelers-title" font-weight="var(--_tw)"');
    expect(titled.svg).toContain('>Sprint 12<');
    expect(titled.height).toBeGreaterThan(plain.height);
  });

  it('renders a board with no columns', () => {
    expect(render('kanban\n\n', options).svg).toContain('<svg');
  });

  it('is deterministic and emits only finite numbers', () => {
    const src =
      BASE +
      "kanban\n  Todo\n    [Create Documentation]\n  id9[Ready]\n    id8[Design grammar]@{ assigned: 'knsv', ticket: MC-1, priority: 'Low' }";
    const first = render(src, options).svg;
    expect(render(src, options).svg).toBe(first);
    expect(first).not.toMatch(/NaN|Infinity|undefined/);
  });
});
