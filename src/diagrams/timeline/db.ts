import { PeleError } from '../../errors.js';

export interface TimelinePeriod {
  text: string;
  events: string[];
  // Name of the section in force when the period was declared, '' before the first one.
  section: string;
  // Index into `sections` of that section, -1 before the first one.
  sectionIndex: number;
}

export interface TimelineModel {
  type: 'timeline';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  direction: 'LR' | 'TD';
  sections: string[];
  periods: TimelinePeriod[];
}

export class TimelineDb implements TimelineModel {
  readonly type = 'timeline' as const;
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  direction: 'LR' | 'TD' = 'LR';
  sections: string[] = [];
  periods: TimelinePeriod[] = [];

  private currentSection = '';

  // The grammar reaches the title and accessibility setters through this.
  getCommonDb(): this {
    return this;
  }

  setDiagramTitle(text: string): void {
    this.title = text;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }

  setDirection(direction: 'LR' | 'TD'): void {
    this.direction = direction;
  }

  addSection(text: string): void {
    this.currentSection = text;
    this.sections.push(text);
  }

  addTask(period: string, _length?: number, event?: string): void {
    this.periods.push({
      text: period,
      events: event ? [event] : [],
      section: this.currentSection,
      sectionIndex: this.sections.length - 1,
    });
  }

  addEvent(event: string): void {
    const period = this.periods[this.periods.length - 1];
    // Mermaid fails here too, with a TypeError.
    if (!period) throw new PeleError('An event needs a time period before it.', 'semantic', { type: 'timeline' });
    period.events.push(event);
  }
}
