import {
  ACC_DESCR,
  ACC_TITLE,
  DIRECTIVE,
  NEWLINE,
  Reader,
  SINGLE_LINE_COMMENT,
  STRING,
  WHITESPACE,
  YAML,
  accDescrValue,
  accTitleValue,
  stringValue,
  titleBefore,
  titleValue,
  tokenize,
  type TokenType,
} from '../common/tokens.js';

const enum T {
  header,
  junction,
  service,
  column,
  group,
  align,
  row,
  dashes,
  in,
  colon,
  dash,
  direction,
  arrowGroup,
  into,
  accDescr,
  accTitle,
  title,
  string,
  id,
  newline,
  whitespace,
  yaml,
  directive,
  comment,
  icon,
  label,
}

const ID: TokenType = { name: 'ID', pattern: /[\w]([-\w]*\w)?/y };

function word(text: string): TokenType {
  return { name: text, pattern: text, longer: [ID] };
}

// The direction letters come before identifiers and have no longer alternative, so Mermaid
// reads an id that starts with L, R, T or B as a direction followed by the rest.
export const ARCHITECTURE_TOKENS: readonly TokenType[] = [
  word('architecture-beta'),
  word('junction'),
  word('service'),
  word('column'),
  word('group'),
  word('align'),
  word('row'),
  { name: '--', pattern: '--', longer: [YAML] },
  word('in'),
  { name: ':', pattern: ':' },
  { name: '-', pattern: '-', longer: [YAML] },
  { name: 'ARROW_DIRECTION', pattern: /L|R|T|B/y },
  { name: 'ARROW_GROUP', pattern: /\{group\}/y },
  { name: 'ARROW_INTO', pattern: /<|>/y },
  ACC_DESCR,
  ACC_TITLE,
  titleBefore(String.raw`[-\w]*\w`),
  STRING,
  ID,
  NEWLINE,
  WHITESPACE,
  YAML,
  DIRECTIVE,
  SINGLE_LINE_COMMENT,
  { name: 'ARCH_ICON', pattern: /\([\w-:]+\)/y },
  { name: 'ARCH_TITLE', pattern: /\[(?:"([^"\\]|\\.)*"|'([^'\\]|\\.)*'|[^\[\]\r\n]+)\]/y },
];

export type Side = 'L' | 'R' | 'T' | 'B';

export interface GroupAst {
  $type: 'Group';
  id: string;
  icon?: string;
  title?: string;
  in?: string;
}

export interface ServiceAst {
  $type: 'Service';
  id: string;
  icon?: string;
  iconText?: string;
  title?: string;
  in?: string;
}

export interface JunctionAst {
  $type: 'Junction';
  id: string;
  in?: string;
}

export interface EdgeAst {
  $type: 'Edge';
  lhsId: string;
  lhsGroup: boolean;
  lhsDir: Side;
  lhsInto: boolean;
  title?: string;
  rhsInto: boolean;
  rhsDir: Side;
  rhsId: string;
  rhsGroup: boolean;
}

export interface AlignmentAst {
  $type: 'Alignment';
  direction: 'row' | 'column';
  members: string[];
}

export interface ArchitectureAst {
  $type: 'Architecture';
  title?: string;
  accTitle?: string;
  accDescr?: string;
  groups: GroupAst[];
  services: ServiceAst[];
  junctions: JunctionAst[];
  edges: EdgeAst[];
  alignments: AlignmentAst[];
}

// Mermaid's ArchitectureValueConverter for ARCH_TITLE.
function labelValue(text: string): string {
  let result = text.slice(1, -1).trim();
  if ((result.startsWith('"') && result.endsWith('"')) || (result.startsWith("'") && result.endsWith("'"))) {
    result = result.slice(1, -1).replace(/\\"/g, '"').replace(/\\'/g, "'");
  }
  return result.trim();
}

export function parseArchitecture(src: string): ArchitectureAst {
  const r = new Reader(tokenize(src, ARCHITECTURE_TOKENS, 'architecture'), ARCHITECTURE_TOKENS, 'architecture');
  const ast: ArchitectureAst = {
    $type: 'Architecture',
    groups: [],
    services: [],
    junctions: [],
    edges: [],
    alignments: [],
  };

  const is = (kind: T): boolean => r.kinds[r.i] === kind;
  const endOfLine = (): void => {
    if (r.kind === -1) return;
    if (r.kind !== T.newline) r.fail('a line break or the end of input');
    while (r.kind === T.newline) r.i++;
  };

  while (r.kind === T.newline) r.i++;
  r.expect(T.header);
  while (r.kind !== -1) {
    switch (r.kind) {
      case T.newline:
        r.i++;
        continue;
      case T.accDescr:
        ast.accDescr = accDescrValue(r.take());
        break;
      case T.accTitle:
        ast.accTitle = accTitleValue(r.take());
        break;
      case T.title:
        ast.title = titleValue(r.take());
        break;
      case T.group: {
        r.i++;
        const group: GroupAst = { $type: 'Group', id: r.expect(T.id) };
        if (is(T.icon)) group.icon = r.take().slice(1, -1);
        if (is(T.label)) group.title = labelValue(r.take());
        if (r.accept(T.in)) group.in = r.expect(T.id);
        ast.groups.push(group);
        break;
      }
      case T.service: {
        r.i++;
        const service: ServiceAst = { $type: 'Service', id: r.expect(T.id) };
        if (is(T.string)) service.iconText = stringValue(r.take());
        else if (is(T.icon)) service.icon = r.take().slice(1, -1);
        if (is(T.label)) service.title = labelValue(r.take());
        if (r.accept(T.in)) service.in = r.expect(T.id);
        ast.services.push(service);
        break;
      }
      case T.junction: {
        r.i++;
        const junction: JunctionAst = { $type: 'Junction', id: r.expect(T.id) };
        if (r.accept(T.in)) junction.in = r.expect(T.id);
        ast.junctions.push(junction);
        break;
      }
      case T.id: {
        const lhsId = r.take();
        const lhsGroup = r.accept(T.arrowGroup);
        r.expect(T.colon);
        const lhsDir = r.expect(T.direction) as Side;
        const lhsInto = r.accept(T.into);
        let title: string | undefined;
        if (!r.accept(T.dashes)) {
          if (!is(T.dash)) r.fail("'--' or '-'");
          r.i++;
          title = labelValue(r.expect(T.label));
          r.expect(T.dash);
        }
        const rhsInto = r.accept(T.into);
        const rhsDir = r.expect(T.direction) as Side;
        r.expect(T.colon);
        const rhsId = r.expect(T.id);
        const rhsGroup = r.accept(T.arrowGroup);
        ast.edges.push({ $type: 'Edge', lhsId, lhsGroup, lhsDir, lhsInto, title, rhsInto, rhsDir, rhsId, rhsGroup });
        break;
      }
      case T.align: {
        r.i++;
        if (!is(T.row) && !is(T.column)) r.fail("'row' or 'column'");
        const direction = r.take() as 'row' | 'column';
        const members = [r.expect(T.id), r.expect(T.id)];
        while (is(T.id)) members.push(r.take());
        ast.alignments.push({ $type: 'Alignment', direction, members });
        break;
      }
      default:
        r.fail('a group, a service, a junction, an edge, an alignment, a title, or a line break');
    }
    endOfLine();
  }
  return ast;
}
