import { MONTHS, WEEKDAYS } from './dates.js';

// Time axis support, following d3-time and d3-time-format (English locale, local time), which Mermaid uses:
// the `axisFormat` specifiers, tick intervals with d3's `every` rule, and its choice of automatic ticks.

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;
const MONTH = 30 * DAY;
const YEAR = 365 * DAY;

export type TickUnit = 'millisecond' | 'second' | 'minute' | 'hour' | 'day' | 'week' | 'month' | 'year';

interface Calendar {
  floor(d: Date): void;
  offset(d: Date, step: number): void;
  // The value d3 tests against a step: a tick falls where it is a multiple of the step.
  field?(d: Date): number;
  // How many values `field` cycles through, which bounds how far apart filtered ticks can be.
  cycle: number;
  ms: number;
}

function startOfDay(d: Date): void {
  d.setHours(0, 0, 0, 0);
}

function weekCalendar(weekStart: number): Calendar {
  return {
    floor(d) {
      d.setDate(d.getDate() - ((d.getDay() + 7 - weekStart) % 7));
      startOfDay(d);
    },
    offset: (d, step) => d.setDate(d.getDate() + step * 7),
    cycle: Infinity,
    ms: WEEK,
  };
}

const CALENDARS: Record<Exclude<TickUnit, 'week'>, Calendar> = {
  millisecond: { floor: () => {}, offset: (d, step) => d.setTime(+d + step), cycle: Infinity, ms: 1 },
  second: {
    floor: (d) => d.setTime(+d - d.getMilliseconds()),
    offset: (d, step) => d.setTime(+d + step * SECOND),
    field: (d) => d.getUTCSeconds(),
    cycle: 60,
    ms: SECOND,
  },
  minute: {
    floor: (d) => d.setTime(+d - d.getMilliseconds() - d.getSeconds() * SECOND),
    offset: (d, step) => d.setTime(+d + step * MINUTE),
    field: (d) => d.getMinutes(),
    cycle: 60,
    ms: MINUTE,
  },
  hour: {
    floor: (d) => d.setTime(+d - d.getMilliseconds() - d.getSeconds() * SECOND - d.getMinutes() * MINUTE),
    offset: (d, step) => d.setTime(+d + step * HOUR),
    field: (d) => d.getHours(),
    cycle: 24,
    ms: HOUR,
  },
  day: {
    floor: startOfDay,
    offset: (d, step) => d.setDate(d.getDate() + step),
    field: (d) => d.getDate() - 1,
    cycle: 31,
    ms: DAY,
  },
  month: {
    floor(d) {
      d.setDate(1);
      startOfDay(d);
    },
    offset: (d, step) => d.setMonth(d.getMonth() + step),
    field: (d) => d.getMonth(),
    cycle: 12,
    ms: MONTH,
  },
  year: {
    floor(d) {
      d.setMonth(0, 1);
      startOfDay(d);
    },
    offset: (d, step) => d.setFullYear(d.getFullYear() + step),
    cycle: Infinity,
    ms: YEAR,
  },
};

// Whole units between two floored dates, discounting a change of UTC offset between them.
function span(start: Date, end: Date, unit: number): number {
  return Math.floor((+end - +start - (end.getTimezoneOffset() - start.getTimezoneOffset()) * MINUTE) / unit);
}

function mod(a: number, b: number): number {
  return ((a % b) + b) % b;
}

// The interval d3's `unit.every(step)` gives.
function every(unit: TickUnit, step: number, weekStart: number): Calendar {
  const base = unit === 'week' ? weekCalendar(weekStart) : CALENDARS[unit];
  if (step <= 1) return base;
  if (unit === 'millisecond') {
    return {
      ...base,
      floor: (d) => d.setTime(Math.floor(+d / step) * step),
      offset: (d, n) => d.setTime(+d + n * step),
    };
  }
  if (unit === 'year') {
    return {
      ...base,
      floor(d) {
        d.setFullYear(Math.floor(d.getFullYear() / step) * step);
        base.floor(d);
      },
      offset: (d, n) => d.setFullYear(d.getFullYear() + n * step),
    };
  }
  if (unit === 'week') {
    const epoch = new Date(0);
    base.floor(epoch);
    return {
      ...base,
      floor(d) {
        base.floor(d);
        base.offset(d, -mod(span(epoch, d, WEEK), step));
      },
      offset: (d, n) => base.offset(d, n * step),
    };
  }
  const field = base.field!;
  const test = (d: Date): boolean => field(d) % step === 0;
  return {
    ...base,
    floor(d) {
      // Date overflow makes field(d) NaN, which can never match the interval.
      for (base.floor(d); Number.isFinite(+d) && !test(d); base.floor(d)) d.setTime(+d - 1);
    },
    offset(d, n) {
      for (let i = 0; i < n && Number.isFinite(+d); i++) {
        do base.offset(d, 1);
        while (Number.isFinite(+d) && !test(d));
      }
    },
  };
}

