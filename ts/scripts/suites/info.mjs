const wording = "Checks the wording of Chevrotain's lexer error.";

export default {
  files: [
    ['info', /^info\.spec\.ts$/],
    ['/packages/parser/tests', /^info\.test\.ts$/],
  ],
  rewrite: [
    [/from '\.\/infoParser\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/index\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
  ],
  // The spec has this test twice.
  skip: [
    { file: 'info.spec.ts', test: 'should throw because of unsupported info grammar', reason: wording },
    { file: 'info.spec.ts', test: 'should throw because of unsupported info grammar', reason: wording },
  ],
};
