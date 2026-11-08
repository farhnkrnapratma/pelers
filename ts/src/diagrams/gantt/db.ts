import { PeleError } from '../../errors.js';
import { safeUrl } from '../../util/url.js';
import { WEEKDAYS, add, addDays, format, isValid, isoDate, parseStrict, type Unit } from './dates.js';
import type { GanttModel, GanttTask } from './types.js';

export interface GanttDbOptions {
  warn?: (message: string) => void;
}

type Tag = 'active' | 'done' | 'crit' | 'milestone' | 'vert';

const RE_NUMERIC_FORMAT = /^[YMDHhmsSZXx\-/.:, T]*$/;
const RE_TAG = /^\s*(active|done|crit|milestone|vert)\s*$/;
const RE_AFTER = /^after\s+([\d\w- ]+)/;
const RE_UNTIL = /^until\s+([\d\w- ]+)/;
const RE_DURATION = /^(\d+(?:\.\d+)?)([Mdhmswy]|ms)$/;
const RE_TOKEN_SPLIT = /[\s,]+/;
const RE_DIGITS = /^\d+$/;
const DAY_NAMES = new Set(WEEKDAYS.map((day) => day.toLowerCase()));

// Scheduling walks tasks day by day and parses dates with a format the diagram supplies. This bounds the
// total, in rough character operations, so a hostile diagram fails instead of hanging.
const WORK_LIMIT = 4e8;
const DAY_STEP = 400;
const DATED_STEP = 200;
const FORMAT_STEP = 400;

// The one place the current time is read.
export function clock(now?: number | Date): number {
  const given = now === undefined ? NaN : +now;
  return Number.isFinite(given) ? given : Date.now();
}

export function parseDuration(str: string): [number, Unit] {
  const statement = RE_DURATION.exec(str.trim());
  return statement ? [Number.parseFloat(statement[1]), statement[2] as Unit] : [NaN, 'ms'];
}

function mergeTokens(existing: string[], txt: string): string[] {
  const merged = new Set(existing);
  for (const token of txt.toLowerCase().split(RE_TOKEN_SPLIT)) if (token !== '') merged.add(token);
  return [...merged];
}

export class GanttDb implements GanttModel {
  readonly type = 'gantt' as const;
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  dateFormat = '';
  axisFormat = '';
  tickInterval: string | undefined;
  todayMarker = '';
  includes: string[] = [];
  excludes: string[] = [];
  inclusiveEndDates = false;
  topAxis = false;
  displayMode = '';
  weekday: string | undefined;
  weekend = 'saturday';
  sections: string[] = [];
  links = new Map<string, string>();
  tasks: GanttTask[] = [];

  private byId = new Map<string, GanttTask>();
  private currentSection = '';
  private taskCount = 0;
  private lastOrder = 0;
  private lastTaskId: string | undefined;
  private options: GanttDbOptions;
  private now = 0;
  private work = 0;
  private includeSet = new Set<string>();
  private excludeSet = new Set<string>();
  private excludedDays: boolean[] = [];
  private datedTokens = false;

  constructor(options: GanttDbOptions = {}) {
    this.options = options;
  }

  setDateFormat(txt: string): void {
    this.dateFormat = txt;
  }

  enableInclusiveEndDates(): void {
    this.inclusiveEndDates = true;
  }

  // Mermaid's grammar calls a method its database does not define, so `topAxis` crashes there.
  TopAxis(): void {
    this.topAxis = true;
  }

  setAxisFormat(txt: string): void {
    this.axisFormat = txt;
  }

  setTickInterval(txt: string): void {
    this.tickInterval = txt;
  }

  setTodayMarker(txt: string): void {
    this.todayMarker = txt;
  }

  setIncludes(txt: string): void {
    this.includes = mergeTokens(this.includes, txt);
  }

  setExcludes(txt: string): void {
    this.excludes = mergeTokens(this.excludes, txt);
  }

  setWeekday(txt: string): void {
    this.weekday = txt;
  }

  setWeekend(startDay: string): void {
    this.weekend = startDay;
  }

  setDiagramTitle(txt: string): void {
    this.title = txt;
  }

  setAccTitle(txt: string): void {
    this.accTitle = txt.replace(/^\s+/g, '');
  }

  setAccDescription(txt: string): void {
    this.accDescr = txt.replace(/\n\s+/g, '\n');
  }

  addSection(txt: string): void {
    this.currentSection = txt;
    this.sections.push(txt);
  }

