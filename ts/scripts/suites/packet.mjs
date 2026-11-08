export default {
  files: [
    ['packet', /^packet\.spec\.ts$/],
    ['/packages/parser/tests', /^packet\.test\.ts$/],
  ],
  docs: 'packet.md',
  notPorted: [['renderer.spec.ts', "Reads positions back from the DOM that Mermaid's renderer builds."]],
  rewrite: [
    [/from '\.\/db\.js'/g, "from './adapter.js'"],
    [/from '\.\/parser\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/index\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
  ],
};
