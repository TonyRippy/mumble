// Copyright (C) 2023, Tony Rippy
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import * as mdb from 'mdb-ui-kit';
export default {
  mdb,
};

import { PrometheusDriver, SampleValue } from 'prometheus-query';
import { Func, linearFunction, ECDF, getRenderFuncForPlot, getRenderFuncForFunction, Layer, CDFGraph } from 'ecdfs';
import { CDF } from "./cdf";
import { Normal, LogNormal } from "./gaussian";
import { KSTest } from "./kolmogorov";
import { viridis } from "./colormap";

function resizeCanvasToDisplaySize(canvas) {
  // Lookup the size the browser is displaying the canvas in CSS pixels.
  const dpr = window.devicePixelRatio;
  const displayWidth  = Math.floor(canvas.clientWidth * dpr);
  const displayHeight = Math.floor(canvas.clientHeight * dpr);

  // Check if the canvas is not the same size.
  const needResize = canvas.width  !== displayWidth ||
    canvas.height !== displayHeight;

  if (needResize) {
    // Make the canvas the same size
    canvas.width  = displayWidth;
    canvas.height = displayHeight;
  }

  return needResize;
}

function getCdfFactory() {
  let distSelect = <HTMLSelectElement>document.getElementById('distype')
  let index = distSelect.selectedIndex;
  if (index == 0) {
    return Normal;
  }
  if (index == 1) {
    return LogNormal;
  }
  throw new Error("Unrecognized distribution type.");
}

//const ITERATIONS_PER_ROUND = 100;
//const MAX_ROUNDS = 50;

const LABEL_FONT = '16px serif';
const XMARGIN = 40;
const YMARGIN = 30;

interface KSPoint {
  cx: number
  cy: number
  mean: number
  stddev: number
  cdf: CDF
  p: number
}

class KSPlot {
  private cx2fx: Func;
  private cy2fy: Func;
  private test: (cdf: CDF) => number;
  private maxp: number;
  private imageData: ImageData;
  private best: KSPoint;
  private selection: KSPoint;

  constructor(private canvas: HTMLCanvasElement,
              private mean: number, private stddev: number,
              sample: number[],
              private cdfFactory: (x: number, y: number) => CDF,
              private gamma: number = 6.5) {
    this.test = (cdf: CDF) => {
      return KSTest(cdf, sample);
    };
    this.canvas = <HTMLCanvasElement>document.getElementById('plot');
    this.best = null;
    this.selection = null;
    this.onResize();
  }

  public onResize() {
    this.cx2fx = linearFunction(0, 0, this.canvas.width - XMARGIN, 2 * this.mean);
    this.cy2fy = linearFunction(0, 2 * this.stddev, this.canvas.height - YMARGIN, 0);
    this.recalculate();
    this.draw();
  }

  public getSelection(): KSPoint {
    if (this.selection != null) {
      return this.selection;
    }
    if (this.best != null) {
      return this.best;
    }
    let cdf = this.cdfFactory(this.mean, this.stddev);
    return {
      cx: (this.canvas.width - XMARGIN) / 2,
      cy: (this.canvas.height - YMARGIN) / 2,
      mean: this.mean,
      stddev: this.stddev,
      cdf: cdf,
      p: this.test(cdf),
    };
  }

  public setSelection(e: MouseEvent): KSPoint {
    const dpr = window.devicePixelRatio;
    let cx = e.offsetX * dpr;
    let cy = e.offsetY * dpr;
    let mean = this.cx2fx.eval(cx - XMARGIN);
    let stddev = Math.max(this.cy2fy.eval(cy), 0);
    let cdf = this.cdfFactory(mean, stddev);
    this.selection = {
      cx: cx,
      cy: cy,
      mean: mean,
      stddev: stddev,
      cdf: cdf,
      p: this.test(cdf),
    };
    this.draw();
    return this.selection;
  }

  public setCdfFactory(cdfFactory: (x: number, y: number) => CDF): void {
    this.cdfFactory = cdfFactory;
    this.recalculate();
    this.draw();
  }

