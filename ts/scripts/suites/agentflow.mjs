const FIXTURES = [
  'complete-example',
  'error-shape-removed',
  'parse-error-unterminated-container',
  'pattern-connector-dotted-form',
  'pattern-parallel',
  'pattern-reference-doc',
  'smoke-valid',
  'warning-shape-unsupported',
];

const CSS =
  'Tests the CSS Mermaid generates for its color themes. Pele writes no stylesheet; colors are presentation attributes.';
const YAML =
  "Asserts js-yaml's own message text, mark and excerpt for a YAML error. Pele has its own YAML reader, so the error gives the position of the metadata block instead of the failing character.";

const LENIENT =
  "Needs js-yaml to reject the block. Pele's YAML reader has no anchors or aliases and reads `*name` and a lone `:` as text, so it accepts the block. The positioned error itself is covered in tests/render/agentflow.test.ts.";

export default {
  files: [
    ['agentflow', /^(?!agentflow-max-text-size|agentflow-render-positions|renderer)[\w-]+\.spec\.ts$/],
    ['agentflow/parser', /\.spec\.ts$/],
    ['agentflow/conformance', /^(?:conformance\.spec|runner\.spec|runner)\.ts$/],
  ],
  // The conformance fixtures are vendored the way the grammar is: copied as they are, next to the specs.
  grammars: [
    'agentflow/parser/agentflow.jison',
    ...FIXTURES.flatMap((name) => [
      `agentflow/conformance/fixtures/${name}-agentflow.mmd`,
      `agentflow/conformance/fixtures/${name}-agentflow.expected.json`,
    ]),
  ],
  docs: 'agentflow.md',
  skip: [
    { file: 'colorSlots.spec.ts', test: 'emits a rule for every kind, at that kind slot colour', reason: CSS },
    { file: 'colorSlots.spec.ts', test: 'paints containers from the slots above the kind range', reason: CSS },
    {
      file: 'colorSlots.spec.ts',
      test: 'names both forms of a container, so a collapsed one is not left grey',
      reason: CSS,
    },
    { file: 'colorSlots.spec.ts', test: 'uses no container slot below the kind range', reason: CSS },
    {
      file: 'colorSlots.spec.ts',
      test: 'strokes without filling when the theme carries no background palette',
      reason: CSS,
    },
    { file: 'colorSlots.spec.ts', test: 'emits nothing for a theme that carries no palette', reason: CSS },
    { file: 'colorSlots.spec.ts', test: 'rejects a look that would break out of the selector', reason: CSS },
    {
      file: 'agentflow-yaml-error-position.spec.ts',
      test: 'single-line block: maps (block 2:16) to the source line of the @{ } block',
      reason: YAML,
    },
    {
      file: 'agentflow-yaml-error-position.spec.ts',
      test: 'multi-line block: maps the body line into source space and keeps the tab caret',
      reason: YAML,
    },
    {
      file: 'agentflow-yaml-error-position.spec.ts',
      test: 'multi-line block: error on the @{ line itself maps with the content-column offset',
      reason: YAML,
    },
    {
      file: 'agentflow-yaml-error-position.spec.ts',
      test: 'applies the frontmatter line offset on top of the block position',
      reason: YAML,
    },
    {
      file: 'agentflow-yaml-error-position.spec.ts',
      test: 'composes with the blank-line fold fix: a block below a @{ }+blank keeps its source line',
      reason: LENIENT,
    },
    {
      file: 'agentflow-metadata-trailing-comma.spec.ts',
      test: 'invalid YAML that commas cannot fix still throws',
      reason: LENIENT,
    },
    {
      file: 'agentflow-empty-metadata.spec.ts',
      test: 'a genuinely malformed block still throws a positioned YAML error, not a TypeError',
      reason: LENIENT,
    },
  ],
  notPorted: [
    [
      'agentflow-max-text-size.spec.ts',
      "Tests Mermaid's maxTextSize plumbing through mermaidAPI under jsdom. Pele checks its limit on the whole source before preprocessing, so the comment-preserving text is never longer than the limit.",
    ],
    [
      'agentflow-render-positions.spec.ts',
      "Drives mermaidAPI.render and mermaidAPI.parse under jsdom and lists Mermaid's detector registry. What it protects, source positions that count front matter and comment lines on the public API, is covered in tests/render/agentflow.test.ts.",
    ],
    [
      'renderer.spec.ts',
      "Mocks Mermaid's unified renderer to check that draw() hands the SVG element id to the DB. Pele writes no element ids.",
    ],
  ],
  rewrite: [
    [/from '(?:\.\.\/|\.\/)agentflowDb\.js'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/parser\/|\.\/parser\/|\.\/)agentflowParser\.js'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/)+config\.js'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/)+logger\.js'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/|\.\/)diagnostics\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/types\.js'/g, "from './adapter.js'"],
    [/from '\.\/transformData\.js'/g, "from './adapter.js'"],
    [/from '\.\/colorSlots\.js'/g, "from './adapter.js'"],
    [/import getStyles from '\.\/styles\.js'/g, "import { getStyles } from './adapter.js'"],
    [/from '\.\.\/\.\.\/rendering-util\/types\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagramAPI\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/diagram-api\/diagram-orchestration\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/Diagram\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/flowchart\/flowDb\.js'/g, "from '../flowchart/adapter.js'"],
    [/from '\.\.\/\.\.\/flowchart\/parser\/flowParser\.ts'/g, "from '../flowchart/adapter.js'"],
    [/join\(thisDir, 'fixtures'\)/g, "join(thisDir, 'upstream')"],
  ],
};
