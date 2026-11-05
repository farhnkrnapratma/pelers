import { describe, expect, it } from 'vitest';
import { ClassMember } from '../../src/diagrams/class/members.js';
import { parseGenericTypes } from '../../src/diagrams/common/generics.js';
import { random } from '../support/corpus.js';

// Pele splits members and converts generics in one pass each. These are Mermaid's own versions,
// a backtracking regular expression and a rescanning loop, kept here to compare results with.

const METHOD = /([#+~-])?(.+)\((.*)\)([\s$*])?(.*)([$*])?/;

function mermaidMethod(input: string) {
  const match = METHOD.exec(input);
  if (!match) return undefined;
  const visibility = match[1] ? match[1].trim() : '';
  let id = match[2];
  const parameters = match[3] ? match[3].trim() : '';
  let classifier = match[4] ? match[4].trim() : '';
  let returnType = match[5] ? match[5].trim() : '';
  if (classifier === '') {
    const last = returnType.substring(returnType.length - 1);
    if (/[$*]/.exec(last)) {
      classifier = last;
      returnType = returnType.substring(0, returnType.length - 1);
    }
  }
  id = id.startsWith(' ') ? ' ' + id.trim() : id.trim();
  return { visibility, id, parameters, classifier, returnType };
}

function mermaidGenerics(input: string): string {
  const count = (text: string): number => Math.max(0, text.split('~').length - 1);
  const processSet = (set: string): string => {
    const tildes = count(set);
    let leading = false;
    if (tildes <= 1) return set;
    if (tildes % 2 !== 0 && set.startsWith('~')) {
      set = set.substring(1);
      leading = true;
    }
    const chars = [...set];
    let first = chars.indexOf('~');
    let last = chars.lastIndexOf('~');
    while (first !== -1 && last !== -1 && first !== last) {
      chars[first] = '<';
      chars[last] = '>';
      first = chars.indexOf('~');
      last = chars.lastIndexOf('~');
    }
    if (leading) chars.unshift('~');
    return chars.join('');
  };
  const sets = input.split(/(,)/);
  const output: string[] = [];
  for (let i = 0; i < sets.length; i++) {
    let set = sets[i];
    if (set === ',' && i > 0 && i + 1 < sets.length && count(sets[i - 1]) === 1 && count(sets[i + 1]) === 1) {
      set = sets[i - 1] + ',' + sets[i + 1];
      i++;
      output.pop();
    }
    output.push(processSet(set));
  }
  return output.join('');
}

const PIECES = [
  '(',
  ')',
  '()',
  '+',
  '-',
  '#',
  '~',
  '$',
  '*',
  ' ',
  '  ',
  ':',
  ',',
  'a',
  'foo',
  'int',
  'List',
  '\r',
  '\n',
  String.fromCharCode(0x2028),
  '\t',
  'é',
];

function randomText(rnd: () => number, pieces: string[], max: number): string {
  let s = '';
  const n = 1 + Math.floor(rnd() * max);
  for (let i = 0; i < n; i++) s += pieces[Math.floor(rnd() * pieces.length)];
  return s;
}

describe('class members', () => {
  it('splits methods as Mermaid does', () => {
    const rnd = random(11);
    let matched = 0;
    for (let i = 0; i < 40000; i++) {
      const input = randomText(rnd, PIECES, 9);
      const expected = mermaidMethod(input);
      const member = new ClassMember(input, 'method');
      if (expected === undefined) {
        expect(member.memberType, JSON.stringify(input)).toBe('attribute');
        continue;
      }
      matched++;
      const { visibility, id, parameters, classifier, returnType } = member;
      expect({ visibility, id, parameters, classifier, returnType }, JSON.stringify(input)).toEqual(expected);
    }
    expect(matched).toBeGreaterThan(5000);
  });

  it('converts generics as Mermaid does', () => {
    const rnd = random(5);
    const pieces = ['~', '~', ',', ', ', 'a', 'List', 'K', ' ', '<', '>', '~T~', '(', ')'];
    for (let i = 0; i < 40000; i++) {
      const input = randomText(rnd, pieces, 12);
      expect(parseGenericTypes(input), JSON.stringify(input)).toBe(mermaidGenerics(input));
    }
  });

  it('keeps a method with no opening parenthesis as an attribute', () => {
    // Mermaid throws a TypeError for these.
    for (const input of ['run)', '(a)', '+x)', ')(', 'a)b']) {
      const member = new ClassMember(input, 'method');
      expect(member.memberType).toBe('attribute');
      expect(member.getDisplayDetails().displayText).toBe(input);
    }
  });

  it('reads visibility and classifiers of attributes', () => {
    const member = new ClassMember('-List~int~ items$', 'attribute');
    expect(member.visibility).toBe('-');
    expect(member.classifier).toBe('$');
    expect(member.getDisplayDetails()).toEqual({
      displayText: '-List<int> items',
      cssStyle: 'text-decoration:underline;',
    });
    expect(new ClassMember('', 'attribute').getDisplayDetails().displayText).toBe('');
  });
});