  public setGamma(gamma: number): void {
    this.gamma = gamma;
    this.recalculate();
    this.draw();
  }

  public recalculate() {
    let w = this.canvas.width - XMARGIN;
    let h = this.canvas.height - YMARGIN;
    this.selection = null;
    let bestP = 0;
    let pixels = new Uint8ClampedArray(w * h * 4);
    for (let cy = 0; cy < h; cy++) {
      let fy = this.cy2fy.eval(cy);
      let i = cy * w * 4;
      for (let cx = 0; cx < w; cx++) {
        let fx = this.cx2fx.eval(cx);
        let cdf = this.cdfFactory(fx, fy);
        let p = this.test(cdf);
        if (p > bestP) {
          bestP = p;
          this.best = {
            cx: cx + XMARGIN,
            cy: cy,
            mean: fx,
            stddev: fy,
            cdf: cdf,
            p: p};
        }
        let color = viridis(Math.pow(p,this.gamma));
        pixels[i] = color.r;
        i++;
        pixels[i] = color.g;
        i++;
        pixels[i] = color.b;
        i++;
        pixels[i] = color.a;
        i++;
      }
    }
    this.imageData = new ImageData(pixels, w, h);
  }

  private drawAxes(ctx: CanvasRenderingContext2D, w: number, h: number) {
    //ctx.font = window.getComputedStyle(this.canvas, null).getPropertyValue('font');
    ctx.font = LABEL_FONT;
    ctx.fillStyle = 'black';
    ctx.lineWidth = 1;
    ctx.strokeStyle = 'black';

    const origin = "0";
    const yLabel = "s";
    const yLabel2 = "2s";
    //let ylt = ctx.measureText(yLabel2);

    const xLabel = "x\u0305";   // x bar
    const xLabel2 = "2x\u0305"; // 2 x bar
    //const xlt = ctx.measureText(xLabel2);

    const space = 5;
    const tickLength = 10;

    // Draw Y axis
    ctx.beginPath();
    let x1 = XMARGIN - tickLength - 0.5;
    let x2 = XMARGIN - 0.5;
    let y0 = 0.5;
    let y2 = h - YMARGIN;
    let y1 = y2 / 2;
    y2 += 0.5;
    ctx.moveTo(x1, y0);
    ctx.lineTo(x2, y0);
    ctx.lineTo(x2, y2);
    ctx.lineTo(x1, y2);
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y1);
    ctx.stroke();

    ctx.textAlign = 'right';
    ctx.textBaseline = 'top';
    ctx.fillText(yLabel2, XMARGIN - tickLength - space, 0);
    ctx.textBaseline = 'middle';
    ctx.fillText(yLabel, XMARGIN - tickLength - space, y1);
    ctx.fillText(origin, XMARGIN - tickLength - space, y2);

    // Draw X axis
    ctx.beginPath();
    y1 = y2 + tickLength;
    ctx.moveTo(x1, y2);
    ctx.lineTo(w, y2);
    let x0 = XMARGIN + 0.5;
    x1 = XMARGIN + (w - XMARGIN) / 2;
    x2 = w - 0.5;
    ctx.moveTo(x0, y1);
    ctx.lineTo(x0, y2);
    ctx.moveTo(x1, y1);
    ctx.lineTo(x1, y2);
    ctx.moveTo(x2, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();

    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    y1 += space;
    ctx.fillText(origin, x0, y1);
    ctx.fillText(xLabel, x1, y1);
    ctx.textAlign = 'right';
    ctx.fillText(xLabel2, x2, y1);
  }

