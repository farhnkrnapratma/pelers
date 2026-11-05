export default {
  files: [
    ['gantt', /^ganttDb\.spec\.ts$/],
    ['gantt/parser', /^gantt\.spec\.js$/],
  ],
  grammars: ['gantt/parser/gantt.jison'],
  docs: 'gantt.md',
  rewrite: [
    [/import \{ parser \} from '\.\/gantt\.jison';/g, "import { parser } from './adapter.js';"],
    [/import ganttDb from '(?:\.\.\/|\.\/)ganttDb\.js';/g, "import { ganttDb } from './adapter.js';"],
    [/import dayjs from 'dayjs';/g, "import { dayjs } from './adapter.js';"],
    [/from '\.\.\/\.\.\/tests\/util\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/logger\.js'/g, "from './adapter.js'"],
  ],
};
