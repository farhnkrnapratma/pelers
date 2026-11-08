export default {
  files: [['timeline', /^timeline\.spec\.js$/]],
  grammars: ['timeline/parser/timeline.jison'],
  docs: 'timeline.md',
  rewrite: [
    [/from '\.\/parser\/timeline\.jison'/g, "from './adapter.js'"],
    [/from '\.\/timelineDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/common\/commonDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
  ],
};
