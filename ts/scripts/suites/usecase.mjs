const SINGLETON =
  "Tests Mermaid's shared DB singleton (commit, clear, counters, config). Pelers builds a new model on every parse and has no DB to commit into.";

export default {
  files: [['usecase', /^(?:usecase|usecase\.parser|usecaseDb)\.spec\.ts$/]],
  docs: 'usecase.md',
  skip: [
    { file: 'usecaseDb.spec.ts', test: 'atomically replaces committed state with a detached model', reason: SINGLETON },
    {
      file: 'usecaseDb.spec.ts',
      test: 'keeps the previous commit when a replacement is incomplete',
      reason: SINGLETON,
    },
    {
      file: 'usecaseDb.spec.ts',
      test: 'clears every collection, counter, AST, direction, config, title, and accessibility field',
      reason: SINGLETON,
    },
    {
      file: 'usecase.spec.ts',
      test: 'hands schema-driven layout, viewport, and font configuration to the renderer contract',
      reason: "Tests Mermaid's global config being copied into the data handed to its renderer.",
    },
    {
      file: 'usecase.spec.ts',
      test: 'consolidates canonical selectors and relies on shared animation keyframes',
      reason: 'Tests the CSS Mermaid generates for its themes. Pelers emits no stylesheet.',
    },
  ],
  notPorted: [
    [
      'usecaseRenderer.spec.ts',
      "Tests Mermaid's renderer: DOM ids, HTML escaping of labels for its HTML label renderer, aria attributes set on d3 selections, and font custom properties.",
    ],
    [
      'usecase-colorIndex.spec.ts',
      "Tests the palette slot numbers Mermaid's redux-color themes read. Pelers colors by role through CSS tokens and has no palette slots.",
    ],
    ['usecaseRoleColors.spec.ts', 'Tests the theme CSS Mermaid generates. Pelers emits no stylesheet.'],
    [
      'usecase.docs.spec.ts',
      "Renders the documentation examples through mermaidAPI under jsdom. The same examples are Pelers's doc corpus and are parsed and rendered by its own tests.",
    ],
  ],
  rewrite: [
    [/from '\.\/(?:usecaseDb|usecaseTypes|styles)\.js'/g, "from './adapter.js'"],
    [/from '\.\/parser\/usecase\.chevrotain\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/common\/(?:commonDb|common)\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/(?:Diagram|config|diagram-api\/diagram-orchestration)\.js'/g, "from './adapter.js'"],
  ],
};
