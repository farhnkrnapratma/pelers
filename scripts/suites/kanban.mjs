export default {
  files: [['kanban', /^kanban\.spec\.ts$/]],
  grammars: ['kanban/parser/kanban.jison'],
  docs: 'kanban.md',
  rewrite: [
    [/from '\.\/parser\/kanban\.jison'/g, "from './adapter.js'"],
    [/from '\.\/kanbanDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/rendering-util\/types\.js'/g, "from './adapter.js'"],
  ],
};