  public draw() {
    let w = this.canvas.width;
    let h = this.canvas.height;
    let ctx = this.canvas.getContext('2d');
    ctx.clearRect(0, 0, w, h);
    ctx.putImageData(this.imageData, XMARGIN, 0);
    this.drawAxes(ctx, w, h);

    // Draw the best
    ctx.globalCompositeOperation='difference';
    //ctx.fillStyle='white';
    ctx.lineWidth = 2;
    ctx.strokeStyle = 'white';

    let s = this.getSelection();
    ctx.beginPath();
    // y1 = y2 + tickLength;
    ctx.moveTo(s.cx - 5, s.cy);
    ctx.lineTo(s.cx + 5, s.cy);
    ctx.moveTo(s.cx, s.cy - 5);
    ctx.lineTo(s.cx, s.cy + 5);
    ctx.stroke();
  }
}

class App {
  private sample: number[];
  private ecdf: ECDF;
  private mean: number;
  private stddev: number;
  private ksPlot: KSPlot;
  private cdfGraph: CDFGraph;
  private gamma: HTMLInputElement;

  constructor() {
    let cdf = <HTMLCanvasElement>document.getElementById('cdf')
    resizeCanvasToDisplaySize(cdf);
    let graph = new CDFGraph(cdf);
    cdf.addEventListener('mousemove', function(e) {
       graph.onMouseMove(e);
    }, false);
    cdf.addEventListener('resize', function(e) {
      resizeCanvasToDisplaySize(cdf);
      graph.onResize(e);
    }, false);
    graph.onResize();
    this.cdfGraph = graph;
    this.sample = [];
    this.gamma = <HTMLInputElement>document.getElementById('gamma');
    this.gamma.oninput = (e) => { this.onSlider(e); };
    (<HTMLSelectElement>document.getElementById('distype')).onchange = (e) => { this.onDistType(); };
  }

  private getGamma(): number {
    return Number(this.gamma.value);
  }

  private onDistType(): void {
    if (this.ksPlot == null) {
      return;
    }
    this.ksPlot.setCdfFactory(getCdfFactory());
  }

  private onSlider(e: Event): void {
     if (this.ksPlot == null) {
      return;
     }
    this.ksPlot.setGamma(this.getGamma());
  }

  private onMouseDown(e: MouseEvent): void {
    this.onMouseMove(e);
  }

  private onMouseMove(e: MouseEvent): void {
    if (e.buttons == 0) {
      return;
    }
    // Pick new distribution
    let select = this.ksPlot.setSelection(e);
    const g = select.cdf;

    let txt = document.getElementById('cdf-text');
    txt.innerHTML = g.toHTML() +
      '<br>' +
      'p = ' + select.p;

    // Render it
    this.cdfGraph.layers[1] = new Layer(
      getRenderFuncForFunction(
        (x) => {
          return g.p(x);
        }
      ),
      { // line
        r: 246,
        g: 190,
        b: 0,
        a: 255,
      },
      { // fill
        r: 246,
        g: 190,
        b: 0,
        a: 64,
      });
    this.cdfGraph.draw();
  }

