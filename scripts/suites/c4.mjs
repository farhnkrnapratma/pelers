export default {
  files: [['c4/parser', /\.spec\.ts$/]],
  grammars: ['c4/parser/c4Diagram.jison'],
  docs: 'c4.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/import c4Db from '\.\.\/c4Db\.js'/g, "import { c4Db } from './adapter.js'"],
    [/from '\.\/c4Diagram\.jison'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/config\.js'/g, "from './adapter.js'"],
  ],
};
