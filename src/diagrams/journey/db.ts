export interface JourneyTask {
  task: string;
  // As written: any number, or NaN when the score is not one.
  score: number;
  people: string[];
  // Name of the section in force when the task was declared, '' before the first one.
  section: string;
}

export interface JourneyModel {
  type: 'journey';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  sections: string[];
  tasks: JourneyTask[];
}

// Everyone named on a task, once each, in code unit order.
export function journeyActors(tasks: JourneyTask[]): string[] {
  const names = new Set<string>();
  for (const task of tasks) for (const name of task.people) names.add(name);
  return [...names].sort();
}

export class JourneyDb implements JourneyModel {
  readonly type = 'journey' as const;
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  sections: string[] = [];
  tasks: JourneyTask[] = [];

  private currentSection = '';

  setDiagramTitle(text: string): void {
    this.title = text;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }

  addSection(text: string): void {
    this.currentSection = text;
    this.sections.push(text);
  }

  // `data` is the rest of the line from its first colon: `:score` or `:score:actor, actor`.
  addTask(task: string, data: string): void {
    const pieces = data.substring(1).split(':');
    const people = pieces.length === 1 ? [] : pieces[1].split(',').map((name) => name.trim());
    this.tasks.push({ task, score: Number(pieces[0]), people, section: this.currentSection });
  }

  getActors(): string[] {
    return journeyActors(this.tasks);
  }
}
