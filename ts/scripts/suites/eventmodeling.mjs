export default {
  files: [
    ['eventmodeling', /\.spec\.ts$/],
    ['/packages/parser/tests', /^eventmodeling\.test\.ts$/],
  ],
  docs: 'eventmodeling.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/from '\.\/(db|parser)\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/mermaidAPI\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/eventmodeling\/event-modeling-validator\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/eventmodeling\/module\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/generated\/ast\.js'/g, "from './adapter.js'"],
  ],
};
