import { buildEventModel, type EventModelingModel } from '../../../src/diagrams/eventmodeling/model.js';
import { parseEventModel, type EventModelAst } from '../../../src/diagrams/eventmodeling/parser.js';
import { checkSourceFrameTypes, type Accept, type CheckedFrame } from './validator.js';
import { parse as peleParse } from '../../../src/index.js';
import { toResult } from '../../support/langium.js';

// The spec stubs text measurement on the DOM's SVGElement, which Pele does not use.
const scope = globalThis as { SVGElement?: unknown };
scope.SVGElement ??= class {};

export type EmModelEntityType = string;
export interface EmTimeFrame {
  $type: 'EmTimeFrame';
  $container: unknown;
  $containerProperty: undefined;
  $containerIndex: undefined;
  $cstNode: undefined;
  name: string;
  entityIdentifier: string;
  modelEntityType: EmModelEntityType;
  sourceFrames: { $refText: string; ref?: EmTimeFrame; error: undefined }[];
}
export interface EventModelingServices {
  validation: { EventModelingValidator: EventModelingValidator; ValidationRegistry?: { register(...args: unknown[]): void } };
}

export const eventModelingParse = (src: string) => toResult<EventModelAst>(parseEventModel, src);

export class EventModelingValidator {
  checkSourceFrameTypes(frame: CheckedFrame, accept: (severity: 'error', message: string) => void): void {
    checkSourceFrameTypes(frame, accept as Accept);
  }
}

// Langium wiring: hands the checks to the registry.
export function registerValidationChecks(services: EventModelingServices): void {
  const validator = services.validation.EventModelingValidator;
  services.validation.ValidationRegistry?.register(
    { EmTimeFrame: validator.checkSourceFrameTypes.bind(validator), EmResetFrame: validator.checkSourceFrameTypes.bind(validator) },
    validator
  );
}

let model: EventModelingModel | undefined;

// Mermaid's event modeling database is a module-level singleton; this one holds the last parsed model.
export const db = {
  clear(): void {
    model = undefined;
  },
  // Mermaid's layout state, of which the specs read the relations.
  getState() {
    if (!model) throw new Error('No data for EventModel');
    return {
      relations: model.relations.map((relation) => ({
        sourceBox: { frame: { name: relation.source.id } },
        targetBox: { frame: { name: relation.target.id } },
      })),
    };
  },
};

export const parser = {
  async parse(src: string): Promise<void> {
    model = undefined;
    model = buildEventModel(parseEventModel(src), undefined);
  },
};

export const mermaidAPI = {
  async parse(src: string): Promise<void> {
    peleParse(src);
  },
};
