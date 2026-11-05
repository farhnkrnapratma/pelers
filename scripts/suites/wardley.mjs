export default {
  files: [['wardley', /\.spec\.ts$/]],
  docs: 'wardley.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/from '\.\/wardley(Diagram|Builder)\.js'/g, "from './adapter.js'"],
    [/import type WardleyDb from '\.\/wardleyDb\.js';/g, "import type { WardleyDb } from './adapter.js';"],
    [/typeof WardleyDb/g, 'WardleyDb'],
    [/\s*\/\/ @ts-expect-error - since type is set to undefined we will have error/g, ''],
  ],
};
