export default {
  files: [['cynefin', /\.spec\.ts$/]],
  docs: 'cynefin.md',
  skip: [],
  notPorted: [],
  rewrite: [
    [/from '\.\/cynefin(Db|Parser|Boundaries)\.js'/g, "from './adapter.js'"],
    [
      /import type \{ DomainBlock, Transition \} from '@mermaid-js\/parser';/g,
      "import type { DomainBlock, Transition } from './adapter.js';",
    ],
  ],
};
