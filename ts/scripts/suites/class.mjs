export default {
  files: [
    ['class/parser', /\.spec\.js$/],
    ['class', /^(?:classDiagram|classDiagram-styles|classDiagram-colorIndex|classTypes)\.spec\.[jt]s$/],
  ],
  grammars: ['class/parser/classDiagram.jison'],
  docs: 'classDiagram.md',
  skip: [
    {
      file: 'classDiagram.spec.ts',
      test: 'should parse diagram with direction',
      reason:
        "Its inline snapshot pins Mermaid's DOM id counter (`classId-Student-154`), the shape name and the markdown-escaped member text its renderer reads. Pele's model has none of these. The direction, members and relations it checks are covered in tests/class/parser.test.ts.",
    },
    {
      file: 'classDiagram.spec.ts',
      test: 'should have functions used in class JISON as own property',
      reason:
        'Checks that ClassDB methods are own properties, a requirement of the Jison runtime. Pele has no Jison runtime.',
    },
  ],
  notPorted: [
    [
      'svgDraw.spec.js',
      "Tests the title helper of Mermaid's legacy d3 class renderer, which the current renderer does not use, under jsdom.",
    ],
  ],
  rewrite: [
    [/from '(?:\.\/parser\/|\.\/)classDiagram\.jison'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/|\.\/)classDb\.js'/g, "from './adapter.js'"],
    [/from '\.\/classTypes\.js'/g, "from './adapter.js'"],
    [/import\('\.\.\/\.\.\/diagram-api\/diagramAPI\.js'\)/g, "import('./adapter.js')"],
    [/import\('\.\.\/\.\.\/config\.js'\)/g, "import('./adapter.js')"],
  ],
};
