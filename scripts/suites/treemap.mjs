export default {
  files: [
    ['treemap', /^utils\.test\.ts$/],
    ['/packages/parser/tests', /^treemap\.test\.ts$/],
  ],
  docs: 'treemap.md',
  rewrite: [
    [/from '\.\/utils\.js'/g, "from './adapter.js'"],
    [/from '\.\/types\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/generated\/ast\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/treemap\/module\.js'/g, "from './adapter.js'"],
    [/import type \{ LangiumParser \} from 'langium'/g, "import type { LangiumParser } from './adapter.js'"],
  ],
};
