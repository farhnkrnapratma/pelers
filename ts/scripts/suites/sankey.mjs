export default {
  files: [['sankey/parser', /^sankey\.spec\.ts$/]],
  // energy.csv is the data file the spec reads; it is vendored untouched next to the grammar.
  grammars: ['sankey/parser/sankey.jison', 'sankey/parser/energy.csv'],
  docs: 'sankey.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/import sankey from '\.\/sankey\.jison'/g, "import sankey from './adapter.js'"],
    [/import db from '\.\.\/sankeyDB\.js'/g, "import { db } from './adapter.js'"],
    [/from '\.\.\/\.\.\/\.\.\/diagram-api\/comments\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/sankeyUtils\.js'/g, "from './adapter.js'"],
    [/'\.\/energy\.csv'/g, "'./upstream/energy.csv'"],
  ],
};
