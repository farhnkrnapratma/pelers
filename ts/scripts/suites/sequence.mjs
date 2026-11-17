const BOUNDS =
  "Checks pixel positions kept by Mermaid's renderer (`renderer.bounds`). Pelers has its own layout and no layout parity. What the test parses is covered in tests/render/sequence.test.ts.";
const BOUNDS_API = "Exercises the bounds bookkeeping object of Mermaid's renderer directly; no diagram is parsed.";

const bounds = (test) => ({ file: 'sequenceDiagram.spec.js', test, reason: BOUNDS });
const boundsApi = (test) => ({ file: 'sequenceDiagram.spec.js', test, reason: BOUNDS_API });

export default {
  files: [['sequence', /^(?:sequenceDiagram\.spec\.js|keywordActors\.spec\.ts)$/]],
  grammars: ['sequence/parser/sequenceDiagram.jison'],
  docs: 'sequenceDiagram.md',
  skip: [
    boundsApi('should handle a simple bound call'),
    boundsApi('should handle an expanding bound'),
    boundsApi('should handle inserts within the bound without changing the outer bounds'),
    boundsApi('should handle a loop without expanding the area'),
    boundsApi('should handle multiple loops without expanding the bounds'),
    boundsApi('should handle a loop that expands the area'),
    bounds('it should handle one actor, when textPlacement is '),
    bounds('should handle one actor and a centered note'),
    bounds('should handle one actor and a note to the left'),
    bounds('should handle one actor and a note to the right'),
    bounds('should handle two actors'),
    bounds('should handle two actors in a box'),
    bounds('should handle two actors with init directive'),
    bounds('should handle two actors with init directive with multiline directive'),
    bounds('should handle two actors and two centered shared notes'),
    bounds('should draw two actors and two messages'),
    bounds('should draw two actors notes to the right'),
    bounds('should draw two actors notes to the left'),
    bounds('should draw two actors notes to the left with text wrapped (inline)'),
    bounds('should draw two actors notes to the left with text wrapped (directive)'),
    bounds('should draw two actors notes to the left with text wrapped and the init directive sets the theme to dark'),
    bounds(
      'should draw two actors, notes to the left with text wrapped and the init directive sets the theme to dark and fontFamily to Menlo, fontSize to 18, and fontWeight to 800'
    ),
    bounds('should draw two loops'),
    bounds('should draw background rect'),
    bounds('should handle one actor, when textPlacement is'),
    {
      file: 'sequenceDiagram.spec.js',
      test: 'should have functions used in sequence JISON as own property',
      reason:
        'Checks that SequenceDB methods are own properties, a requirement of the Jison runtime. Pelers has no Jison runtime.',
    },
    {
      file: 'keywordActors.spec.ts',
      test: 'parses a details statement',
      reason:
        'Reads the properties from a page element by id. Pelers has no page to read; it parses the statement and keeps the id.',
    },
  ],
  notPorted: [
    ['actorBandModel.spec.ts', 'Renders through Mermaid in a DOM and measures its actor shapes.'],
    ['actorSizing.spec.ts', "Tests Mermaid's stick figure drawing code against a DOM."],
    ['lifelineStart.spec.ts', "Tests where Mermaid's actor shapes start their lifelines, in pixels."],
    ['noteFontWeight.spec.ts', "Tests the CSS Mermaid's themes generate."],
    ['palettePicking.spec.ts', "Tests Mermaid's theme palette lookup."],
    ['rectSectionFill.spec.ts', "Tests the colors of Mermaid's themes."],
    ['sequenceRenderer.spec.js', "Tests Mermaid's drawMessage against a mocked d3."],
    ['svgDraw.spec.js', "Tests Mermaid's d3 drawing helpers."],
  ],
  rewrite: [
    [/from '\.\/sequenceDb\.js'/g, "from './adapter.js'"],
    [/import parser from '\.\/parser\/sequenceDiagram\.jison'/g, "import { parser } from './adapter.js'"],
    [/import mermaidAPI from '\.\.\/\.\.\/mermaidAPI\.js'/g, "import { mermaidAPI } from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagram-orchestration\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/Diagram\.js'/g, "from './adapter.js'"],
    // jsdomIt gives a test a DOM. The one test that uses it is skipped, so plain `it` will do.
    [/import \{ jsdomIt \} from '\.\.\/\.\.\/tests\/util\.js';\n/g, ''],
    [/\bjsdomIt\(/g, 'it('],
    // A test name written as a template literal, made into one the skip list can find.
    [
      /it\(`\nit should handle one actor, when textPlacement is \$\{textPlacement\}`/g,
      "it('it should handle one actor, when textPlacement is ' + textPlacement",
    ],
  ],
};