  addTask(descr: string, dataStr: string): void {
    const data = (dataStr.startsWith(':') ? dataStr.slice(1) : dataStr).split(',');
    const task: GanttTask = {
      id: '',
      task: descr,
      section: this.currentSection,
      startTime: undefined,
      endTime: undefined,
      renderEndTime: null,
      manualEndTime: false,
      processed: false,
      active: false,
      done: false,
      crit: false,
      milestone: false,
      vert: false,
      order: -1,
      classes: [],
      prevTaskId: this.lastTaskId,
      raw: { data: dataStr, start: undefined, end: '' },
    };

    let tag: RegExpExecArray | null;
    while (data.length > 0 && (tag = RE_TAG.exec(data[0])) !== null) {
      task[tag[1] as Tag] = true;
      data.shift();
    }
    // Mermaid fails with a TypeError on both of these.
    if (data.length === 0) {
      throw new PeleError(`Task "${descr.trim()}" needs a duration or an end date.`, 'semantic', { type: 'gantt' });
    }
    if (data.length > 3) {
      throw new PeleError(
        `Task "${descr.trim()}" has too many fields. A task takes an id, a start, and an end or duration.`,
        'semantic',
        { type: 'gantt' }
      );
    }
    for (let i = 0; i < data.length; i++) data[i] = data[i].trim();

    if (data.length === 3) {
      task.id = data[0];
    } else {
      this.taskCount++;
      task.id = 'task' + this.taskCount;
    }
    if (data.length > 1) task.raw.start = data[data.length - 2];
    task.raw.end = data[data.length - 1];

    if (!task.vert) task.order = this.lastOrder++;
    this.tasks.push(task);
    this.lastTaskId = task.id;
    this.byId.set(task.id, task);
  }

  setLink(ids: string, link: string): void {
    const href = safeUrl(link);
    for (const id of ids.split(',')) {
      if (this.byId.has(id)) this.links.set(id, href);
    }
    this.setClass(ids, 'clickable');
  }

  // Callbacks are recognized and never run; the task is only marked clickable, as in Mermaid.
  setClickEvent(ids: string, _functionName: string, _functionArgs: string | null): void {
    this.setClass(ids, 'clickable');
  }

  setClass(ids: string, className: string): void {
    for (const id of ids.split(',')) this.byId.get(id)?.classes.push(className);
  }

  // Resolves every task's start and end, from scratch on each call. A task may refer to tasks defined
  // after it, so, like Mermaid, this makes up to eleven passes, each of which schedules every task again.
  getTasks(now?: number | Date): GanttTask[] {
    this.now = clock(now);
    this.work = 0;
    this.includeSet = new Set(this.includes);
    this.excludeSet = new Set(this.excludes);
    const weekendStart = this.weekend === 'friday' ? 5 : this.weekend === 'saturday' ? 6 : NaN;
    const weekends = this.excludeSet.has('weekends');
    this.excludedDays = WEEKDAYS.map((name, day) => {
      const iso = day || 7;
      return (
        (weekends && (iso === weekendStart || iso === weekendStart + 1)) || this.excludeSet.has(name.toLowerCase())
      );
    });
    // A date written with only these tokens and separators can never read as a weekday name, so when
    // nothing else is excluded or included there is no need to format each day.
    this.datedTokens = this.includes.length > 0 || !RE_NUMERIC_FORMAT.test(this.dateFormat.trim());
    for (const token of this.excludes) if (token !== 'weekends' && !DAY_NAMES.has(token)) this.datedTokens = true;

    for (const task of this.tasks) {
      task.startTime = task.endTime = undefined;
      task.renderEndTime = null;
      task.processed = task.manualEndTime = false;
    }

    let allProcessed = this.compileTasks();
    for (let i = 0; !allProcessed && i < 10; i++) allProcessed = this.compileTasks();
    return this.tasks;
  }

  // Whether a day is excluded. A day matches a token by its weekday name, by its date as YYYY-MM-DD,
  // or by its date written in the chart's own format.
  isInvalidDate(date: Date): boolean {
    const weekday = this.excludedDays[date.getDay()];
    if (!this.datedTokens) return weekday;
    const template = this.dateFormat.trim();
    const dateOnly = isoDate(date);
    let formatted: string | undefined = dateOnly;
    this.spend(DATED_STEP);
    if (template !== 'YYYY-MM-DD') {
      this.spend(FORMAT_STEP + template.length * 8);
      formatted = format(date, template);
    }
    const listed = (set: Set<string>): boolean => set.has(dateOnly) || (formatted !== undefined && set.has(formatted));
    if (listed(this.includeSet)) return false;
    return weekday || listed(this.excludeSet);
  }

  private spend(units: number): void {
    this.work += units;
    if (this.work > WORK_LIMIT) {
      throw new PeleError('The gantt chart takes too much work to schedule.', 'limit', { type: 'gantt' });
    }
  }

  private parse(str: string, template: string): Date | undefined {
    this.spend((template.length + 1) * (str.length + 1));
    return parseStrict(str, template, new Date(this.now));
  }

  private today(): Date {
    const today = new Date(this.now);
    today.setHours(0, 0, 0, 0);
    return today;
  }

