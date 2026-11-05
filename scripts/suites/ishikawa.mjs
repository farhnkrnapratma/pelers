export default {
  files: [['ishikawa', /^ishikawa\.spec\.ts$/]],
  grammars: ['ishikawa/parser/ishikawa.jison'],
  docs: 'ishikawa.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/from '\.\/parser\/ishikawa\.jison'/g, "from './adapter.js'"],
    [/from '\.\/ishikawaDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
  ],
};
