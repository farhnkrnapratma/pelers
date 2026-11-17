import { describe, expect, it } from 'vitest';
import { PelersError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import type { RenderOptions } from '../../src/types.js';

const options = { measurer: metricsMeasurer };
const count = (svg: string, re: RegExp): number => svg.match(re)?.length ?? 0;

const TREE =
  'treeView-beta\n    my-project/\n        src/\n            index.js\n        package.json\n    README.md\n';

describe('treeView rendering', () => {
  it('draws a row per entry, directories in bold, and a guide per parent', () => {
    const { svg, type } = render(TREE, options);
    expect(type).toBe('treeView');
    expect(supports(TREE)).toBe(true);
    expect(count(svg, /class="pelers-node pelers-tree-(dir|file)"/g)).toBe(5);
    expect(count(svg, /class="pelers-node pelers-tree-dir"/g)).toBe(2);
    expect(svg).toContain('font-weight="var(--_hw)" xml:space="preserve">my-project</text>');
    expect(svg).toContain('xml:space="preserve">index.js</text>');
    expect(count(svg, /<path d="M/g)).toBe(2);
    expect(svg).not.toContain('>/<');
  });

  it('exposes the model as a tree', () => {
    const model = parse(TREE);
    expect(model.type).toBe('treeView');
    if (model.type !== 'treeView') return;
    expect(model.root.children.map((n) => n.name)).toEqual(['my-project', 'README.md']);
    expect(model.root.children[0].children.map((n) => [n.name, n.nodeType])).toEqual([
      ['src', 'directory'],
      ['package.json', 'file'],
    ]);
    expect(model.count).toBe(6);
  });

  it('accepts box-drawing input and gives the same model', () => {
    const boxed = parse(
      'treeView-beta\n├── src/\n│   ├── index.ts\n│   └── utils.ts\n├── package.json\n└── README.md\n'
    );
    const indented = parse(
      'treeView-beta\n    src/\n        index.ts\n        utils.ts\n    package.json\n    README.md\n'
    );
    expect(boxed).toEqual(indented);
  });

  it('reports box-drawing errors against the original lines', () => {
    const src = 'treeView-beta\n│\n├── src/\n│\n│   └── "a\n';
    try {
      render(src, options);
      expect.unreachable();
    } catch (error) {
      expect(error).toBeInstanceOf(PelersError);
      expect((error as PelersError).line).toBe(5);
      expect((error as PelersError).message).toContain('line 5');
    }
    expect(() => render('treeView-beta\n├── \n', options)).toThrow('Line 2: Empty node');
    expect(() => render('treeView-beta\n├── a\n   b\n', options)).toThrow('Line 3: Unexpected indentation');
    expect(() => render('---\ntitle: T\n---\ntreeView-beta\n└─ "bad', options)).toThrow(
      expect.objectContaining({
        line: 5,
        column: 4,
        snippet: '└─ "bad\n---^',
        message: expect.stringMatching(/^Lexical error on line 5\./),
      })
    );
    expect(() => render('---\ntitle: T\n---\ntreeView-beta\n├── \n', options)).toThrow('Line 5: Empty node');
    expect(() => render('treeView-beta\n└─ "ok"\t"bad', options)).toThrow(
      expect.objectContaining({ line: 2, column: 9 })
    );
    expect(() => render('treeView-beta\n└── "ok"\t\t"bad', options)).toThrow(
      expect.objectContaining({ line: 2, column: 11, snippet: '└── "ok"\t\t"bad\n----------^' })
    );
    expect(() => render('---\ntitle: T\n---\ntreeView-beta\n"bad\nhello ─ world\nx\ny', options)).toThrow(
      expect.objectContaining({ line: 5, column: 1 })
    );
  });

  it('draws highlights, descriptions in one column, and extra classes', () => {
    const { svg } = render(
      'treeView-beta\n    src/\n        App.tsx :::highlight ## main\n        index.js :::mine ## entry point\n',
      options
    );
    expect(count(svg, /class="pelers-tree-highlight"/g)).toBe(1);
    expect(svg).toContain('class="pelers-node pelers-tree-file highlight"');
    expect(svg).toContain('class="pelers-node pelers-tree-file mine"');
    const xs = [...svg.matchAll(/class="pelers-tree-description" x="([\d.]+)"/g)].map((m) => m[1]);
    expect(xs).toHaveLength(2);
    expect(xs[0]).toBe(xs[1]);
  });

  it('measures names in the monospace font when monospace lists files', () => {
    const src = 'treeView-beta\n    src/\n        App.tsx ## main\n';
    const descX = (svg: string): number => Number(/class="pelers-tree-description" x="([\d.]+)"/.exec(svg)![1]);
    const plain = render(src, options).svg;
    for (const maxWidth of [undefined, 90]) {
      const svg = render(src, { ...options, maxWidth, monospace: ['files'] }).svg;
      expect(count(svg, /<text class="pelers-label"[^>]* font-family="var\(--_fm\)"/g)).toBe(maxWidth ? 3 : 2);
      expect(svg).toMatch(/<text class="pelers-label"[^>]* font-weight="var\(--_hw\)"[^>]*>src</);
      expect(svg).not.toMatch(/<text class="pelers-tree-description"[^>]* font-family=/);
    }
    expect(plain).not.toContain('font-family="');
    expect(descX(render(src, { ...options, monospace: ['files'] }).svg)).toBeGreaterThan(descX(plain));
    for (const monospace of [[], ['file'], ['members'], 'files']) {
      expect(render(src, { ...options, monospace } as RenderOptions).svg).toBe(plain);
    }
  });

  it('shows icons only when asked, and offers each icon name to the host', () => {
    expect(render(TREE, options).svg).not.toContain('pelers-icon');
    const shown = render('---\nconfig:\n  treeView:\n    showIcons: true\n---\n' + TREE, options).svg;
    expect(count(shown, /class="pelers-icon"/g)).toBe(5);
    expect(shown).toContain('data-icon="mermaid-treeview:folder"');
    expect(shown).toContain('data-icon="mermaid-treeview:file"');

    const asked: string[] = [];
    const custom = render('treeView-beta\n  App.tsx icon(logos:react)\n  index.js\n  x.rs icon(rust)\n  y icon()\n', {
      ...options,
      icons: (name) => (asked.push(name), name === 'logos:react' ? '<circle r="4"/>' : null),
    }).svg;
    expect(asked).toEqual(['logos:react', 'mermaid-treeview:rust']);
    expect(custom).toContain('data-icon="logos:react"');
    expect(custom).toContain('<circle r="4"/>');
    expect(count(custom, /class="pelers-icon"/g)).toBe(2);
  });

  it('maps file names and extensions to icons from config', () => {
    const src =
      '---\nconfig:\n  treeView:\n    showIcons: true\n    defaultIconPack: material\n    filenameIcons:\n      Dockerfile: docker\n    extensionIcons:\n      .ts: typescript\n      txt: none\n---\n' +
      'treeView-beta\n  a.ts\n  Dockerfile\n  notes.txt\n  constructor\n  dir.ts/\n';
    const { svg } = render(src, options);
    expect(svg).toContain('data-icon="material:typescript"');
    expect(svg).toContain('data-icon="material:docker"');
    expect(count(svg, /data-icon="mermaid-treeview:file"/g)).toBe(1);
    expect(count(svg, /data-icon="mermaid-treeview:folder"/g)).toBe(1);
    expect(count(svg, /class="pelers-icon"/g)).toBe(4);
  });

  it('keeps names as written', () => {
    const { svg } = render('treeView-beta\n  "a  b   c"\n  <T>.ts ## x <br> y\n  #35;hash\n', options);
    expect(svg).toContain('>a  b   c</text>');
    expect(svg).toContain('>&lt;T&gt;.ts</text>');
    expect(svg).toContain('>x &lt;br&gt; y</text>');
    expect(svg).toContain('>#hash</text>');
  });

  it('reads names that start with title', () => {
    const model = parse('treeView-beta\ntitles/\n  title.txt\n  title-page.md\n');
    expect(model.type).toBe('treeView');
    if (model.type === 'treeView') {
      const [dir] = model.root.children;
      expect([dir.name, ...dir.children.map((c) => c.name)]).toEqual(['titles', 'title.txt', 'title-page.md']);
    }
    const { svg } = render('treeView-beta\ntitles/\n├── title.txt\n└── title-page.md\n', options);
    expect(svg).toContain('>title.txt</text>');
    expect(svg).not.toContain('>.txt</text>');
    const titled = parse('treeView-beta\ntitle%% c\naccTitle: A\nroot\n');
    expect(titled.type === 'treeView' && [titled.accTitle, titled.root.children.map((c) => c.name)]).toEqual([
      'A',
      ['root'],
    ]);
  });

  it('draws titles and accessible names', () => {
    const { svg } = render('---\ntitle: Front\n---\ntreeView-beta\naccTitle: Acc\naccDescr: Described\n  a\n', options);
    expect(svg).toContain('>Front<');
    expect(svg).toContain('<title id="pelers-title">Acc</title>');
    expect(svg).toContain('<desc id="pelers-desc">Described</desc>');
    expect(render('---\ntitle: Front\n---\ntreeView-beta\ntitle Own\n  a\n', options).svg).toContain('>Own<');
  });

  it('honours the layout options', () => {
    const base = render(TREE, options);
    const wide = render(
      '---\nconfig:\n  treeView:\n    rowIndent: 80\n    paddingY: 10\n    lineThickness: 3\n---\n' + TREE,
      options
    );
    expect(wide.width).toBeGreaterThan(base.width);
    expect(wide.height).toBeGreaterThan(base.height);
    expect(wide.svg).toContain('stroke-width="3"');
  });

  it('draws an empty tree and is deterministic', () => {
    expect(render('treeView-beta', options).svg).toContain('<svg');
    expect(render(TREE, options).svg).toBe(render(TREE, options).svg);
  });

  it('rejects what Mermaid rejects', () => {
    for (const src of [
      'treeView-beta title x\n a',
      'treeView-beta\n a\ntitle late',
      'treeView-beta\n  \n  a',
      'treeView-beta\n :::x',
    ]) {
      expect(() => render(src, options), src).toThrow(PelersError);
    }
  });
});