// Ticks from start to stop inclusive, or undefined when there would be more than `limit`.
function range(calendar: Calendar, start: number, stop: number, limit: number): number[] | undefined {
  const d = new Date(start - 1);
  calendar.floor(d);
  calendar.offset(d, 1);
  calendar.floor(d);
  const out: number[] = [];
  let previous = -Infinity;
  while (+d <= stop && +d > previous) {
    if (out.length >= limit) return undefined;
    previous = +d;
    out.push(previous);
    calendar.offset(d, 1);
    calendar.floor(d);
  }
  return out;
}

function tickStep(start: number, stop: number, count: number): number {
  const step = (stop - start) / Math.max(0, count);
  const power = Math.floor(Math.log10(step));
  const error = step / Math.pow(10, power);
  const factor = error >= Math.sqrt(50) ? 10 : error >= Math.sqrt(10) ? 5 : error >= Math.SQRT2 ? 2 : 1;
  return Math.pow(10, power) * factor;
}

const AUTO: [TickUnit, number, number][] = [
  ['second', 1, SECOND],
  ['second', 5, 5 * SECOND],
  ['second', 15, 15 * SECOND],
  ['second', 30, 30 * SECOND],
  ['minute', 1, MINUTE],
  ['minute', 5, 5 * MINUTE],
  ['minute', 15, 15 * MINUTE],
  ['minute', 30, 30 * MINUTE],
  ['hour', 1, HOUR],
  ['hour', 3, 3 * HOUR],
  ['hour', 6, 6 * HOUR],
  ['hour', 12, 12 * HOUR],
  ['day', 1, DAY],
  ['day', 2, 2 * DAY],
  ['week', 1, WEEK],
  ['month', 1, MONTH],
  ['month', 3, 3 * MONTH],
  ['year', 1, YEAR],
];

// About `count` ticks at round times, chosen the way d3's time scale chooses them.
export function autoTicks(start: number, stop: number, count: number): number[] {
  const target = (stop - start) / count;
  let i = 0;
  while (i < AUTO.length && AUTO[i][2] <= target) i++;
  let calendar: Calendar;
  if (i === AUTO.length) {
    calendar = every('year', Math.max(1, tickStep(start / YEAR, stop / YEAR, count)), 0);
  } else if (i === 0) {
    calendar = every('millisecond', Math.max(tickStep(start, stop, count), 1), 0);
  } else {
    const [unit, step] = AUTO[target / AUTO[i - 1][2] < AUTO[i][2] / target ? i - 1 : i];
    calendar = every(unit, step, 0);
  }
  return range(calendar, start, stop, 4 * count + 8) ?? [];
}

const RE_INTERVAL = /^([1-9]\d*)(millisecond|second|minute|hour|day|week|month)$/;
const WEEK_STARTS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];

// Ticks for a `tickInterval` such as `1week`, or undefined when it is not one or asks for more than `limit`.
export function intervalTicks(
  interval: string,
  weekday: string,
  start: number,
  stop: number,
  limit: number
): number[] | undefined {
  const m = RE_INTERVAL.exec(interval);
  if (!m) return undefined;
  const step = parseInt(m[1], 10);
  const unit = m[2] as TickUnit;
  const calendar = every(unit, step, Math.max(0, WEEK_STARTS.indexOf(weekday)));
  if ((stop - start) / (Math.min(step, calendar.cycle) * calendar.ms) > limit) return undefined;
  return range(calendar, start, stop, limit);
}

function pad(value: number, fill: string, width: number): string {
  const sign = value < 0 ? '-' : '';
  const digits = String(sign ? -value : value);
  return sign + (digits.length < width ? fill.repeat(width - digits.length) + digits : digits);
}

