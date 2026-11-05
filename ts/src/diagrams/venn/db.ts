import { PeleError } from '../../errors.js';
import type { VennModel, VennStyle, VennSubset, VennText } from './types.js';

// Mermaid takes one pair of quotes off an id, label or style value.
export function normalizeText(text: string): string {
  const trimmed = text.trim();
  return trimmed.length >= 2 && trimmed.startsWith('"') && trimmed.endsWith('"') ? trimmed.slice(1, -1) : trimmed;
}

function normalizeIds(ids: string[]): string[] {
  return ids.map(normalizeText).sort();
}

export class VennDb implements VennModel {
  readonly type = 'venn' as const;
  title: string | undefined;
  subsets: VennSubset[] = [];
  textNodes: VennText[] = [];
  styles: VennStyle[] = [];
  private known = new Set<string>();
  private current: string[] | undefined;
  // Whether an indented `text` line belongs to the set or union before it. The lexer reads this.
  private indent = false;

  setDiagramTitle(title: string): void {
    this.title = title;
  }

  // A set without a size gets 10, and an overlap of n sets 10 / n².
  addSubsetData(ids: string[], label: string | undefined, size: number | undefined): void {
    const sets = normalizeIds(ids);
    this.current = sets;
    if (sets.length === 1) this.known.add(sets[0]);
    this.subsets.push({ sets, size: size ?? 10 / ids.length ** 2, label: label ? normalizeText(label) : undefined });
  }

  addTextData(ids: string[], id: string, label: string | undefined): void {
    this.textNodes.push({
      sets: normalizeIds(ids),
      id: normalizeText(id),
      label: label ? normalizeText(label) : undefined,
    });
  }

  addStyleData(ids: string[], data: [string, string][]): void {
    const styles = new Map<string, string>();
    for (const [key, value] of data) styles.set(key, value ? normalizeText(value) : value);
    this.styles.push({ targets: normalizeIds(ids), styles });
  }

  validateUnionIdentifiers(ids: string[]): void {
    const unknown = ids.map(normalizeText).filter((id) => !this.known.has(id));
    if (unknown.length > 0) {
      throw new PeleError(`unknown set identifier: ${unknown.join(', ')}`, 'semantic', { type: 'venn' });
    }
  }

  getCurrentSets(): string[] | undefined {
    return this.current;
  }

  getIndentMode(): boolean {
    return this.indent;
  }

  setIndentMode(enabled: boolean): void {
    this.indent = enabled;
  }
}
