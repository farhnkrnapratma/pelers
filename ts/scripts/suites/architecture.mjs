export default {
  files: [
    ['architecture', /^architecture\.spec\.ts$/],
    ['/packages/parser/tests', /^architecture\.test\.ts$/],
  ],
  docs: 'architecture.md',
  skip: [
    {
      file: 'architecture.spec.ts',
      test: 'should not produce console warnings for edges without labels',
      reason: 'Tests a cytoscape stylesheet. Pelers does not use cytoscape.',
    },
    {
      file: 'architecture.spec.ts',
      test: 'should round-trip user-supplied fcose knobs',
      reason: "Tests Mermaid's config plumbing for options of the fcose layout engine, which Pelers does not use.",
    },
    {
      file: 'architecture.spec.ts',
      test: 'should leave defaults intact when only one knob is set',
      reason: "Tests Mermaid's config plumbing for options of the fcose layout engine, which Pelers does not use.",
    },
    {
      file: 'architecture.spec.ts',
      test: 'should block proto pollution via service id',
      reason:
        "Reads `getDataStructures().groupAlignments`, an input to Mermaid's fcose layout that Pelers's layout does not build. Group ids like `__proto__` are covered in tests/render/architecture-security.test.ts.",
    },
  ],
  notPorted: [
    ['svgDraw.spec.ts', "Tests Mermaid's DOM renderer and cytoscape layout."],
    [
      'architectureSeed.spec.ts',
      'Tests the seeding of Math.random for the fcose layout engine. Pelers has no randomness.',
    ],
  ],
  rewrite: [
    [/import cytoscape from 'cytoscape';/g, "import { cytoscape } from './adapter.js';"],
    [/from '\.\/architectureParser\.js'/g, "from './adapter.js'"],
    [/from '\.\/architectureDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/config\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/src\/language\/index\.js'/g, "from './adapter.js'"],
    [/from '\.\/test-util\.js'/g, "from './adapter.js'"],
  ],
};
