import { PeleError } from '../../errors.js';
import { parseYaml, type YamlValue } from '../../util/yaml.js';
import { getType } from '../common/outline.js';

export interface KanbanNode {
  id: string;
  level: number;
  label: string;
  icon?: string;
  assigned?: string;
  ticket?: string;
  priority?: string;
  cssClasses?: string;
}

export interface KanbanSection extends KanbanNode {
  items: KanbanNode[];
}

export interface KanbanModel {
  type: 'kanban';
  title: string | undefined;
  sections: KanbanSection[];
}

export class KanbanDb implements KanbanModel {
  readonly type = 'kanban';
  title: string | undefined;
  sections: KanbanSection[] = [];
  getType = getType;

  private last: KanbanNode | undefined;
  private sectionLevel = 0;
  private misplaced: KanbanNode | undefined;

  addNode(level: number, id: string, descr: string, _type: number, shapeData?: string): void {
    const node: KanbanNode = { id, level, label: descr };

    if (shapeData !== undefined) {
      const yaml = shapeData.includes('\n') ? shapeData + '\n' : '{\n' + shapeData + '\n}';
      let doc: YamlValue;
      try {
        doc = parseYaml(yaml);
      } catch (error) {
        throw new PeleError(`Metadata of "${node.id}" is not valid YAML. ${(error as Error).message}`, 'syntax', {
          type: 'kanban',
        });
      }
      // Mermaid throws a TypeError on metadata that is empty or not a mapping. Here it is ignored.
      if (doc !== null && typeof doc === 'object' && !Array.isArray(doc)) {
        const shape = doc.shape ? String(doc.shape) : '';
        if (shape !== shape.toLowerCase() || shape.includes('_')) {
          throw new PeleError(`No such shape: ${shape}. Shape names should be lowercase.`, 'semantic', {
            type: 'kanban',
          });
        }
        if (doc.label) node.label = String(doc.label);
        if (doc.icon) node.icon = String(doc.icon);
        if (doc.assigned) node.assigned = String(doc.assigned);
        if (doc.ticket) node.ticket = String(doc.ticket);
        if (doc.priority) node.priority = String(doc.priority);
      }
    }

    // A node indented less than the first column becomes a card, and Mermaid rejects whatever follows it.
    if (this.misplaced) {
      throw new PeleError(`Items without section detected, found section ("${this.misplaced.label}")`, 'semantic', {
        type: 'kanban',
      });
    }
    const sections = this.sections;
    if (sections.length === 0) this.sectionLevel = level;
    this.last = node;
    if (level === this.sectionLevel) {
      const section = node as KanbanSection;
      section.items = [];
      sections.push(section);
    } else {
      sections[sections.length - 1].items.push(node);
      if (level < this.sectionLevel) this.misplaced = node;
    }
  }

  // Mermaid throws a TypeError when a decoration comes before any node. Here it is ignored.
  decorateNode(decoration?: { class?: string; icon?: string }): void {
    const node = this.last;
    if (!decoration || !node) return;
    if (decoration.icon) node.icon = decoration.icon;
    if (decoration.class) node.cssClasses = decoration.class;
  }
}
