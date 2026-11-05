import { Relationships, RequirementDb, RequirementType, RiskLevel, VerifyType } from '../../../src/diagrams/requirement/db.js';
import { buildRequirementGraph } from '../../../src/diagrams/requirement/graph.js';
import { parseRequirement } from '../../../src/diagrams/requirement/parser.js';

export type { Relation } from '../../../src/diagrams/requirement/types.js';
export type RelationshipType = string;

export function setConfig(_config: unknown): void {}

// Exposes Pele's requirement model through the names Mermaid's specs call on RequirementDB.
export class RequirementDB extends RequirementDb {
  RequirementType = RequirementType;
  RiskLevel = RiskLevel;
  VerifyType = VerifyType;
  Relationships = Relationships;

  clear(): void {
    Object.assign(this, new RequirementDb());
  }

  getDirection() {
    return this.direction;
  }

  getRequirements() {
    return this.requirements;
  }

  getElements() {
    return this.elements;
  }

  getRelationships() {
    return this.relations;
  }

  getClasses() {
    return this.classes;
  }

  getAccTitle() {
    return this.accTitle ?? '';
  }

  getAccDescription() {
    return this.accDescr ?? '';
  }

  getData() {
    return { ...buildRequirementGraph(this), other: {}, config: {} };
  }
}

const parser = {
  yy: undefined as unknown as RequirementDB,
  parse(src: string): void {
    parseRequirement(src, parser.yy);
  },
  parser: undefined as unknown,
};
parser.parser = parser;

export default parser;
