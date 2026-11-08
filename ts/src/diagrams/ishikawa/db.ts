import type { IshikawaModel, IshikawaNode } from './types.js';

export class IshikawaDb implements IshikawaModel {
  readonly type = 'ishikawa' as const;
  title: string | undefined;
  root: IshikawaNode | undefined;
  // The line of ancestors of the last node, each with its level.
  private stack: { level: number; node: IshikawaNode }[] = [];
  private baseLevel: number | undefined;

  // The first line is the effect. Every later line is a cause, nested by its indentation relative
  // to the first cause, so the effect itself may be indented more than the causes.
  addNode(rawLevel: number, text: string): void {
    const node: IshikawaNode = { text, children: [] };
    if (!this.root) {
      this.root = node;
      this.stack = [{ level: 0, node }];
      return;
    }
    this.baseLevel ??= rawLevel;
    const level = Math.max(rawLevel - this.baseLevel + 1, 1);
    const stack = this.stack;
    while (stack.length > 1 && stack[stack.length - 1].level >= level) stack.pop();
    stack[stack.length - 1].node.children.push(node);
    stack.push({ level, node });
  }
}
