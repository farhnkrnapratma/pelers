export default {
  files: [
    ['flowchart/parser', /\.spec\.js$/],
    ['flowchart', /^flowDb\.spec\.ts$/],
  ],
  grammars: ['flowchart/parser/flow.jison'],
  docs: 'flowchart.md',
  skip: [
    {
      file: 'flow.spec.js',
      test: "should be able to parse a '<'",
      reason:
        "Expects the label `<` to be stored as `&lt;`, which is DOMPurify's serialization. Pelers stores label text as written and escapes it when writing SVG.",
    },
    {
      file: 'flowDb.spec.ts',
      test: 'should have functions used in flow JISON as own property',
      reason:
        'Checks that FlowDB methods are own properties, a requirement of the Jison runtime. Pelers has no Jison runtime.',
    },
  ],
  notPorted: [
    ['flowChartShapes.spec.js', "Tests Mermaid's legacy dagre-d3 shape renderer."],
    ['flowDiagram.spec.ts', "Tests Mermaid's diagram registration and config plumbing."],
    ['flowRenderer-elk-default.spec.ts', "Tests Mermaid's choice between its dagre and ELK layout engines."],
    ['flowRenderer-v3-unified.spec.ts', "Tests Mermaid's DOM renderer wiring."],
    [
      'flowDb-elk-duplicate-subgraph.spec.ts',
      'Runs ELK layout. Its parsing half, a repeated subgraph id merging into one subgraph, is covered in tests/flowchart/parser.test.ts.',
    ],
  ],
  rewrite: [
    [/from '(?:\.\.\/|\.\/)flowDb\.js'/g, "from './adapter.js'"],
    [/from '(?:\.\/parser\/|\.\/)flowParser\.(?:ts|js)'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/config\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/diagram-api\/comments\.js'/g, "from './adapter.js'"],
    [/from '\.\/types\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/logger\.js'/g, "from './adapter.js'"],
  ],
};