  public start(sample: number[]) {
    this.sample = sample;

    // Build ECDF from the list of samples:
    this.ecdf = new ECDF();
    for (let i = 0; i < sample.length; i++) {
      this.ecdf.addSample(sample[i]);
    };
    this.mean = this.ecdf.mean();
    this.stddev = this.ecdf.stddev(this.mean);

    let txt = document.getElementById('plot-text');
    txt.innerHTML =
      'Sample mean (x&#x0304) = ' + this.mean +
      '<br>' +
      'Sample standard deviation (s) = ' + this.stddev;

    // Build the KS Plot
    let ksCanvas = <HTMLCanvasElement>document.getElementById('plot');
    resizeCanvasToDisplaySize(ksCanvas);
    let ks = new KSPlot(ksCanvas, this.mean, this.stddev, sample, getCdfFactory(), this.getGamma());

    ksCanvas.addEventListener('resize', function(e) {
      resizeCanvasToDisplaySize(ksCanvas);
      ks.onResize();
    }, false);
    this.ksPlot = ks;
    ksCanvas.onmousedown = (e) => this.onMouseDown(e);
    ksCanvas.onmousemove = (e) => this.onMouseMove(e);

    let select = ks.getSelection();
    txt = document.getElementById('cdf-text');
    txt.innerHTML = select.cdf.toHTML() +
      '<br>' +
      'p = ' + select.p;

    // Build the CDF Plot
    let cdf = <HTMLCanvasElement>document.getElementById('cdf')
    resizeCanvasToDisplaySize(cdf);
    let graph = new CDFGraph(cdf);
    cdf.addEventListener('mousemove', function(e) {
       graph.onMouseMove(e);
    }, false);
    cdf.addEventListener('resize', function(e) {
      resizeCanvasToDisplaySize(cdf);
      graph.onResize(e);
    }, false);
    graph.onResize();
    this.cdfGraph = graph;
    this.sample = [];

    // Plot the distributions:
    let p = this.ecdf.getRawCDF();

    // Size the graph
    let xMargin = (p.maxX - p.minX) * 0.1;
    graph.setRangeX(p.minX - xMargin, p.maxX + xMargin);

    // Render this as an image.
    graph.layers[0] = new Layer(
      getRenderFuncForPlot(p),
      { // line
        r: 0,
        g: 0,
        b: 255,
        a: 255
      },
      { // fill
        r: 0,
        g: 0,
        b: 255,
        a: 128,
      });

    let g = select.cdf;
    graph.layers[1] = new Layer(
      getRenderFuncForFunction(
        (x) => {
          return g.p(x);
        }
      ),
      { // line
        r: 246,
        g: 190,
        b: 0,
        a: 255,
      },
      { // fill
        r: 246,
        g: 190,
        b: 0,
        a: 64,
      });
    graph.draw();
  }
}

const app = new App();

function parseData(str: String): number[] {
  let sample = [];
  let words = str.split(/,|\s+/);
  for (let i in words) {
    let x = words[i].trim();
    if (x.length > 0) {
      sample.push(+x);
    }
  }
  return sample;
}

let link = document.getElementById('run-raw-data');
link.onclick = function() {
  let data = <HTMLTextAreaElement>document.getElementById('data');
  app.start(parseData(data.value));
  return false;
};

let queryData = {};


// Init prometheus query dates.
function initQuery() {
  const now = new Date();
  now.setMinutes(now.getMinutes() - now.getTimezoneOffset());
  let start = new Date();
  start.setTime(now.getTime() - 15 * 60 * 1000);
  let i = <HTMLInputElement>document.getElementById('query-start');
  let v = start.toISOString().slice(0,16);
  i.value = v;
  i = <HTMLInputElement>document.getElementById('query-end');
  v = now.toISOString().slice(0,16);
  i.value = v;
}
initQuery();

link = document.getElementById('promrun');
link.onclick = function() {
  const addr = <HTMLInputElement>document.getElementById('promaddr');
  const prom = new PrometheusDriver({
    endpoint: addr.value
  });
  const q = <HTMLTextAreaElement>document.getElementById('query');
  const start = Date.parse(
    (<HTMLInputElement>document.getElementById('query-start')).value);
  const end = Date.parse(
    (<HTMLInputElement>document.getElementById('query-end')).value);
  const step = 15; // seconds
  prom.rangeQuery(q.value, start, end, step)
    .then((res) => {
      const table = <HTMLTableElement>document.getElementById('promresults');
      // Remove any existing rows
      while (table.tBodies.length > 0) {
        table.removeChild(table.tBodies[0]);
      }
      const results = table.createTBody();
      const series = res.result;
      series.forEach((s) => {
        const row = results.insertRow();
        const data: number[] = [];
        s.values.forEach((sv: SampleValue, idx: number) => {
          data.push(sv.value);
        });
        queryData[row.rowIndex] = data;
        let link = document.createElement('a');
        link.href = '#';
        link.text = s.metric.toString();
        link.onclick = (_) => {
          (<HTMLTextAreaElement>document.getElementById("data")).value = data.toString();
          app.start(data);
          return false;
        };
        row.insertCell(-1).append(link);
      });
    })
    .catch(console.error);
  return false;
};
