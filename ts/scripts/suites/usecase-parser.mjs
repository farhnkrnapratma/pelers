// The use case specs under parser/, kept apart from scripts/suites/usecase.mjs because both
// directories have a file named usecase.parser.spec.ts.
export default {
  files: [['usecase/parser', /\.spec\.ts$/]],
  skip: [
    {
      file: 'usecase.parser.spec.ts',
      test: 'defines the stable canonical CST rule names',
      reason:
        "Reads the rule names out of Chevrotain's grammar recorder. Pele's parser is hand-written and has no grammar object.",
    },
  ],
  notPorted: [],
  rewrite: [[/from '\.\/(?:usecase\.lexer|usecase\.parser|usecaseJson)\.js'/g, "from './adapter.js'"]],
};
