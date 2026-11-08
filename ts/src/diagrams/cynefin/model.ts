import type { CynefinAst, DomainName } from './parser.js';

export type { DomainName } from './parser.js';

export interface CynefinItem {
  label: string;
}

export interface CynefinDomain {
  name: DomainName;
  items: CynefinItem[];
}

export interface CynefinTransition {
  from: DomainName;
  to: DomainName;
  label?: string;
}

export interface CynefinModel {
  type: 'cynefin';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  // The domains the source declares. All five are drawn either way.
  domains: Map<DomainName, CynefinDomain>;
  transitions: CynefinTransition[];
}

export interface DomainBlock {
  domain: string;
  items?: { label: string }[];
}

export interface Transition {
  from: string;
  to: string;
  label?: string;
}

export class CynefinDb {
  domains = new Map<DomainName, CynefinDomain>();
  transitions: CynefinTransition[] = [];

  // A later block for a domain replaces an earlier one.
  setDomains(blocks: DomainBlock[] | null | undefined): void {
    if (!blocks) return;
    for (const block of blocks) {
      const name = block.domain as DomainName;
      this.domains.set(name, { name, items: (block.items ?? []).map((item) => ({ label: item.label })) });
    }
  }

  setTransitions(transitions: Transition[] | null | undefined): void {
    if (!transitions) return;
    this.transitions = transitions
      .filter((t) => t.from !== t.to)
      .map((t) => ({ from: t.from as DomainName, to: t.to as DomainName, label: t.label || undefined }));
  }
}

export function buildCynefin(ast: CynefinAst, title: string | undefined): CynefinModel {
  const db = new CynefinDb();
  db.setDomains(ast.domains);
  db.setTransitions(ast.transitions);
  return {
    type: 'cynefin',
    title: ast.title || title,
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : undefined,
    domains: db.domains,
    transitions: db.transitions,
  };
}
