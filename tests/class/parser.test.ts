import { describe, expect, it } from 'vitest';
import { ClassDb } from '../../src/diagrams/class/db.js';
import { buildClassGraph } from '../../src/diagrams/class/graph.js';
import { parseClassDiagram } from '../../src/diagrams/class/parser.js';
import { PeleError } from '../../src/errors.js';
import { parse as parseAny } from '../../src/index.js';

function parse(src: string): ClassDb {
  const db = new ClassDb();
  parseClassDiagram(src, db);
  return db;
}

describe('class diagram parser', () => {
  // The assertions of Mermaid's "should parse diagram with direction", without its DOM ids.
  it('parses a diagram with a direction, members and cardinalities', () => {
    const db = parse(`classDiagram
          direction TB
          class Student {
            -idCard : IdCard
          }
          class IdCard{
            -id : int
            -name : string
          }
          class Bike{
            -id : int
            -name : string
          }
          Student "1" --o "1" IdCard : carries
          Student "1" --o "1" Bike : rides`);
    expect(db.direction).toBe('TB');
    const student = db.classes.get('Student')!;
    expect(student).toMatchObject({
      id: 'Student',
      label: 'Student',
      type: '',
      methods: [],
      annotations: [],
      cssClasses: 'default',
      styles: [],
    });
    expect(student.members).toHaveLength(1);
    expect(student.members[0]).toMatchObject({
      id: 'idCard : IdCard',
      visibility: '-',
      classifier: '',
      memberType: 'attribute',
    });
    expect(db.relations).toEqual([
      {
        id1: 'Student',
        id2: 'IdCard',
        relation: { lineType: 0, type1: 'none', type2: 0 },
        relationTitle1: '1',
        relationTitle2: '1',
        title: 'carries',
      },
      {
        id1: 'Student',
        id2: 'Bike',
        relation: { lineType: 0, type1: 'none', type2: 0 },
        relationTitle1: '1',
        relationTitle2: '1',
        title: 'rides',
      },
    ]);
  });

  it('is reached through the public parse function, with the front matter title', () => {
    const model = parseAny('---\ntitle: Animals\n---\nclassDiagram-v2\n  Animal <|-- Duck');
    expect(model.type).toBe('class');
    if (model.type !== 'class') return;
    expect(model.title).toBe('Animals');
    expect([...model.classes.keys()]).toEqual(['Animal', 'Duck']);
  });

  // Mermaid 12.1.0 throws a TypeError here: the class is only created by a later statement.
  it('accepts an annotation for a class that is not declared yet', () => {
    const db = parse('classDiagram\n<<interface>> Shape\nclass Shape {\n  draw()\n}');
    expect(db.classes.get('Shape')!.annotations).toEqual(['interface']);
    expect(db.classes.get('Shape')!.methods).toHaveLength(1);
  });

  it('reads an annotation written as a member', () => {
    const db = parse('classDiagram\nclass Shape {\n  <<interface>>\n  draw()\n}\nColor : <<enumeration>>');
    expect(db.classes.get('Shape')!.annotations).toEqual(['interface']);
    expect(db.classes.get('Color')!.annotations).toEqual(['enumeration']);
  });

  it('applies a classDef to whole class names only', () => {
    // In Mermaid a classDef named `def` styles every class, because `default` contains it.
    const db = parse('classDiagram\nclass A:::defined\nclass B\nclassDef def fill:#f00\nclassDef defined fill:#0f0');
    expect(db.classes.get('A')!.styles).toEqual(['fill:#0f0']);
    expect(db.classes.get('B')!.styles).toEqual([]);
  });

  it('applies the default classDef to every class declared before it', () => {
    const db = parse('classDiagram\nclass A\nclassDef default fill:#f00,stroke:#000\nclass B');
    expect(db.classes.get('A')!.styles).toEqual(['fill:#f00', 'stroke:#000']);
    expect(db.classes.get('B')!.styles).toEqual([]);
    expect(db.styleClasses.get('default')!.styles).toEqual(['fill:#f00,stroke:#000']);
  });

  it('ignores cssClass and link for an id that starts with a digit, as Mermaid does', () => {
    const db = parse('classDiagram\nclass 1A\nclass B\ncssClass "1A,B" hot\nlink 1A "https://example.com"');
    expect(db.classes.get('1A')!.cssClasses).toBe('default');
    expect(db.classes.get('1A')!.link).toBeUndefined();
    expect(db.classes.get('B')!.cssClasses).toBe('default hot');
  });

  it('turns the lollipop end of a relation into an interface', () => {
    const db = parse('classDiagram\nbar ()-- foo\nfoo --() baz\nA ()--() B');
    expect(db.interfaces).toEqual([
      { id: 'interface0', label: 'bar', classId: 'foo' },
      { id: 'interface1', label: 'baz', classId: 'foo' },
    ]);
    expect([...db.classes.keys()]).toEqual(['foo', 'A', 'B']);
    expect(db.relations.map((r) => [r.id1, r.id2])).toEqual([
      ['interface0', 'foo'],
      ['foo', 'interface1'],
      ['A', 'B'],
    ]);
  });

  it('keeps interfaces, notes and classes apart when their ids collide', () => {
    const db = parse(
      'classDiagram\nclass interface0\nclass note0\nbar ()-- interface0\nnote for note0 "n"\nnamespace note0 {\n class X\n}'
    );
    const { nodes, edges } = buildClassGraph(db);
    const shapeAt = (index: number): string => nodes[index].shape;
    expect(edges.map((e) => [shapeAt(e.source), shapeAt(e.target)])).toEqual([
      ['note', 'classBox'],
      ['interface', 'classBox'],
    ]);
    expect(nodes[nodes.findIndex((n) => n.id === 'X')].parent).toBe(nodes.findIndex((n) => n.isGroup));
  });

  it('marks a class with a callback as clickable and keeps nothing to run', () => {
    const db = parse('classDiagram\nclass A\nclick A call alert("x") "tip"\ncallback A "alert"');
    const node = db.classes.get('A')!;
    expect(node.haveCallback).toBe(true);
    expect(node.cssClasses).toBe('default clickable clickable');
    expect(node.tooltip).toBe('tip');
    expect(JSON.stringify(node)).not.toContain('alert');
  });

  it('sanitizes link targets', () => {
    const db = parse(
      'classDiagram\nclass A\nclass B\nlink A "javascript:alert(1)"\nclick B href "https://example.com/a b" "tip" _self'
    );
    expect(db.classes.get('A')!.link).toBe('about:blank');
    expect(db.classes.get('A')!.linkTarget).toBe('_blank');
    expect(db.classes.get('B')!.link).toBe('https://example.com/a%20b');
    expect(db.classes.get('B')!.linkTarget).toBe('_self');
  });

  it('nests namespaces by block and by dotted name', () => {
    const db = parse(
      'classDiagram\nnamespace A.B {\n class X\n}\nnamespace A {\n namespace C["See"] {\n  class Y\n  note "n"\n }\n}'
    );
    expect([...db.namespaces.keys()]).toEqual(['A', 'A.B', 'A.C']);
    expect(db.namespaces.get('A.C')).toMatchObject({ label: 'See', parent: 'A', explicit: true });
    expect([...db.namespaces.get('A')!.children.keys()]).toEqual(['A.B', 'A.C']);
    expect(db.classes.get('Y')!.parent).toBe('A.C');
    expect(db.notes.get('note0')!.parent).toBe('A.C');
  });

  it('accepts statements with no header line when the input ends inside a token, as the grammar does', () => {
    expect([...parse('B <|-- A~').classes.keys()]).toEqual(['B', 'A']);
    expect(() => parse('class A')).toThrow(PeleError);
  });

  it('reports syntax errors in the form Mermaid uses', () => {
    expect(() => parse('classDiagram\nnote link')).toThrow(
      /Parse error on line 2:\nnote link\n-----\^\nExpecting 'STR', got 'LINK'/
    );
    expect(() => parse('classDiagram\nclass A { x')).toThrow(/got 'EOF_IN_STRUCT'/);
    expect(() => parse('classDiagram\nclass A\n@')).toThrow(/Lexical error on line 3\. Unrecognized text\./);
    expect(() => parse('classDiagram\n')).toThrow(PeleError);
    expect(() => parse('classDiagram\nnamespace N {\n}')).toThrow(
      /Expecting 'NAMESPACE', 'CLASS', 'NOTE_FOR', 'NOTE', got 'STRUCT_STOP'/
    );
  });
});
