export default {
  files: [
    ['pie', /^pie\.spec\.ts$/],
    ['/packages/parser/tests', /^pie\.test\.ts$/],
  ],
  docs: 'pie.md',
  notPorted: [['pieRenderer.spec.ts', "Tests Mermaid's d3 renderer."]],
  rewrite: [
    [/from '\.\/pieParser\.js'/g, "from './adapter.js'"],
    [/from '\.\/pieDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/index\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
  ],
};
