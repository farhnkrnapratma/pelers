export default {
  files: [
    ['requirement/parser', /\.spec\.js$/],
    ['requirement', /^requirementDb\.spec\.ts$/],
  ],
  grammars: ['requirement/parser/requirementDiagram.jison'],
  docs: 'requirementDiagram.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/from '\.\.\/\.\.\/\.\.\/config\.js'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/|\.\/)requirementDb\.js'/g, "from './adapter.js'"],
    [/from '\.\/requirementDiagram\.jison'/g, "from './adapter.js'"],
    [/from '\.\/types\.js'/g, "from './adapter.js'"],
  ],
};
