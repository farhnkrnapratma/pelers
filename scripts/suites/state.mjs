export default {
  files: [
    ['state/parser', /\.spec\.js$/],
    ['state', /^(?:stateDb|stateDiagram|stateDiagram-v2|stateNoteNode)\.spec\.js$/],
    ['state', /^(?:stateNoteEdge|stateDiagram-colorIndex)\.spec\.ts$/],
  ],
  grammars: ['state/parser/stateDiagram.jison'],
  docs: 'stateDiagram.md',
  skip: [
    {
      file: 'stateDb.spec.js',
      test: 'sanitizes on the description',
      reason:
        'Expects DOMPurify to have removed a script element from the stored description. Pele stores text as written and escapes it when writing SVG.',
    },
    {
      file: 'stateDb.spec.js',
      test: 'should have functions used in flow JISON as own property',
      reason:
        'Checks that StateDB methods are own properties, a requirement of the Jison runtime. Pele has no Jison runtime.',
    },
    {
      file: 'stateDiagram-v2.spec.js',
      test: 'should handle note statements',
      reason:
        "Its snapshot expects `<br/>` in the note text to be stored as `<br>`, which is DOMPurify's serialization. Pele stores text as written; both spellings break the line when drawn.",
    },
    {
      file: 'stateDiagram-v2.spec.js',
      test: 'should handle multiline notes with different line breaks',
      reason:
        "Its snapshot expects every spelling of `<br>` in the note text to be stored as `<br>`, which is DOMPurify's serialization. Pele stores text as written; all of them break the line when drawn.",
    },
  ],
  notPorted: [
    ['stateRenderer.spec.js', "Tests Mermaid's legacy v1 renderer against a mocked DOM and dagre."],
    ['stateRenderer-v3-unified.spec.js', "Tests Mermaid's DOM renderer wiring and its HTML tooltip element."],
  ],
  rewrite: [
    [/from '(?:\.\/parser\/|\.\/)stateDiagram\.jison'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/|\.\/)stateDb\.js'/g, "from './adapter.js'"],
    [/from '\.\/stateCommon\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/config\.js'/g, "from './adapter.js'"],
  ],
};
