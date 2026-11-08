export interface XyText {
  text: string;
  type: 'text' | 'markdown';
}

export interface DataPoint {
  value: number;
  label: string;
}

export interface BandAxis {
  type: 'band';
  title: string;
  categories: string[];
}

export interface LinearAxis {
  type: 'linear';
  title: string;
  min: number;
  max: number;
}

// Each value with its place on the x axis: a category, or a number written as text.
// A category with no value has `undefined`.
export type PlotData = [string, number | undefined][];

export interface Plot {
  type: 'line' | 'bar';
  title: string;
  data: PlotData;
  pointLabels?: string[];
}

export interface XyChartModel {
  type: 'xychart';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  orientation: 'vertical' | 'horizontal' | undefined;
  xAxis: BandAxis | LinearAxis;
  yAxis: LinearAxis;
  // False while the y range comes from the data and not from a `y-axis` statement.
  hasSetYAxis: boolean;
  plots: Plot[];
}

export class XyChartDb implements XyChartModel {
  readonly type = 'xychart';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  orientation: 'vertical' | 'horizontal' | undefined;
  xAxis: BandAxis | LinearAxis = { type: 'band', title: '', categories: [] };
  yAxis: LinearAxis = { type: 'linear', title: '', min: Infinity, max: -Infinity };
  hasSetXAxis = false;
  hasSetYAxis = false;
  plots: Plot[] = [];

  setOrientation(orientation: string): void {
    this.orientation = orientation === 'horizontal' ? 'horizontal' : 'vertical';
  }

  setXAxisTitle(title: XyText): void {
    this.xAxis.title = title.text.trim();
  }

  setXAxisRangeData(min: number, max: number): void {
    this.xAxis = { type: 'linear', title: this.xAxis.title, min, max };
    this.hasSetXAxis = true;
  }

  setXAxisBand(categories: XyText[]): void {
    this.xAxis = { type: 'band', title: this.xAxis.title, categories: categories.map((c) => c.text.trim()) };
    this.hasSetXAxis = true;
  }

  setYAxisTitle(title: XyText): void {
    this.yAxis.title = title.text.trim();
  }

  setYAxisRangeData(min: number, max: number): void {
    this.yAxis = { type: 'linear', title: this.yAxis.title, min, max };
    this.hasSetYAxis = true;
  }

  private plotData(values: number[]): PlotData {
    if (values.length === 0) return [];
    // The first series fixes the x range when no x-axis statement did.
    if (!this.hasSetXAxis) this.setXAxisRangeData(1, values.length);
    const x = this.xAxis;
    if (x.type === 'band' && values.length > x.categories.length) values = values.slice(0, x.categories.length);
    if (!this.hasSetYAxis) {
      let { min, max } = this.yAxis;
      for (const value of values) {
        if (value < min) min = value;
        if (value > max) max = value;
      }
      this.yAxis = { type: 'linear', title: this.yAxis.title, min, max };
    }
    if (x.type === 'band') return x.categories.map((category, i) => [category, values[i]]);
    if (values.length === 1) return [[`${x.min}`, values[0]]];
    const step = (x.max - x.min) / (values.length - 1);
    return values.map((value, i) => [`${x.min + i * step}`, value]);
  }

  private addPlot(type: 'line' | 'bar', title: XyText, data: DataPoint[]): void {
    const plot: Plot = { type, title: title.text.trim(), data: this.plotData(data.map((d) => d.value)) };
    // Mermaid accepts labels on the points of a bar series and ignores them.
    if (type === 'line' && data.some((d) => d.label.trim() !== '')) plot.pointLabels = data.map((d) => d.label.trim());
    this.plots.push(plot);
  }

  setLineData(title: XyText, data: DataPoint[]): void {
    this.addPlot('line', title, data);
  }

  setBarData(title: XyText, data: DataPoint[]): void {
    this.addPlot('bar', title, data);
  }

  setDiagramTitle(text: string): void {
    this.title = text;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }
}
