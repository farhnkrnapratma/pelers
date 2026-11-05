export default {
  files: [
    ['user-journey', /^journeyDb\.spec\.js$/],
    ['user-journey/parser', /^journey\.spec\.js$/],
  ],
  grammars: ['user-journey/parser/journey.jison'],
  docs: 'userJourney.md',
  rewrite: [
    [/from '\.\/journey\.jison'/g, "from './adapter.js'"],
    [/from '(?:\.\.\/|\.\/)journeyDb\.js'/g, "from './adapter.js'"],
    [/from '\.\.\/\.\.\/tests\/util\.js'/g, "from './adapter.js'"],
  ],
};
