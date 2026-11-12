export default {
  files: [
    ['er/parser', /\.spec\.js$/],
    ['er', /^erDb\.spec\.js$/],
  ],
  grammars: ['er/parser/erDiagram.jison'],
  docs: 'entityRelationshipDiagram.md',
  skip: [],
  notPorted: [
    [
      'erRenderer.spec.ts',
      "Tests generateId from Mermaid's legacy d3 renderer, which derives DOM ids from a UUID. Pelers writes no element ids.",
    ],
  ],
  rewrite: [
    [/from '(?:\.\.\/|\.\/)erDb\.js'/g, "from './adapter.js'"],
    [/from '\.\/erDiagram\.jison'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/config\.js'/g, "from './adapter.js'"],
  ],
};
