import type { Diagram } from '../../types.js';
import { buildTreeView, readTreeView, type TreeViewModel } from './model.js';
import { renderTreeView } from './render.js';

export const treeView: Diagram<TreeViewModel> = {
  type: 'treeView',
  parse: (source, _config, title) => buildTreeView(readTreeView(source), title),
  render: renderTreeView,
};
