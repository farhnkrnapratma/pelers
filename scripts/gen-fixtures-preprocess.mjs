import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const { parseYaml } = await import(`${root}ts/src/util/yaml.ts`);
const { cleanupComments, encodeEntities, preprocess } = await import(`${root}ts/src/preprocess.ts`);
const { inputOffset } = await import(`${root}ts/src/errors.ts`);

function mulberry32(seed) {
    return () => {
        seed = (seed + 0x6d2b79f5) | 0;
        let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}
const random = mulberry32(777);
const pick = list => list[Math.floor(random() * list.length)];

function encode(value) {
    if (value === null) return 'null';
    if (value === undefined) return 'undefined';
    if (typeof value === 'boolean') return String(value);
    if (typeof value === 'number') return Object.is(value, -0) ? 'n:-0' : `n:${value}`;
    if (typeof value === 'string') return JSON.stringify(value);
    if (Array.isArray(value)) return `[${value.map(encode).join(',')}]`;
    return `{${Object.keys(value).map(key => `${JSON.stringify(key)}:${encode(value[key])}`).join(',')}}`;
}

function attempt(run) {
    try {
        return run();
    } catch (error) {
        return `!${error.message}`;
    }
}

const YAML_CASES = [
    '{\n shape: rounded \n}',
    '{\nshape: rounded , label: "DD"\n}',
    '{\n label: "This is }", other: "clock"\n}',
    '{\n animate: true, animation: fast, w: 100, h: 50.5, x: null, y: ~\n}',
    '{\n label: \'single quoted\', icon: "fa:fa-user", form: circle, pos: t \n}',
    '{\n a: [1, 2, three], b: {c: d, e: [f]}\n}',
    '{\n}',
    '\n        shape: circle\n        other: "clock"\n     \n',
    '\n        label: |\n          This is a\n          multiline string\n        other: "clock"\n     \n',
    '\n        label: >\n          folded\n          text\n\n          second paragraph\n        other: 1\n',
    '\n        label: "This is a<br/>multiline string"\n        other: "clock"\n',
    'title: Hello world\nconfig:\n  theme: dark\n  flowchart:\n    curve: linear\n    htmlLabels: false\n',
    'title: "Quoted: title"\ndisplayMode: compact\n',
    'config:\n  themeVariables:\n    primaryColor: "#ff0000"\n    fontSize: 16px\n  look: handDrawn # trailing comment\n',
    'list:\n  - a\n  - b: 1\n    c: 2\n  - [x, y]\nempty:\n',
    'a: 0x1f\nb: -12\nc: 1e3\nd: .5\ne: "12"\nf: True\ng: FALSE\nh: Null\n',
    'key with spaces: plain value\nurl: http://example.com/a#b\n',
    'multi: this is\n  continued on the next line\nnext: 1\n',
    '# only a comment\n',
    '',
    'just a scalar',
    'a\nb\n  c',
    '- 1\n- 2\n',
    'a: {\n  b: 1,\n  c: [2,\n    3]\n}\nd: "two\n  lines"\ne: 5\n',
    '{\n a: 1, # comment\n b: 2\n}',
    '__proto__: 1\nb:\n  __proto__: {x: 1}\n',
    '{ a: 1',
    'a:\n\tb: 1',
    '{ a: ' + '['.repeat(100) + ' }',
    'a: "\\xZZ"',
    'a: "\\u00e9\\x41\\U0001F600\\n\\t\\q"',
    'a: "\\uD800"',
    '1: a\n2: b\n01: c\n',
    '? complex\n: value',
    'a: &anchor 1\nb: *anchor',
    'a: !!str 1',
    '- - a\n  - b\n- c',
    'a: |+\n  keep\n\n\nb: >-\n  fold\n  me\n',
    'a: |2\n    indented\nb: 1',
];
const YAML_FRAGMENTS = [
    'a: 1\n', 'b:\n  - x\n  - y: 2\n', '{a: 1, b: [2,3]}', '"q"', "'s'", '|\n  text\n', '>-\n  a\n  b\n', '# c\n', '- ', ': ',
    ',', '[', ']', '{', '}', '  ', '\t', '~', 'null', 'true', '0x1f', '.inf', '-.inf', '.nan', '1e3', '"\\u00e9"', '"\\x41"',
    '"\\n"', 'key: ', 'k: v\n', '  nested: 1\n', '- a\n', '- b: 1\n  c: 2\n', 'x y z', '#', ' #c', '\n', '\n\n', '---\n',
    '...', '"a\nb"', "'it''s'", 'é', '漢', '"\\\n  c"', ':', '?', '-', '- -', 'a:b', 'a: b: c', '[a, b', '{a: [1, 2], b',
];
const yaml = [];
const engineSpecific = result => /code point/.test(result);
function addYaml(source) {
    const result = attempt(() => encode(parseYaml(source)));
    if (!engineSpecific(result)) yaml.push([source, result]);
}
for (const source of YAML_CASES) addYaml(source);
for (let i = 0; i < 2500; i++) {
    let source = '';
    for (let n = 1 + Math.floor(random() * 8); n > 0; n--) source += pick(YAML_FRAGMENTS);
    addYaml(source);
}

const PRE_FRAGMENTS = [
    '---\n', '---', 'title: x\n', 'title: 12\n', 'title: ""\n', 'config:\n  theme: dark\n', 'config:\n  flowchart:\n    curve: linear\n',
    'displayMode: compact\n', 'config:\n  gantt: 5\n', '%% c\n', '  %% c\n', '%%\n', '%%{init: {"a":1}}%%\n',
    "%%{init: {'flowchart': {'x': 2}}}%%", '%%{wrap}%%', '%%{ init : {"b": {"c": 1}} }%%\n', '%%{initialize: {"d": [1]}}%%',
    '%%{', '}%%', '%%{x}%%', '%%{init: nope}%%', '%%{init: {"__proto__": {"p": 1}}}%%', 'graph TD\n', 'info\n', 'A-->B\n',
    '<span class="x">', '</span>', '<b a="1" c="2">', '<a', '>', '<', 'x="y"', '#35;', '#quot;', '#a', '#;', 'style A fill:#f00;\n',
    'classDef a fill:#0f0;\n', 'style B x:y;', '\r\n', '\r', '  ', '\t', ' ', '\n', 'é', '漢', ':', '#', ';', 'x: [1,\n', '"', "'",
    '\u00a0', '\u2028',
];
const pre = [];
const mapping = [];
function addPre(source) {
    const result = attempt(() => {
        const out = preprocess(source);
        return JSON.stringify({
            text: out.text,
            title: out.title ?? null,
            config: encode(out.config),
            withComments: out.withComments,
            lineOffset: out.lineOffset,
        });
    });
    pre.push([source, result]);
    if (/^[\x00-\x7f]*$/.test(source) && !result.startsWith('!')) {
        const steps = [];
        const out = preprocess(source, steps);
        const parsed = encodeEntities(cleanupComments(out.withComments, steps), steps);
        const length = encodeEntities(out.text).length + 1;
        mapping.push([source, Array.from({ length: length + 1 }, (_, at) => inputOffset(steps, at)), parsed]);
    }
}
for (const source of [
    '---\ntitle: My chart\nconfig:\n  flowchart:\n    curve: linear\n---\ngraph TD\n  A',
    "---\nconfig:\n  flowchart:\n    curve: linear\n    rankSpacing: 10\n---\n%%{init: {'flowchart': {'rankSpacing': 99}}}%%\ngraph TD\n  A",
    '%%{init: {"__proto__": {"polluted": true}, "constructor": {"prototype": {"polluted": true}}}}%%\ngraph TD\nA',
    '---\nconfig:\n  __proto__:\n    polluted: true\n---\ngraph TD\nA',
    'graph TD\r\nA["<span class="x">hi</span>"]\r',
    '%% one\ngraph TD\n  %% two\n  A --> B\n',
    '  ---\n  title: indented\n  ---\ngraph TD',
    '---\ntitle: a\n---\n---\ntitle: b\n---\ninfo',
    '---\n: bad\n---\ninfo',
    '\n\n  %% lead\ninfo\n',
]) addPre(source);
for (let i = 0; i < 2500; i++) {
    let source = '';
    for (let n = 1 + Math.floor(random() * 9); n > 0; n--) source += pick(PRE_FRAGMENTS);
    addPre(source);
}

const ENTITY_FRAGMENTS = ['style', 'classDef', ':', '#', ';', ' ', '\n', 'a', 'fill', '9', '\t', ':#', '#f9f;', '\r', '#35;', '#x;', 'é'];
const entities = [];
for (let i = 0; i < 2000; i++) {
    let source = '';
    for (let n = Math.floor(random() * 14); n > 0; n--) source += pick(ENTITY_FRAGMENTS);
    entities.push([source, encodeEntities(source)]);
}

writeFileSync(`${root}tests/fixtures/preprocess.json`, `${JSON.stringify({ yaml, pre, mapping, entities })}\n`);
console.log(`fixtures: ${yaml.length} yaml, ${pre.length} preprocess, ${mapping.length} mappings, ${entities.length} entities`);
