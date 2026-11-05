import type { IconResolver } from '../types.js';

// The host's icons for the render in progress. A label leaves room for an icon only when the
// host has one by that name, and layout has to know that before anything is drawn.
let resolver: IconResolver | undefined;
let found = new Map<string, string>();

export function withIcons<T>(icons: IconResolver | undefined, run: () => T): T {
  const outer = resolver;
  const outerFound = found;
  resolver = icons;
  found = new Map();
  try {
    return run();
  } finally {
    resolver = outer;
    found = outerFound;
  }
}

export function iconMarkup(name: string): string {
  if (resolver === undefined) return '';
  let markup = found.get(name);
  if (markup === undefined) {
    markup = resolver(name) ?? '';
    found.set(name, markup);
  }
  return markup;
}
