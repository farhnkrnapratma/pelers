const STYLESHEET = 'Tests the CSS stylesheet Mermaid generates for themes. Pelers emits no stylesheet.';

export default {
  files: [
    ['block/parser', /\.spec\.ts$/],
    ['block', /\.spec\.ts$/],
  ],
  grammars: ['block/parser/block.jison'],
  docs: 'block.md',
  skip: [
    {
      file: 'block.spec.ts',
      test: 'sanitizes labels with the current dompurify config',
      reason:
        'Expects DOMPurify to strip a tag from a label. Pelers stores label text as written and escapes it when writing SVG.',
    },
    {
      file: 'blockDB.spec.ts',
      test: 'should call getConfig at sanitization time, not at module load time',
      reason: 'Checks when Mermaid reads its global config for DOMPurify. Pelers has neither.',
    },
    {
      file: 'blockColorIndex.spec.ts',
      test: 'emits one composite rule per palette entry under a colour theme',
      reason: STYLESHEET,
    },
    {
      file: 'blockColorIndex.spec.ts',
      test: 'leaves the simple shapes to the flat theme colour',
      reason: STYLESHEET,
    },
    {
      file: 'blockColorIndex.spec.ts',
      test: 'emits nothing for a theme that carries no palette',
      reason: STYLESHEET,
    },
    {
      file: 'blockColorIndex.spec.ts',
      test: 'rejects a look that would break out of the selector',
      reason: STYLESHEET,
    },
  ],
  notPorted: [],
  rewrite: [
    [/import block from '\.\/(?:parser\/)?block\.jison'/g, "import { block } from './adapter.js'"],
    [/import db from '(?:\.\.\/|\.\/)blockDB\.js'/g, "import { db } from './adapter.js'"],
    [/import \* as configApi from '(?:\.\.\/)+config\.js'/g, "import { configApi } from './adapter.js'"],
    [/import getStyles from '\.\/styles\.js'/g, "import { getStyles } from './adapter.js'"],
    [/from '(?:\.\.\/)+diagram-api\/diagramAPI\.js'/g, "from './config.js'"],
    [/from '(?:\.\.\/)+logger\.js'/g, "from './adapter.js'"],
    [/from '\.\/(?:blockDB|blockTypes|layout)\.js'/g, "from './adapter.js'"],
  ],
};
