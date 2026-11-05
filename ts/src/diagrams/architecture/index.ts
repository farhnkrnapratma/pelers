import type { Diagram } from '../../types.js';
import { ArchitectureDb, populate } from './db.js';
import { parseArchitecture } from './parser.js';
import { renderArchitecture } from './render.js';

export const architecture: Diagram<ArchitectureDb> = {
  type: 'architecture',
  parse(source, _config, title) {
    const db = new ArchitectureDb();
    if (title) db.title = title;
    populate(parseArchitecture(source), db);
    return db;
  },
  render: renderArchitecture,
};
