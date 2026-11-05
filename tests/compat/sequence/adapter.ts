import { detect } from '../../../src/detect.js';
import { SeqDb } from '../../../src/diagrams/sequence/db.js';
import { sequence } from '../../../src/diagrams/sequence/index.js';
import { parseSequence } from '../../../src/diagrams/sequence/parser.js';
import { numberMessages, renderSequence } from '../../../src/diagrams/sequence/render.js';
import type { SeqActor, SeqMessage } from '../../../src/diagrams/sequence/types.js';
import { encodeEntities, preprocess, type Config } from '../../../src/preprocess.js';
import { metricsMeasurer } from '../../../src/text/measurer.js';
import { ARROWTYPE, LINETYPE, PLACEMENT } from '../../support/sequence-constants.js';

let siteConfig: Config = {};
let config: Config = {};

function clone(value: Config): Config {
  return JSON.parse(JSON.stringify(value)) as Config;
}

export function setSiteConfig(next: Config): void {
  siteConfig = clone(next);
  config = clone(next);
}

export function addDiagrams(): void {}

type ActorView = Omit<SeqActor, 'links' | 'properties'> & {
  links: Record<string, unknown>;
  properties: Record<string, unknown>;
};

function actorView(actor: SeqActor): ActorView {
  return { ...actor, links: Object.fromEntries(actor.links), properties: Object.fromEntries(actor.properties) };
}

// Exposes Pele's sequence model through the method names Mermaid's specs call on SequenceDB.
// Mermaid keeps participant links and properties in plain objects; the views here do the same.
export class SequenceDB extends SeqDb {
  readonly LINETYPE = LINETYPE;
  readonly ARROWTYPE = ARROWTYPE;
  readonly PLACEMENT = PLACEMENT;
  private numbersShown = false;

  constructor() {
    const seq = (config.sequence ?? {}) as Config;
    super({ wrap: typeof config.wrap === 'boolean' ? config.wrap : undefined, sequenceWrap: seq.wrap === true });
  }

  getActors(): Map<string, ActorView> {
    return new Map([...this.actors].map(([id, actor]) => [id, actorView(actor)]));
  }

  getActor(id: string): ActorView {
    return actorView(this.actors.get(id)!);
  }

  getActorKeys(): string[] {
    return [...this.actors.keys()];
  }

  getMessages(): SeqMessage[] {
    return this.messages;
  }

  getBoxes() {
    return this.boxes;
  }

  getCreatedActors() {
    return this.createdActors;
  }

  getDestroyedActors() {
    return this.destroyedActors;
  }

  getDiagramTitle(): string {
    return this.title ?? '';
  }

  getAccTitle(): string {
    return this.accTitle ?? '';
  }

  getAccDescription(): string {
    return this.accDescr ?? '';
  }

  getConfig() {
    return config.sequence;
  }

  enableSequenceNumbers(): void {
    this.numbersShown = true;
  }

  disableSequenceNumbers(): void {
    this.numbersShown = false;
  }

  showSequenceNumbers(): boolean {
    return this.numbersShown;
  }
}

// Mermaid's renderer numbers the messages while it draws and leaves the result on the model.
// Here Pele's renderer runs, and the numbers it would draw are copied to where the specs look.
const renderer = {
  draw(_text: string, _id: string, _version: string, diagram: { db: SequenceDB }): void {
    const db = diagram.db;
    renderSequence(db, config, { measurer: metricsMeasurer });
    const numbering = numberMessages(db.messages, false);
    db.messages.forEach((message, i) => {
      (message as SeqMessage & { msgModel?: object }).msgModel = Number.isNaN(numbering.index[i])
        ? {}
        : { sequenceIndex: numbering.index[i], sequenceVisible: numbering.shown[i] };
    });
    if (numbering.visible) db.enableSequenceNumbers();
    else db.disableSequenceNumbers();
  },
  // The bounds bookkeeping of Mermaid's renderer has no counterpart; the specs that test it are skipped.
  bounds: { init(): void {} },
};

function build(source: string): SequenceDB {
  const db = new SequenceDB();
  parseSequence(source, db);
  return db;
}

// Like Mermaid's Diagram.fromText, this parses the text as given, without preprocessing.
export class Diagram {
  static async fromText(text: string): Promise<{ db: SequenceDB; renderer: typeof renderer; type: string }> {
    if (detect(preprocess(text).text) !== 'sequence') throw new Error('Not a sequence diagram.');
    return { db: build(encodeEntities(text) + '\n'), renderer, type: 'sequence' };
  }
}

export const mermaidAPI = {
  initialize(next: Config): void {
    setSiteConfig({ ...siteConfig, ...next });
  },
  reset(): void {
    config = clone(siteConfig);
  },
  getConfig(): Config {
    return config;
  },
  // The whole of Pele's pipeline: front matter, directives, comments, then the parser.
  async parse(text: string): Promise<{ diagramType: string }> {
    const pre = preprocess(text);
    if (detect(pre.text) !== 'sequence') throw new Error('Not a sequence diagram.');
    sequence.parse(encodeEntities(pre.text) + '\n', { ...config, ...pre.config }, pre.title);
    return { diagramType: 'sequence' };
  },
};

export const parser = {
  yy: undefined as unknown as SequenceDB,
  parse(src: string): void {
    parseSequence(src, parser.yy);
  },
};
