import { describe, expect, it } from 'vitest';
import { cleanupComments, encodeEntities, preprocess } from '../../src/preprocess.js';
import { random } from '../support/corpus.js';

// Mermaid's own encodeEntities, kept here as the reference. Its first two regexes backtrack
// badly on long lines, so the comparison uses short ones.
function reference(text: string): string {
  let txt = text;
  txt = txt.replace(/style.*:\S*#.*;/g, (s) => s.substring(0, s.length - 1));
  txt = txt.replace(/classDef.*:\S*#.*;/g, (s) => s.substring(0, s.length - 1));
  txt = txt.replace(/#\w+;/g, (s) => {
    const inner = s.substring(1, s.length - 1);
    return (/^\+?\d+$/.test(inner) ? 'ﬂ°°' : 'ﬂ°') + inner + '¶ß';
  });
  return txt;
}

describe('entity encoding', () => {
  it('matches Mermaid on hand-picked lines', () => {
    for (const text of [
      'style A fill:#f9f,stroke:#333;',
      'style A fill:#f9f;stroke:#333;\nB --> C;',
      'classDef green fill:#9f6,stroke:#333,stroke-width:2px;',
      'A["a #35; b #quot;c#quot;"]',
      'style A color: #fff;',
      'style A fill:red;',
      'mystyle x:#1; classDef y:#2; z;',
      'style a:#b\n;',
      'style a b :c#d e;f;g',
      '#;# ; #a ;#a;',
    ]) {
      expect(encodeEntities(text), text).toBe(reference(text));
    }
  });

  it('matches Mermaid on random lines', () => {
    const rnd = random(11);
    const parts = ['style', 'classDef', ':', '#', ';', ' ', '\n', 'a', 'fill', '9', '\t', ':#', '#f9f;', '\r'];
    for (let i = 0; i < 20000; i++) {
      let text = '';
      const count = Math.floor(rnd() * 14);
      for (let k = 0; k < count; k++) text += parts[Math.floor(rnd() * parts.length)];
      expect(encodeEntities(text), JSON.stringify(text)).toBe(reference(text));
    }
  });

  it('stays fast where the regex does not', () => {
    const started = performance.now();
    encodeEntities('graph TD\nstyle A ' + 'style:x#'.repeat(20000));
    encodeEntities('classDef ' + ':#'.repeat(50000));
    expect(performance.now() - started).toBeLessThan(500);
  });
});

describe('preprocess', () => {
  it('removes comment lines but keeps directives', () => {
    expect(cleanupComments('%% one\ngraph TD\n  %% two\n  A --> B\n')).toBe('graph TD\n  A --> B\n');
    expect(cleanupComments('%%{init: {}}%%\ngraph TD')).toBe('%%{init: {}}%%\ngraph TD');
  });

  it('reads the title and config from front matter', () => {
    const result = preprocess('---\ntitle: My chart\nconfig:\n  flowchart:\n    curve: linear\n---\ngraph TD\n  A');
    expect(result.title).toBe('My chart');
    expect(result.config).toEqual({ flowchart: { curve: 'linear' } });
    expect(result.text).toBe('graph TD\n  A');
  });

  it('merges init directives over front matter and removes them', () => {
    const result = preprocess(
      "---\nconfig:\n  flowchart:\n    curve: linear\n    rankSpacing: 10\n---\n%%{init: {'flowchart': {'rankSpacing': 99}}}%%\ngraph TD\n  A"
    );
    expect(result.config).toEqual({ flowchart: { curve: 'linear', rankSpacing: 99 } });
    expect(result.text).toBe('graph TD\n  A');
  });

  it('does not let config reach object prototypes', () => {
    preprocess(
      '%%{init: {"__proto__": {"polluted": true}, "constructor": {"prototype": {"polluted": true}}}}%%\ngraph TD\nA'
    );
    preprocess('---\nconfig:\n  __proto__:\n    polluted: true\n---\ngraph TD\nA');
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it('normalizes line endings and attribute quotes', () => {
    expect(preprocess('graph TD\r\nA["<span class="x">hi</span>"]\r').text).toBe(
      'graph TD\nA["<span class=\'x\'>hi</span>"]\n'
    );
  });

  it('handles text that looks like unclosed tags in linear time', () => {
    const started = performance.now();
    preprocess('graph TD\nA["' + '<a'.repeat(50000) + '"]');
    expect(performance.now() - started).toBeLessThan(200);
  });
});
