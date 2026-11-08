import { readFileSync, readdirSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { examplePages } from '../../website/src/lib/examples.js';

const corpora = Object.fromEntries(
  readdirSync('tests/corpus')
    .filter((file) => file.endsWith('-docs.json'))
    .map((file) => [`../../../tests/corpus/${file}`, JSON.parse(readFileSync(`tests/corpus/${file}`, 'utf8'))])
);

describe('example pages', () => {
  const pages = examplePages(corpora);

  it('has a page for every type with examples, except info', () => {
    expect(pages.map((page) => page.type)).not.toContain('info');
    expect(pages.length).toBe(Object.keys(corpora).length - 1);
  });
});
