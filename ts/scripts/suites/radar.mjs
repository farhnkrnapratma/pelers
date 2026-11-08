const wording = "Checks the wording of Langium's MermaidParseError.";

export default {
  files: [
    ['radar', /^radar\.spec\.ts$/],
    ['/packages/parser/tests', /^radar\.test\.ts$/],
  ],
  docs: 'radar.md',
  rewrite: [
    [/from '\.\/db\.js'/g, "from './adapter.js'"],
    [/from '\.\/parser\.js'/g, "from './adapter.js'"],
    [/from '\.\/renderer\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/Diagram\.js'/g, "from './adapter.js'"],
    [/import mermaidAPI from '\.\.\/\.\.\/mermaidAPI\.js'/g, "import { mermaidAPI } from './adapter.js'"],
    [/from '\.\.\/src\/language\/index\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/parse\.js'/g, "from './adapter.js'"],
  ],
  skip: [
    'should include line and column numbers in parser errors for radar diagrams',
    'should include line and column numbers for missing curve entries',
    'should include line and column numbers for invalid axis syntax',
    'should handle lexer errors with line and column numbers',
    'should format error message with "Parse error on line X, column Y" prefix',
    'should report an unknown location for errors at the end of input',
  ].map((test) => ({ file: 'radar.test.ts', test, reason: wording })),
};
