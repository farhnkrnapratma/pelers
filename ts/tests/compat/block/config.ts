// Stands in for Mermaid's diagram-api module, which the specs spy on to change `block.padding`.
export interface MermaidConfig {
  block?: { padding?: number };
}

let config: MermaidConfig = {};

export function setConfig(next: MermaidConfig): void {
  config = next;
}

export function getConfig(): MermaidConfig {
  return config;
}
