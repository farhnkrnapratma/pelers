const SPECIALS = [
  '"',
  "$",
  "'",
  "[",
  "]",
  "(",
  ")",
  "{",
  "}",
  "|",
  ":",
  ";",
  ",",
  "-->",
  "---",
  "==>",
  "%%",
  "#",
  "\t",
  " ",
  "\n",
  "Ü",
  "中",
  "😀",
  "&",
  "<",
  ">",
  "\\",
  "$",
  "*",
  "+",
  ".",
  "?",
  "!",
  "~",
  "`",
  "%",
  "@",
];

export function rng(seed) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function tokens(source) {
  return source.split(/(\s+|[^\w\s])/).filter((t) => t.length > 0);
}

export function mutate(source, random) {
  const lines = source.split("\n");
  const op = Math.floor(random() * 9);
  const at = (n) => Math.floor(random() * Math.max(1, n));
  switch (op) {
    case 0:
      if (lines.length > 2) lines.splice(at(lines.length - 1) + 1, 1);
      return lines.join("\n");
    case 1: {
      const i = at(lines.length);
      lines.splice(i, 0, lines[at(lines.length)]);
      return lines.join("\n");
    }
    case 2: {
      const i = at(lines.length);
      const j = at(lines.length);
      [lines[i], lines[j]] = [lines[j], lines[i]];
      return lines.join("\n");
    }
    case 3: {
      const toks = tokens(source);
      const t = toks[at(toks.length)];
      const pick = toks[at(toks.length)];
      toks[toks.indexOf(t)] = pick;
      return toks.join("");
    }
    case 4: {
      const pos = at(source.length + 1);
      return (
        source.slice(0, pos) + SPECIALS[at(SPECIALS.length)] + source.slice(pos)
      );
    }
    case 5: {
      const pos = at(source.length + 1);
      return source.slice(0, pos) + source.slice(pos + 1 + at(4));
    }
    case 6:
      return source.slice(0, at(source.length + 1));
    case 7: {
      const toks = tokens(source);
      toks.splice(at(toks.length), 0, SPECIALS[at(SPECIALS.length)]);
      return toks.join("");
    }
    default: {
      const i = at(lines.length);
      lines[i] = lines[i].replace(/\s+/g, " ").trim();
      return lines.join("\n");
    }
  }
}

const DATE_TOKENS = [
  "YYYY",
  "YY",
  "MMMM",
  "MMM",
  "MM",
  "M",
  "Do",
  "DDDD",
  "DD",
  "D",
  "dddd",
  "ddd",
  "d",
  "HH",
  "H",
  "hh",
  "h",
  "mm",
  "m",
  "ss",
  "s",
  "SSS",
  "A",
  "a",
  "ZZ",
  "Z",
  "X",
  "x",
  "W",
  "Q",
  "w",
  "ww",
  "gggg",
  "-",
  "/",
  " ",
  ":",
  "T",
  ",",
];
const AXIS_TOKENS = [
  "%a",
  "%A",
  "%b",
  "%B",
  "%d",
  "%e",
  "%f",
  "%H",
  "%I",
  "%j",
  "%L",
  "%m",
  "%M",
  "%p",
  "%q",
  "%Q",
  "%s",
  "%S",
  "%u",
  "%U",
  "%V",
  "%w",
  "%W",
  "%x",
  "%X",
  "%y",
  "%Y",
  "%Z",
  "%%",
  "%c",
  "%-d",
  "%_m",
  "%0d",
  "%-H",
  "%3f",
  " ",
  "-",
  ":",
];

export function formatVariants(source, random, count) {
  const variants = [];
  const directive = /^(\s*)(dateFormat|axisFormat)([ \t]+)(.*)$/m;
  const match = source.match(directive);
  if (!match) return variants;
  const tokens = match[2] === "dateFormat" ? DATE_TOKENS : AXIS_TOKENS;
  for (let k = 0; k < count; k++) {
    const length = 1 + Math.floor(random() * 6);
    let value = "";
    for (let j = 0; j < length; j++)
      value += tokens[Math.floor(random() * tokens.length)];
    variants.push(
      source.replace(
        directive,
        (_, indent, key, gap) => indent + key + gap + value,
      ),
    );
  }
  return variants;
}

