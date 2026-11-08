import { register } from './core.js';
import { all } from './diagrams/registry.js';

register(...all);

export * from './core.js';
export type * from './models.js';
