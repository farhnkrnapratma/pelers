const RECOVERY =
  "Reads the partial tree that Chevrotain's error recovery leaves behind. Mermaid's `parse()` throws on any parser error, so Mermaid rejects this input, and so does Pele.";

export default {
  files: [
    ['git', /^gitGraph\.spec\.ts$/],
    ['/packages/parser/tests', /^gitGraph\.test\.ts$/],
  ],
  docs: 'gitgraph.md',
  skip: [
    { file: 'gitGraph.test.ts', test: 'should ignore malformed properties and not break parsing', reason: RECOVERY },
    { file: 'gitGraph.test.ts', test: 'should ignore malformed properties in merge statements', reason: RECOVERY },
    { file: 'gitGraph.test.ts', test: 'should parse cherry-picking with a commit id', reason: RECOVERY },
  ],
  notPorted: [
    [
      'gitGraphParser.ts',
      'Its tests are written inside the source file, so there is no spec file to copy. They are ported by hand to tests/git/populate.test.ts.',
    ],
    [
      'gitGraphRenderer.ts',
      "Its tests are written inside the source file and check the pixel positions and class names of Mermaid's d3 renderer.",
    ],
  ],
  rewrite: [
    [/from '\.\/gitGraphParser\.js'/g, "from './adapter.js'"],
    [/from '\.\/gitGraphAst\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/logger\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
    [/^import type .*\n/gm, ''],
  ],
};
