import type { Config } from '../../preprocess.js';
import type { Diagram } from '../../types.js';
import { AgentflowDb } from './db.js';
import { parseAgentflow } from './parser.js';
import { agentGraph, renderAgentflow } from './render.js';

export const agentflow: Diagram<AgentflowDb> = {
  type: 'agentflow',
  keepComments: true,
  parse(source, config, title, lineOffset, limits) {
    // Mermaid reads this setting from the flowchart section for agentflow too.
    const flow = (config.flowchart ?? {}) as Config;
    const db = new AgentflowDb({
      inheritDir: flow.inheritDir === true,
      direction: typeof config.direction === 'string' ? config.direction : undefined,
      lineOffset,
      source,
      maxEdges: limits.maxEdges,
    });
    if (title) db.title = title;
    parseAgentflow(source, db);
    agentGraph(db, config);
    return db;
  },
  render: renderAgentflow,
};