const GANTT_UNITS = [
  "millisecond",
  "second",
  "minute",
  "hour",
  "day",
  "week",
  "month",
  "year",
  "milliseconds",
  "weeks",
  "months",
  "bogus",
];
const GANTT_WEEKDAYS = [
  "sunday",
  "monday",
  "tuesday",
  "wednesday",
  "thursday",
  "friday",
  "saturday",
  "Friday",
  "bogus",
];
const GANTT_DAYS = [
  "2024-01-01",
  "2024-02-29",
  "2024-12-25",
  "not-a-date",
  "friday",
  "saturday",
  "weekends",
  "sunday",
];

const GANTT_EXTRA = [
  "%% a comment",
  '%%{init: {"theme": "dark"}}%%',
  "%%{ unclosed",
  "accTitle: Title here",
  "accTitle Title no colon",
  "accDescr: One line description",
  "accDescr {\n  multi line\n}",
  "accDescr {",
  "title Plan title",
  "section Extra",
];

function ganttDirective(random) {
  if (random() < 0.3)
    return GANTT_EXTRA[Math.floor(random() * GANTT_EXTRA.length)];
  switch (Math.floor(random() * 8)) {
    case 0:
      return `tickInterval ${Math.floor(random() * 12)}${random() < 0.2 ? " " : ""}${GANTT_UNITS[Math.floor(random() * GANTT_UNITS.length)]}`;
    case 1:
      return `weekday ${GANTT_WEEKDAYS[Math.floor(random() * GANTT_WEEKDAYS.length)]}`;
    case 2:
      return `weekend ${GANTT_WEEKDAYS[Math.floor(random() * GANTT_WEEKDAYS.length)]}`;
    case 3:
      return `excludes ${GANTT_DAYS[Math.floor(random() * GANTT_DAYS.length)]}`;
    case 4:
      return `includes ${GANTT_DAYS[Math.floor(random() * GANTT_DAYS.length)]}`;
    case 5:
      return (
        "todayMarker " +
        ["off", "stroke:#f00,stroke-width:2px", "bogus"][
          Math.floor(random() * 3)
        ]
      );
    case 6:
      return `excludes ${GANTT_DAYS[Math.floor(random() * GANTT_DAYS.length)]}, ${GANTT_DAYS[Math.floor(random() * GANTT_DAYS.length)]}`;
    default:
      return `dateFormat ${["YYYY-MM-DD", "DD/MM/YYYY", "X", "x"][Math.floor(random() * 4)]}`;
  }
}

function ganttDirectives(source, random, count) {
  if (!/^\s*gantt\b/.test(source)) return [];
  const lines = source.split("\n");
  const variants = [];
  for (let k = 0; k < count; k++) {
    const copy = [...lines];
    copy.splice(1, 0, ganttDirective(random));
    variants.push(copy.join("\n"));
  }
  return variants;
}

const SEQUENCE_STATEMENTS = [
  "participant P as Alice",
  "actor P",
  'participant P@{ "type": "database" }',
  "autonumber",
  "autonumber 10 5",
  "autonumber off",
  "Note left of P: note",
  "Note right of P: note",
  "Note over P,Q: note",
  "rect rgb(0,0,255)",
  "end",
  "loop check",
  "alt ok",
  "else other",
  "opt maybe",
  "par first",
  "and second",
  "critical c",
  "option d",
  "break stop",
  "box Aqua Group",
  "activate P",
  "deactivate P",
  "P->>Q: hi",
  "P-->>Q: back",
  "P-)Q: async",
  "P-xQ: lost",
  "P<<->>Q: both",
  "P--xQ: cross",
  "P->Q: plain",
  "P+>Q: activate",
  "P-->>-Q: deactivate",
  "link P: Dashboard @ https://example.com",
  "create participant R",
  "destroy R",
  "title: Demo",
  "%% comment",
  "P-|\\Q: top",
  "P-|/Q: bottom",
  "P-\\Q: stick top",
  "P-//Q: stick bottom",
  "P--|\\Q: dotted top",
  "P--|/Q: dotted bottom",
  "P--\\Q: dotted stick top",
  "P--//Q: dotted stick bottom",
  "P/|-Q: reverse top",
  "P\\|-Q: reverse bottom",
  "P//-Q: reverse stick top",
  "P\\\\-Q: reverse stick bottom",
  "P/|--Q: dotted reverse top",
  "P\\|--Q: dotted reverse bottom",
  "P//--Q: dotted reverse stick top",
  "P\\\\--Q: dotted reverse stick bottom",
];
const CLASS_STATEMENTS = [
  "class C1 {",
  "  +int x",
  "  -foo()",
  "}",
  "C1 <|-- C2",
  "C1 *-- C2",
  "C1 o-- C2",
  "C1 --> C2 : label",
  "C1 .. C2",
  "C1 : +m()",
  "<<interface>> C1",
  'note for C1 "text"',
  "direction RL",
  'cssClass "C1" hi',
  "click C1 call fn()",
  "classDef foo fill:#f00",
  'C1 "1" --> "*" C2',
  "namespace N { class X }",
  "C1 : +name String",
  "C1 ..> C2 : uses",
  "C1 --|> C2",
  "classDef fam font-family:Arial",
  "style C1 font-family:Courier",
  "style C1 text-decoration:underline",
  "classDef dec text-decoration:underline",
  "C1 : +m() $$x^2$$",
  'note for C1 "$$y$$"',
  "class C2:::fam",
  "C2 : +n() $$z$$",
  "C1 --> C1 : loop",
  "C1 o-- C1",
  "C2 ..> C2 : self",
  "<<$$k$$>> C2",
  'C2 "1" --> "*" C2 : self',
  "C1 *-- C1 : part",
  "C1 <|-- C1",
];
const ER_STATEMENTS = [
  "A ||--o{ B : has",
  "A }o--|| B : x",
  "A |o--o| B : y",
  "A ||--|{ B : z",
  "A {}--|| B : w",
  "A ||..o{ B : dash",
  'entity A {\n  int id PK\n  string name "x" UK\n}',
  "direction LR",
  "classDef x fill:#f00",
  "style A fill:#0f0",
  "class A x",
  "A ||--|| C : one",
];

