export default {
  files: [
    ['xychart/parser', /\.spec\.ts$/],
    ['xychart', /^xychartDb\.spec\.ts$/],
  ],
  grammars: ['xychart/parser/xychart.jison'],
  docs: 'xyChart.md',
  notPorted: [
    ['chartBuilder/textDimensionCalculator.spec.ts', "Tests Mermaid's DOM text measurement."],
    ['chartBuilder/components/chartTitle.spec.ts', "Tests the title component of Mermaid's chart layout."],
    ['chartBuilder/components/legend.spec.ts', "Tests the legend component of Mermaid's chart layout."],
  ],
  rewrite: [
    [/from '\.\/xychart\.jison'/g, "from './adapter.js'"],
    [/from '\.\/xychartDb\.js'/g, "from './adapter.js'"],
  ],
};
