import type { Diagram } from '../../types.js';
import { buildGit, type GitModel } from './db.js';
import { parseGit } from './parser.js';
import { renderGit } from './render.js';

export const gitGraph: Diagram<GitModel> = {
  type: 'gitGraph',
  parse: (source, config, title) => buildGit(parseGit(source), config, title),
  render: renderGit,
};