function insertStatements(source, random, count, statements, header) {
  if (!new RegExp("^\\s*" + header + "\\b").test(source)) return [];
  const lines = source.split("\n");
  const variants = [];
  for (let k = 0; k < count; k++) {
    const copy = [...lines];
    const at = 1 + Math.floor(random() * Math.max(1, copy.length - 1));
    copy.splice(at, 0, statements[Math.floor(random() * statements.length)]);
    variants.push(copy.join("\n"));
  }
  return variants;
}

const USECASE_IDS = ["User", "Login", "Admin", "Sys", "Pay"];
const USECASE_TYPES = [
  "normal",
  "hollow",
  "awesome",
  "icon",
  "giant",
  "rect",
  "package",
  "edge",
  "boundary",
  "",
];
const USECASE_PROPERTIES = [
  () => ['icon: "fa:user"', "icon: user", 'icon: ""'],
  () => ["business: true", "business: false"],
  () => ["animate: true", "animate: false", "animation: fade"],
  () => ["fillColor: red", "fillColor: #0f0"],
  () => ['label: "Long label"'],
];

const USECASE_CONFLICTS = [
  'Authorize("Other label")',
  'Login("Sign in again")',
  'Login["Sign in"]',
  'systemBoundary sb9["Other"]',
  'systemBoundary sb1["Payment service"]',
  "Login -.-> Pay : <<include>>",
  "Login -.-> Pay : <<extend>>",
  "Login -.-> Pay : <<bogus>>",
  'Pay["Pay"]',
  "end",
  "actor Admin@{ type: normal }",
  "boundary Sys@{ type: rect }",
  'Authorize("Authorize payment")',
  'Login("Login")@{ business: true }',
];

function usecaseStatement(random) {
  if (random() < 0.35)
    return USECASE_CONFLICTS[Math.floor(random() * USECASE_CONFLICTS.length)];
  const id = USECASE_IDS[Math.floor(random() * USECASE_IDS.length)];
  const kind = ["actor", "usecase", "boundary", "edge"][
    Math.floor(random() * 4)
  ];
  const pick = (list) => list[Math.floor(random() * list.length)];
  if (kind === "edge") {
    const other = USECASE_IDS[Math.floor(random() * USECASE_IDS.length)];
    return (
      id +
      " --> " +
      other +
      "@{ " +
      pick(
        USECASE_PROPERTIES[Math.floor(random() * USECASE_PROPERTIES.length)](),
      )[0] +
      " }"
    );
  }
  const props = [];
  const count = 1 + Math.floor(random() * 3);
  for (let k = 0; k < count; k++) {
    if (random() < 0.4) props.push("type: " + pick(USECASE_TYPES));
    else
      props.push(
        pick(
          USECASE_PROPERTIES[
            Math.floor(random() * USECASE_PROPERTIES.length)
          ](),
        ),
      );
  }
  return kind + " " + id + "@{ " + props.join(", ") + " }";
}

