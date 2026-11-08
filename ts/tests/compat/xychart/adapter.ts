import { XyChartDb, type DataPoint, type XyText } from '../../../src/diagrams/xychart/db.js';
import { parseXyChart } from '../../../src/diagrams/xychart/parser.js';

export const parser = {
  yy: undefined as unknown as XyChartDb,
  parse(src: string): void {
    parseXyChart(src, parser.yy);
  },
};

let db = new XyChartDb();

// Mermaid's XY chart database is a module-level singleton; this one holds a model that `clear` replaces.
export default {
  clear(): void {
    db = new XyChartDb();
  },
  setXAxisBand(categories: XyText[]): void {
    db.setXAxisBand(categories);
  },
  setLineData(title: XyText, data: DataPoint[]): void {
    db.setLineData(title, data);
  },
  setBarData(title: XyText, data: DataPoint[]): void {
    db.setBarData(title, data);
  },
  getXYChartData: (): XyChartDb => db,
};
