export default {
  files: [
    ['quadrant-chart/parser', /\.spec\.ts$/],
    ['quadrant-chart', /^quadrantDb\.spec\.ts$/],
  ],
  grammars: ['quadrant-chart/parser/quadrant.jison'],
  docs: 'quadrantChart.md',
  skip: [
    {
      file: 'quadrantDb.spec.ts',
      test: 'should call getConfig at sanitization time, not at module load time',
      reason:
        'Checks when Mermaid reads its global config to sanitize text with DOMPurify. Pele has no global config and escapes text when writing SVG.',
    },
  ],
  rewrite: [
    [/from '\.\/quadrant\.jison'/g, "from './adapter.js'"],
    [/from '\.\/quadrantDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
  ],
};
