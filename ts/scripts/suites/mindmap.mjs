export default {
  files: [
    ['mindmap', /^mindmap\.spec\.ts$/],
    ['mindmap', /^mindmapDb\.getData\.test\.ts$/],
  ],
  grammars: ['mindmap/parser/mindmap.jison'],
  docs: 'mindmap.md',
  notPorted: [
    [
      'mindmapLayout.spec.ts',
      "Tests which of Mermaid's layout engines (cose-bilkent, ELK, dagre) the config selects. Pele has one mindmap layout.",
    ],
  ],
  rewrite: [
    [/from '\.\/parser\/mindmap\.jison'/g, "from './adapter.js'"],
    [/from '\.\/mindmapDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/rendering-util\/types\.js'/g, "from './adapter.js'"],
    [/vi\.mock\('\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "vi.mock('./config.js'"],
  ],
};
