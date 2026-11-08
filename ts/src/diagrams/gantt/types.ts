export interface GanttTask {
  id: string;
  // The task's label, under the name Mermaid gives it.
  task: string;
  section: string;
  // Unset until `getTasks` has scheduled the task, and still unset when what it refers to never resolves.
  startTime: Date | undefined;
  endTime: Date | undefined;
  // Where the bar ends when excluded days pushed the task's end out; null when the end was given as a date.
  renderEndTime: Date | null;
  manualEndTime: boolean;
  processed: boolean;
  active: boolean;
  done: boolean;
  crit: boolean;
  milestone: boolean;
  vert: boolean;
  // Row index in source order; -1 for vertical markers, which take no row.
  order: number;
  classes: string[];
  prevTaskId: string | undefined;
  raw: { data: string; start: string | undefined; end: string };
}

export interface GanttModel {
  type: 'gantt';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  dateFormat: string;
  axisFormat: string;
  tickInterval: string | undefined;
  todayMarker: string;
  includes: string[];
  excludes: string[];
  inclusiveEndDates: boolean;
  topAxis: boolean;
  displayMode: string;
  // The day week-based ticks start on, when the chart names one. Mermaid's default is Sunday.
  weekday: string | undefined;
  weekend: string;
  sections: string[];
  links: Map<string, string>;
  tasks: GanttTask[];
  // Schedules the tasks against the given time, or the current time, and returns them.
  getTasks(now?: number | Date): GanttTask[];
}
