export default {
  files: [
    ['treeView', /\.spec\.ts$/],
    ['/packages/parser/tests', /^treeView(ValueConverter)?\.test\.ts$/],
  ],
  docs: 'treeView.md',
  skip: [
    {
      file: 'icons.spec.ts',
      test: 'every icon has a non-empty body that inherits color via currentColor',
      reason:
        "Checks the markup of Mermaid's Iconify pack. Pele keeps path data for the two built-in glyphs and colors them with theme tokens.",
    },
  ],
  notPorted: [],
  rewrite: [
    [/from '\.\/(db|parser|icons|boxDrawingPreprocessor)\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/treeView\/(module|valueConverter)\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
  ],
};
