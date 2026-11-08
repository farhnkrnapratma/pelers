// Two rules, as in Mermaid: a node takes the color of its kind, from a fixed slot, so editing the
// diagram around it never recolors it; containers count through the slots above the kinds, in the
// order they were declared, so a frame does not take the color of a node inside it.

export const KIND_SLOT: ReadonlyMap<string, number> = new Map([
  ['tool', 0],
  ['task', 1],
  ['decision', 2],
  ['input', 3],
  ['refdoc', 4],
  ['connector', 5],
  ['action', 6],
]);

export const KIND_COUNT = KIND_SLOT.size;

// At least one, so that a palette shorter than the kinds still gives containers a slot.
export const containerSlotCount = (paletteLength: number): number => Math.max(1, paletteLength - KIND_COUNT);

export const containerSlot = (n: number, paletteLength: number): number => {
  const slot = KIND_COUNT + (n % containerSlotCount(paletteLength));
  return paletteLength > 0 ? slot % paletteLength : slot;
};

interface SlotNode {
  id: string;
  shape?: string;
  isGroup?: boolean;
  colorIndex?: number;
  kind?: string;
}

export function assignColorSlots(
  nodes: SlotNode[],
  kindOf: (id: string) => string | undefined,
  containerOrder: ReadonlyMap<string, number>,
  paletteLength: number
): void {
  let fallback = containerOrder.size;
  for (const node of nodes) {
    // A collapsed container is drawn as a node but keeps its slot, so collapsing one does not
    // recolor the containers after it.
    if (node.isGroup || node.shape === 'collapsedGroup') {
      node.colorIndex = containerSlot(containerOrder.get(node.id) ?? fallback++, paletteLength);
      continue;
    }
    const kind = kindOf(node.id);
    if (kind && KIND_SLOT.has(kind)) node.kind = kind;
  }
}