function usecaseStatements(source, random, count) {
  if (!/^\s*usecase-beta\b/.test(source)) return [];
  const lines = source.split("\n");
  const variants = [];
  for (let k = 0; k < count; k++) {
    const copy = [...lines];
    copy.splice(1, 0, usecaseStatement(random));
    variants.push(copy.join("\n"));
  }
  return variants;
}

const MATH_BODIES = [
  "x^2",
  "\\frac{a}{b}",
  "\\sum_{i=0}^{n} i",
  "",
  "unclosed",
  "a$b",
  "\\alpha + \\beta",
  "\\sqrt{2}",
  "\\text{hi}",
];

function mathVariants(source, random, count) {
  const variants = [];
  for (let k = 0; k < count; k++) {
    const body = MATH_BODIES[Math.floor(random() * MATH_BODIES.length)];
    const replaced = source.replace(/\[([^\]\n]*)\]|"([^"\n]*)"/g, (whole) => {
      if (random() < 0.5) return whole;
      return whole[0] + "$" + body + "$" + whole[whole.length - 1];
    });
    variants.push(replaced);
  }
  return variants;
}

const BLOCK_STATEMENTS = [
  "columns 3",
  "columns auto",
  "space",
  "space:2",
  'a["Square"]:2',
  'b(("circle"))',
  'c{"diamond"}',
  'd>"asym"]',
  'e[("database")]',
  'f[["subroutine"]]',
  'g[/"parallelogram"/]',
  'h[\\"alt"/]',
  'i("round")',
  'j(["stadium"])',
  'k{{"hexagon"}}',
  "block:grp\n  columns 2\n  m n\nend",
  "x --> y",
  "x -.-> y",
  "x ==> y",
  "x --x y",
  "x --o y",
  'x -- "label" --> y',
  'x -."dotted".-> y',
  "classDef cls fill:#f00",
  "class x cls",
  "style x fill:#0f0",
  "x:2",
  'q<["arrow"]>(right)',
  'r<["arrow"]>(down)',
  "x --> x",
  "y --> z:3",
];

const FLOW_STATEMENTS = [
  "A --> B",
  "A -.-> B",
  "A ==> B",
  "A --- B",
  "A -- text --> B",
  "A -. text .-> B",
  "A == text ==> B",
  "A --x B",
  "A --o B",
  "A <--> B",
  "A o--o B",
  "A x--x B",
  "subgraph S1 [Title]",
  "subgraph S2",
  "end",
  "classDef c fill:#f00",
  "class A c",
  "style A fill:#0f0",
  "linkStyle 0 stroke:#f00",
  'click A "https://example.com"',
  'click A callback "tip"',
  "A@{ shape: rect }",
  "A((circle))",
  "A>asym]",
  "A[(db)]",
  "A[[sub]]",
  "A{{hex}}",
  "A[/para/]",
  "A[\\alt/]",
  "A(round)",
  "A([stadium])",
  "direction LR",
  "%% comment",
  "A:::c",
  "A & B --> C",
  "A -->|label| B",
  "A ---|label| B",
  "A --> B & C",
  'subgraph "quoted title"',
  'A["$$x^2$$"] --> B',
  'A@{ icon: "fa:bell" } --> B',
];

export function mutantCases(corpus, perSource, seed0) {
  const cases = [];
  corpus.forEach(({ type, source }, index) => {
    if (type === "info") return;
    const random = rng(seed0 + index);
    const texts = [source];
    for (let k = 0; k < perSource; k++) texts.push(mutate(source, random));
    texts.push(...formatVariants(source, random, perSource));
    texts.push(...ganttDirectives(source, random, perSource));
    texts.push(
      ...insertStatements(
        source,
        random,
        perSource,
        SEQUENCE_STATEMENTS,
        "sequenceDiagram",
      ),
    );
    texts.push(
      ...insertStatements(
        source,
        random,
        perSource,
        CLASS_STATEMENTS,
        "classDiagram",
      ),
    );
    texts.push(
      ...insertStatements(
        source,
        random,
        perSource,
        ER_STATEMENTS,
        "erDiagram",
      ),
    );
    texts.push(...usecaseStatements(source, random, perSource));
    texts.push(
      ...insertStatements(
        source,
        random,
        perSource,
        BLOCK_STATEMENTS,
        "block-beta",
      ),
    );
    texts.push(
      ...insertStatements(
        source,
        random,
        perSource,
        FLOW_STATEMENTS,
        "flowchart",
      ),
    );
    texts.push(...mathVariants(source, random, perSource));
    for (const text of texts)
      cases.push({ index, type, source: text.toWellFormed() });
  });
  return cases;
}