function floored(date: Date, calendar: Calendar): Date {
  const d = new Date(+date);
  calendar.floor(d);
  return d;
}

// Weeks since the last `weekStart` day before the year began, which is d3's %U and %W.
function weekOfYear(date: Date, weekStart: number): number {
  const week = weekCalendar(weekStart);
  const yearStart = floored(date, CALENDARS.year);
  return span(floored(new Date(+yearStart - 1), week), floored(date, week), WEEK);
}

// The Thursday that decides which ISO week and ISO year a date belongs to.
function isoThursday(date: Date): Date {
  const thursday = weekCalendar(4);
  const day = date.getDay();
  const d = floored(day >= 4 || day === 0 ? date : new Date(+date - 1), thursday);
  if (!(day >= 4 || day === 0)) {
    thursday.offset(d, 1);
    thursday.floor(d);
  }
  return d;
}

const PADS = new Map([
  ['-', ''],
  ['_', ' '],
  ['0', '0'],
]);

// Formats a date with a d3 time format specifier such as `%Y-%m-%d`.
export function timeFormat(specifier: string, date: Date): string {
  let out = '';
  let from = 0;
  const n = specifier.length;
  for (let i = 0; i < n; i++) {
    if (specifier.charCodeAt(i) !== 37) continue;
    out += specifier.slice(from, i);
    let c = specifier.charAt(++i);
    let fill = PADS.get(c);
    if (fill !== undefined) c = specifier.charAt(++i);
    else fill = c === 'e' ? ' ' : '0';
    out += directive(c, date, fill);
    from = i + 1;
  }
  return out + specifier.slice(from);
}

function directive(c: string, d: Date, p: string): string {
  switch (c) {
    case 'a':
      return WEEKDAYS[d.getDay()].slice(0, 3);
    case 'A':
      return WEEKDAYS[d.getDay()];
    case 'b':
      return MONTHS[d.getMonth()].slice(0, 3);
    case 'B':
      return MONTHS[d.getMonth()];
    case 'c':
      return timeFormat('%x, %X', d);
    case 'd':
    case 'e':
      return pad(d.getDate(), p, 2);
    case 'f':
      return pad(d.getMilliseconds(), p, 3) + '000';
    case 'g':
      return pad(isoThursday(d).getFullYear() % 100, p, 2);
    case 'G':
      return pad(isoThursday(d).getFullYear() % 10000, p, 4);
    case 'H':
      return pad(d.getHours(), p, 2);
    case 'I':
      return pad(d.getHours() % 12 || 12, p, 2);
    case 'j':
      return pad(1 + span(floored(d, CALENDARS.year), floored(d, CALENDARS.day), DAY), p, 3);
    case 'L':
      return pad(d.getMilliseconds(), p, 3);
    case 'm':
      return pad(d.getMonth() + 1, p, 2);
    case 'M':
      return pad(d.getMinutes(), p, 2);
    case 'p':
      return d.getHours() >= 12 ? 'PM' : 'AM';
    case 'q':
      return String(1 + Math.floor(d.getMonth() / 3));
    case 'Q':
      return String(+d);
    case 's':
      return String(Math.floor(+d / 1000));
    case 'S':
      return pad(d.getSeconds(), p, 2);
    case 'u':
      return String(d.getDay() || 7);
    case 'U':
      return pad(weekOfYear(d, 0), p, 2);
    case 'V': {
      const thursday = isoThursday(d);
      const yearStart = floored(thursday, CALENDARS.year);
      const first = floored(yearStart, weekCalendar(4));
      return pad(span(first, thursday, WEEK) + (yearStart.getDay() === 4 ? 1 : 0), p, 2);
    }
    case 'w':
      return String(d.getDay());
    case 'W':
      return pad(weekOfYear(d, 1), p, 2);
    case 'x':
      return timeFormat('%-m/%-d/%Y', d);
    case 'X':
      return timeFormat('%-I:%M:%S %p', d);
    case 'y':
      return pad(d.getFullYear() % 100, p, 2);
    case 'Y':
      return pad(d.getFullYear() % 10000, p, 4);
    case 'Z': {
      const z = d.getTimezoneOffset();
      const abs = Math.abs(z);
      return (z > 0 ? '-' : '+') + pad(Math.floor(abs / 60), '0', 2) + pad(abs % 60, '0', 2);
    }
    case '%':
      return '%';
    default:
      return c;
  }
}
