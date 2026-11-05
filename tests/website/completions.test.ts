import { describe, expect, it } from 'vitest';
import { detectType, registered, render } from '../../src/index.js';
import { all } from '../../src/diagrams/registry.js';
import { isValidShape } from '../../src/diagrams/flowchart/shapes.js';
import { SHAPE_ALIASES } from '../../src/diagrams/agentflow/shapes.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import {
  agentflowShapes,
  declarations,
  diagramNames,
  flowchartShapes,
  keywords,
  mermaidCompletions,
} from '../../website/src/lib/playground-completions.js';

// `¦` marks the cursor.
function at(marked: string, explicit = false): string[] | null {
  const position = marked.indexOf('¦');
  const source = marked.slice(0, position) + marked.slice(position + 1);
  const found = mermaidCompletions(source, position, detectType(source), explicit);
  return found && found.options.map((option) => option.label);
}

function applied(marked: string, label: string): string {
  const position = marked.indexOf('¦');
  const source = marked.slice(0, position) + marked.slice(position + 1);
  const found = mermaidCompletions(source, position, detectType(source), true)!;
  const option = found.options.find((candidate) => candidate.label === label)!;
  return source.slice(0, found.from) + (option.apply ?? option.label) + source.slice(found.to);
}

describe('playground completions', () => {
  it('offers a declaration that Pele draws for every diagram type', () => {
    const offered = new Set<string>();
    for (const [label] of declarations) {
      const type = detectType(label);
      expect(type, label).not.toBeNull();
      expect(registered(type!), label).toBe(true);
      offered.add(type!);
    }
    expect([...offered].sort()).toEqual(all.map((diagram) => diagram.type).sort());
  });

  it('defines keywords only for registered diagram types', () => {
    const types = new Set<string>(all.map((diagram) => diagram.type));
    for (const type of Object.keys(keywords)) {
      expect(types.has(type), type).toBe(true);
    }
  });

  it('only offers shapes that exist', () => {
    for (const shape of flowchartShapes) expect(isValidShape(shape), shape).toBe(true);
    for (const shape of agentflowShapes) expect(SHAPE_ALIASES.has(shape), shape).toBe(true);
  });

  it('offers diagram types on the first line and directions after a flowchart', () => {
    expect(at('¦', true)).toContain('sequenceDiagram');
    expect(at('¦')).toBeNull();
    expect(at('seq¦')).toContain('sequenceDiagram');
    expect(at('---\ntitle: x\n---\n%% note\nflow¦')).toContain('flowchart');
    expect(at('flowchart ¦')).toEqual(['TD', 'BT', 'LR', 'RL']);
    expect(at('graph L¦')).toContain('LR');
    expect(at('sequenceDiagram ¦')).toBeNull();
    expect(applied('seq¦\n  A->>B: hi', 'sequenceDiagram')).toBe('sequenceDiagram\n  A->>B: hi');
  });

  it('offers the statements of the diagram type at the start of a line', () => {
    expect(at('flowchart LR\n  sub¦')).toContain('subgraph');
    expect(at('flowchart LR\n  sub¦')).not.toContain('participant');
    expect(at('sequenceDiagram\n  par¦')).toEqual(expect.arrayContaining(['participant', 'par']));
    expect(at('gitGraph\n  c¦')).toEqual(expect.arrayContaining(['commit', 'checkout', 'cherry-pick']));
    expect(at('flowchart LR\n  ¦')).toBeNull();
    expect(at('flowchart LR\n  ¦', true)).toContain('subgraph');
    expect(applied('C4Context\n  Per¦', 'Person')).toBe('C4Context\n  Person(');
    expect(at('mindmap\n  ro¦')).toBeNull();
  });

  it('offers the names a diagram already uses', () => {
    const flow =
      'flowchart LR\n  Start[Begin here] --> Check{Ready?}\n  Check -->|yes| Done\n  Check --o Other\n  classDef big fill:#f9f\n';
    expect(diagramNames(flow, 'flowchart').sort()).toEqual(['Check', 'Done', 'Other', 'Start']);
    expect(at(flow + '  Done --> S¦')).toEqual(expect.arrayContaining(['Start']));
    expect(at(flow + '  Done --> S¦')).not.toContain('S');
    expect(at(flow + '  Ch¦')).toEqual(expect.arrayContaining(['Check', 'subgraph']));
    expect(at(flow + '  Done --> ¦')).toBeNull();
    expect(at(flow + '  Done --> ¦', true)).toContain('Start');

    const sequence =
      'sequenceDiagram\n  participant A as Alice Smith\n  actor Bob\n  A->>Bob: Hello there\n  Bob--xA: Bye\n  Carol-)A: Later\n';
    expect(diagramNames(sequence, 'sequence').sort()).toEqual(['A', 'Bob', 'Carol']);
    expect(at(sequence + '  Bob->>C¦')).toEqual(['A', 'Bob', 'Carol']);

    const er =
      'erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER {\n    string id\n    int total\n  }\n  LINE-ITEM }|..|{ ORDER : in\n';
    expect(diagramNames(er, 'er').sort()).toEqual(['CUSTOMER', 'LINE-ITEM', 'ORDER']);
    expect(at(er + '  CUSTOMER ||--o{ L¦')).toContain('LINE-ITEM');

    const classes =
      'classDiagram\n  class Animal {\n    +String name\n    +run()\n  }\n  Animal <|-- Dog : is a\n  Cat o-- Tail\n';
    expect(diagramNames(classes, 'class').sort()).toEqual(['Animal', 'Cat', 'Dog', 'Tail']);

    const state =
      'stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running: start\n  state Running {\n    Fast --> Slow\n  }\n';
    expect(diagramNames(state, 'state').sort()).toEqual(['Fast', 'Idle', 'Running', 'Slow']);
  });

  it('offers values where only a few are possible', () => {
    expect(at('flowchart LR\n  A@{ shape: ¦')).toEqual(flowchartShapes);
    expect(at('flowchart LR\n  A@{ label: "x", shape: cy¦')).toContain('cyl');
    expect(at('agentflow-beta TB\n  a["A"]@{ shape: ¦')).toEqual(agentflowShapes);
    expect(at('flowchart LR\n  subgraph one\n    direction ¦')).toEqual(['TB', 'BT', 'LR', 'RL']);
    expect(at('flowchart LR\n  classDef big fill:#f9f\n  classDef small,tiny fill:#eee\n  A:::¦')).toEqual([
      'big',
      'small',
      'tiny',
    ]);
    expect(at('flowchart LR\n  classDef big fill:#f9f\n  A --> B\n  class A,B ¦')).toEqual(['big']);
    expect(at('flowchart LR\n  A --> B\n  style ¦')).toEqual(['A', 'B']);
    expect(at('flowchart LR\n  A --> B\n  click A ¦')).toEqual(['href', 'call']);
    expect(at('flowchart LR\n  classDef big ¦')).toContain('stroke-width:');
    expect(at('flowchart LR\n  classDef big fill:#f9f,str¦')).toContain('stroke:');
    expect(at('flowchart LR\n  classDef big fill:#f¦')).toBeNull();
    expect(at('sequenceDiagram\n  A->>B: hi\n  Note ¦')).toEqual(['right of', 'left of', 'over']);
    expect(at('sequenceDiagram\n  A->>B: hi\n  Note right o¦')).toContain('right of');
    expect(applied('sequenceDiagram\n  A->>B: hi\n  Note right o¦', 'right of')).toBe(
      'sequenceDiagram\n  A->>B: hi\n  Note right of'
    );
    expect(at('sequenceDiagram\n  A->>B: hi\n  Note over ¦')).toEqual(['A', 'B']);
    expect(at('sequenceDiagram\n  A->>B: hi\n  activate ¦')).toEqual(['A', 'B']);
    expect(at('gitGraph\n  commit\n  branch develop\n  branch "release/1"\n  checkout ¦')).toEqual([
      'main',
      'develop',
      'release/1',
    ]);
    expect(at('gitGraph\n  commit ¦')).toEqual(['id:', 'tag:', 'type:', 'order:', 'parent:']);
    expect(at('gitGraph\n  commit id: "a b" t¦')).toContain('tag:');
    expect(at('gitGraph\n  commit id: "a t¦')).toBeNull();
    expect(at('gitGraph\n  commit type: ¦')).toEqual(['NORMAL', 'REVERSE', 'HIGHLIGHT']);
    expect(applied('gitGraph\n  commit t¦', 'tag:')).toBe('gitGraph\n  commit tag: ');
    expect(at('requirementDiagram\n  requirement r {\n    risk: ¦')).toEqual(['low', 'medium', 'high']);
    expect(at('requirementDiagram\n  requirement r {\n    v¦')).toContain('verifymethod:');
    expect(at('requirementDiagram\n  requirement r {\n  }\n  element e {\n  }\n  e - ¦')).toContain('satisfies');
    expect(at('gantt\n  section A\n  Task one :¦')).toBeNull();
    expect(at('gantt\n  section A\n  Task one :c¦')).toContain('crit');
    expect(at('gantt\n  section A\n  Task one :crit, a¦')).toContain('active');
    expect(at('gantt\n  section A\n  Task one :a1, 2024-01-01, 3¦')).toBeNull();
    expect(at('architecture-beta\n  group api(cloud)[API]\n  service db(¦')).toContain('database');
    expect(at('architecture-beta\n  group api(cloud)[API]\n  service db(database)[DB] in ¦')).toEqual(['api']);
  });

  it('stays out of the way inside text', () => {
    expect(at('---\nti¦\n---\nflowchart LR\n  A --> B')).toBeNull();
    expect(at('flowchart LR\n  %% sub¦')).toBeNull();
    expect(at('flowchart LR\n  A --> B\n  B["A¦')).toBeNull();
    expect(at('flowchart LR\n  A --> B\n  B[A¦')).toBeNull();
    expect(at('flowchart LR\n  A --> B\n  B -->|A¦')).toBeNull();
    expect(at('flowchart LR\n  A --> B\n  B(A¦')).toBeNull();
    expect(at('sequenceDiagram\n  A->>B: hi\n  A->>B: A¦')).toBeNull();
    expect(at('stateDiagram-v2\n  Idle --> Running\n  Idle : I¦')).toBeNull();
    expect(at('classDiagram\n  class Animal {\n    A¦')).toBeNull();
    expect(at('erDiagram\n  ORDER {\n    string O¦')).toBeNull();
    expect(at('flowchart LR\n  A --> B\n  click A href "A¦')).toBeNull();
    expect(at('pie\n  "Dogs" : 3\n  D¦')).toEqual(['title']);
  });

  it('offers nothing that does not parse', () => {
    const lines: [string, string][] = [
      [
        'flowchart LR\n  A --> B\n  classDef big fill:#eee\n',
        '  subgraph one\n  end\n  click A href "https://example.com"\n  class A big\n  style B stroke-width:2px\n  linkStyle 0 stroke-dasharray:4\n',
      ],
      [
        'sequenceDiagram\n  participant A\n  actor B\n',
        '  A->>B: hi\n  Note right of A: x\n  Note over A,B: y\n  activate B\n  deactivate B\n  loop again\n  A->>B: hi\n  end\n  critical c\n  A->>B: hi\n  option o\n  A->>B: hi\n  end\n',
      ],
      [
        'gitGraph\n  commit id: "a" tag: "v1" type: HIGHLIGHT\n',
        '  branch develop order: 2\n  commit\n  checkout main\n  merge develop tag: "v2"\n  switch develop\n',
      ],
      ['stateDiagram-v2\n  Idle --> Running\n', '  note right of Idle\n    text\n  end note\n  direction LR\n'],
      [
        'classDiagram\n  class Animal\n  classDef big fill:#eee\n',
        '  note for Animal "text"\n  cssClass "Animal" big\n  namespace Zoo {\n    class Cat\n  }\n',
      ],
      [
        'gantt\n  dateFormat YYYY-MM-DD\n  section A\n',
        '  One :done, crit, a1, 2024-01-01, 3d\n  Two :active, milestone, after a1, 1d\n  excludes weekends\n  weekday monday\n  todayMarker off\n  inclusiveEndDates\n  topAxis\n',
      ],
      [
        'radar-beta\n  axis a, b, c\n  curve x{1, 2, 3}\n',
        '  graticule polygon\n  showLegend false\n  ticks 4\n  max 5\n  min 0\n',
      ],
      ['architecture-beta\n  group api(cloud)[API]\n', '  service db(database)[DB] in api\n  junction j in api\n'],
      [
        'requirementDiagram\n  requirement r {\n    id: 1\n    text: t\n    risk: high\n    verifymethod: test\n  }\n  element e {\n    type: sim\n    docref: ref\n  }\n',
        '  e - satisfies -> r\n  e - traces -> r\n',
      ],
    ];
    for (const [head, tail] of lines)
      expect(() => render(head + tail, { measurer: metricsMeasurer }), head).not.toThrow();
  });
});
