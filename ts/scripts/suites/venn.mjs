const DOM =
  "Draws with Mermaid's renderer into a jsdom document and reads the elements back. Pelers's output is covered by its own render tests.";

export default {
  files: [
    ['venn/parser', /\.spec\.ts$/],
    ['venn', /^vennRenderer\.spec\.ts$/],
  ],
  grammars: ['venn/parser/venn.jison'],
  docs: 'venn.md',
  skip: [
    { file: 'vennRenderer.spec.ts', test: 'renders a title when provided', reason: DOM },
    { file: 'vennRenderer.spec.ts', test: 'renders text nodes with custom color via style data', reason: DOM },
    { file: 'vennRenderer.spec.ts', test: 'applies theme colors to circles', reason: DOM },
    { file: 'vennRenderer.spec.ts', test: 'user override colors take priority over theme via style data', reason: DOM },
    { file: 'vennRenderer.spec.ts', test: 'computes contrasting text color for dark backgrounds', reason: DOM },
    { file: 'vennRenderer.spec.ts', test: 'renders debug layout helpers when enabled', reason: DOM },
  ],
  notPorted: [
    [
      'vennPalette.spec.ts',
      'Tests which colors each Mermaid theme gives the circles. Pelers takes its circle colors from the --pelers-series tokens.',
    ],
  ],
  rewrite: [
    [/from '\.\/venn\.jison'/g, "from './adapter.js'"],
    [/from '\.\.\/vennDB\.js'/g, "from './adapter.js'"],
    [/from '\.\/vennRenderer\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/(?:Diagram|config)\.js'/g, "from './adapter.js'"],
  ],
};
