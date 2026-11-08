import { describe, expect, it } from 'vitest';
import { playgroundHref, readPlaygroundLink } from '../../website/src/lib/playground-link.js';
import { playgroundLimits } from '../../website/src/lib/playground-limits.js';

const hashOf = (href: string) => href.slice(href.indexOf('#'));

describe('playground links', () => {
  it('carries the source and CSS', () => {
    const href = playgroundHref('flowchart LR\n  a --> b', '.pele { --pele-line: red; }');
    expect(readPlaygroundLink(hashOf(href))).toEqual({
      source: 'flowchart LR\n  a --> b',
      css: '.pele { --pele-line: red; }',
    });
  });

  it('leaves CSS out of links without it', () => {
    const href = playgroundHref('flowchart LR\n  a --> b');
    expect(href).not.toContain('css=');
    expect(readPlaygroundLink(hashOf(href))).toEqual({ source: 'flowchart LR\n  a --> b', css: '' });
  });

  it('rejects CSS over the limit', () => {
    const css = 'a'.repeat(playgroundLimits.source + 1);
    expect(() => playgroundHref('flowchart LR', css)).toThrow('CSS is too large');
    expect(readPlaygroundLink(`#${new URLSearchParams({ source: 'flowchart LR', css })}`)).toBeNull();
  });
});
