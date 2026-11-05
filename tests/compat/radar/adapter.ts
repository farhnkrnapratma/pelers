import { buildRadar, type RadarModel } from '../../../src/diagrams/radar/model.js';
import { parseRadar, type RadarAst } from '../../../src/diagrams/radar/parser.js';
import { PeleError, parse as peleParse, render } from '../../../src/index.js';
import { metricsMeasurer } from '../../../src/text/measurer.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';
export { closedRoundCurve, relativeRadius } from '../../../src/diagrams/radar/render.js';

export const Radar = { $type: 'Radar' };

export const radarParse = (src: string) => toResult<RadarAst>(parseRadar, src);

export const MermaidParseError = PeleError;

export async function parse(_type: string, src: string): Promise<RadarAst> {
  return parseRadar(src);
}

const DEFAULTS = { showLegend: true, ticks: 5, max: null, min: 0, graticule: 'circle' };

let model: RadarModel | undefined;

// Mermaid's radar database is a module-level singleton; this one holds the last parsed model.
export const db = {
  clear(): void {
    model = undefined;
  },
  getConfig: () => ({}),
  getAxes: () => model?.axes ?? [],
  getCurves: () => model?.curves ?? [],
  getOptions: () => model?.options ?? { ...DEFAULTS },
  getDiagramTitle: () => model?.title ?? '',
  getAccTitle: () => model?.accTitle ?? '',
  getAccDescription: () => model?.accDescr ?? '',
};

export const parser = {
  async parse(src: string): Promise<void> {
    model = buildRadar(parseRadar(src), undefined);
  },
};

// The spec's drawing tests go through Mermaid's API; here they go through Pele's.
export const mermaidAPI = {
  async parse(src: string): Promise<void> {
    peleParse(src);
  },
};

export const Diagram = {
  async fromText(src: string) {
    return {
      renderer: {
        async draw(): Promise<void> {
          render(src, { measurer: metricsMeasurer });
        },
      },
    };
  },
};