  private warn(message: () => string): void {
    if (this.options.warn) this.options.warn(message());
  }

  private compileTasks(): boolean {
    let allProcessed = true;
    for (const task of this.tasks) {
      this.compileTask(task);
      allProcessed = allProcessed && task.processed;
    }
    return allProcessed;
  }

  private compileTask(task: GanttTask): void {
    const start = task.raw.start;
    if (start === undefined) {
      // Mermaid fails with a TypeError when the first task has no start. It starts today here.
      const previous = task.prevTaskId === undefined ? undefined : this.byId.get(task.prevTaskId);
      task.startTime = previous ? previous.endTime : this.today();
    } else {
      const startTime = this.getStartDate(start);
      if (startTime) task.startTime = startTime;
    }
    if (!task.startTime) return;
    task.endTime = this.getEndDate(task.startTime, task.raw.end);
    if (!task.endTime) return;
    task.processed = true;
    task.manualEndTime = this.parse(task.raw.end, 'YYYY-MM-DD') !== undefined;
    this.checkTaskDates(task);
  }

  private references(keyword: 'after' | 'until', list: string): GanttTask[] {
    const found: GanttTask[] = [];
    let unknown: string[] | undefined;
    for (const id of list.split(' ')) {
      if (id === '') continue;
      const task = this.byId.get(id);
      if (task) found.push(task);
      else (unknown ??= []).push(id);
    }
    if (unknown) {
      const ids = unknown;
      this.warn(() => `Gantt: "${keyword}" refers to task ids that do not exist: ${ids.join(', ')}.`);
    }
    return found;
  }

  private getStartDate(raw: string): Date | undefined {
    const str = raw.trim();
    const template = this.dateFormat.trim();

    if ((template === 'x' || template === 'X') && RE_DIGITS.test(str)) return this.checked(new Date(Number(str)), str);

    const after = RE_AFTER.exec(str);
    if (after !== null) {
      const tasks = this.references('after', after[1]);
      let latest: GanttTask | undefined;
      for (const task of tasks) {
        if (!latest || task.endTime! > latest.endTime!) latest = task;
      }
      return latest ? latest.endTime : this.today();
    }

    const parsed = this.parse(str, template);
    if (parsed) return parsed;

    // What does not fit the format gets a second chance with the JavaScript date parser, as in Mermaid.
    const date = new Date(str);
    if (!isValid(date) || date.getFullYear() < -10000 || date.getFullYear() > 10000) {
      throw new PeleError('Invalid date:' + str, 'semantic', { type: 'gantt' });
    }
    return date;
  }

  private checked(date: Date, str: string): Date {
    if (!isValid(date)) throw new PeleError('Invalid date:' + str, 'semantic', { type: 'gantt' });
    return date;
  }

  private getEndDate(startTime: Date, raw: string): Date | undefined {
    const str = raw.trim();

    const until = RE_UNTIL.exec(str);
    if (until !== null) {
      const tasks = this.references('until', until[1]);
      let earliest: GanttTask | undefined;
      for (const task of tasks) {
        if (!earliest || task.startTime! < earliest.startTime!) earliest = task;
      }
      return earliest ? earliest.startTime : this.today();
    }

    const template = this.dateFormat.trim();
    const parsed = this.parse(str, template);
    if (parsed) return this.inclusiveEndDates ? addDays(parsed, 1) : parsed;

    const [value, unit] = parseDuration(str);
    if (Number.isNaN(value)) {
      this.warn(
        () =>
          `Gantt: "${str}" is neither a date in the "${template}" format nor a duration such as "3d", so the task gets no duration.`
      );
      return new Date(startTime);
    }
    const end = add(startTime, value, unit);
    return isValid(end) ? end : new Date(startTime);
  }

  // Pushes the end of a task out by one day for every excluded day it covers.
  private checkTaskDates(task: GanttTask): void {
    if (this.excludes.length === 0 || task.manualEndTime) return;
    const cursor = addDays(task.startTime!, 1);
    const endTime = new Date(task.endTime!);
    const maxEndTime = +addDays(endTime, 10000);
    let invalid = false;
    let renderEndTime: number | null = null;
    while (+cursor <= +endTime) {
      this.spend(DAY_STEP);
      if (!invalid) renderEndTime = +endTime;
      invalid = this.isInvalidDate(cursor);
      if (invalid) {
        endTime.setDate(endTime.getDate() + 1);
        if (+endTime > maxEndTime) {
          throw new PeleError(
            'Failed to find a valid date that was not excluded by `excludes` after 10,000 iterations.',
            'semantic',
            { type: 'gantt' }
          );
        }
      }
      cursor.setDate(cursor.getDate() + 1);
    }
    task.endTime = endTime;
    task.renderEndTime = renderEndTime === null ? null : new Date(renderEndTime);
  }
}
