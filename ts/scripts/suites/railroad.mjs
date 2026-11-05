export default {
  files: [
    ['railroad', /^(?:railroadDb|\w+Detector)\.spec\.ts$/],
    ['railroad/parser', /\.spec\.ts$/],
    ['/packages/parser/tests', /^railroad\.test\.ts$/],
  ],
  docs: 'railroad.md',
  skip: [
    {
      file: 'railroadDb.spec.ts',
      test: 'should sanitize titles before storing them',
      reason: 'Checks DOMPurify output. Pele stores text as written and escapes it when it writes the SVG.',
    },
    {
      file: 'railroadDb.spec.ts',
      test: 'should sanitize rule names and terminal values at the db boundary',
      reason: 'Checks DOMPurify output. Pele stores text as written and escapes it when it writes the SVG.',
    },
  ],
  notPorted: [
    [
      'railroadRenderer.spec.ts',
      "Tests Mermaid's DOM renderer: element transforms and path coordinates from a stubbed getBBox. Pele has its own layout.",
    ],
    [
      'styles.spec.ts',
      'Tests the stylesheet text Mermaid generates from theme variables. Pele writes no stylesheet; colors come from CSS custom properties.',
    ],
  ],
  rewrite: [
    [/import \{ parser \} from '\.\/railroadParser\.js'/g, "import { railroadParser as parser } from './adapter.js'"],
    [/import \{ parser \} from '\.\/(ebnf|abnf|peg)Parser\.js'/g, "import { $1Parser as parser } from './adapter.js'"],
    [/from '(?:\.\.\/|\.\/)railroadDb\.js'/g, "from './adapter.js'"],
    [/from '\.\/\w+Detector\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/generated\/ast\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/railroad\/module\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/parse\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
  ],
};
