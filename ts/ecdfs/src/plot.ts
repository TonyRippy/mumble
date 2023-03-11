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

import { Func, ConstFunc, linearFunction, cubicFunction } from "./func";

interface SegmentList {
  x: number
  f: Func
  next: SegmentList
}

export function startSegment(): SegmentList {
  return {
    x: Number.NEGATIVE_INFINITY,
    f: new ConstFunc(0),
    next: null
  };
}

export function findFrontTail(x1: number, y1: number, dy1: number): SegmentList {
  let x0 = x1 - 2 * y1 / dy1;
  return {
    x: x0,
    f: cubicFunction(x0, 0, 0, x1, y1, dy1),
    next : null
  };
}

export function findBackTail(x1: number, y1: number, dy1: number): SegmentList {
  let x2 = 2 * (1 - y1) / dy1 + x1;
  return {
    x: x1,
    f: cubicFunction(x1, y1, dy1, x2, 1, 0),
    next : {
      x: x2,
      f: new ConstFunc(1),
      next: null
    }
  };
}

export interface Plot {
  segments: SegmentList
  minX: number
  maxX: number
  maxY: number
}

function emptyPlot(): Plot {
  return {
    segments: startSegment(),
    minX: 0,
    maxX: 1,
    maxY: 1
  };
}

/*
function fillPoint(ctx: CanvasRenderingContext2D, p: Point): void {
  ctx.beginPath();
  ctx.arc(p.x, p.y, 2, 0, 2 * Math.PI);
  ctx.fill();
}
*/

interface RGBA {
  r: number
  g: number
  b: number
  a: number
}

/*

segments --> func

func --> render to y array

Layer = (func, line color, fill color) -> image 

[

  
] 
*/

interface RenderFunc {
  (w: number, cx2fx: Func, fy2cy: Func): number[];
}

export function getRenderFuncForPlot(p: Plot): RenderFunc {
  return function(w: number, cx2fx: Func, fy2cy: Func): number[] {
    // For each X in the image, find the Y value.
    let ys = new Array<number>(w);    
    let s = p.segments;
    for (let cx = 0; cx < w; cx++) {
      // What is the value of x in function space?
      let fx = cx2fx.eval(cx);
      // Have we started the next segment?
      if (s.next != null && fx >= s.next.x) {
        s = s.next;
      }
      // Calculate the value of y in function space.
      let fy = s.f.eval(fx);
      ys[cx] = fy2cy.eval(fy);
    }
    return ys;
  }
}

export function getRenderFuncForFunction(f: Function): RenderFunc {
  return function(w: number, cx2fx: Func, fy2cy: Func): number[] {
    let ys = new Array<number>(w);    
    for (let cx = 0; cx < w; cx++) {
      const fx = cx2fx.eval(cx);
      const fy = f(fx);
      const cy = fy2cy.eval(fy);
      ys[cx] = cy;
    }
    return ys;
  }
}
  

export class Layer {
  public ys: number[];
  public image: ImageData;
  
  constructor(private f: RenderFunc, private lineColor: RGBA, private fillColor: RGBA) {
  }
  
  public render(w: number, h: number, cx2fx: Func, fy2cy: Func): ImageData {
    // For each X in the image, find the Y value.
    this.ys = this.f(w, cx2fx, fy2cy);

    // Render the fill
    let pixels = new Uint8ClampedArray(w * h * 4);
    for (let cy = 0; cy < h; cy++) {
      let i = cy * w * 4;
      for (let cx = 0; cx < w; cx++) {
        if (this.ys[cx] <= cy) {
          // color under the graph
          pixels[i] = this.fillColor.r;
          i++;
          pixels[i] = this.fillColor.g;
          i++;
          pixels[i] = this.fillColor.b;
          i++;
          pixels[i] = this.fillColor.a;
          i++;
        } else {
          // color over the graph
          pixels[i] = 0; // R
          i++;
          pixels[i] = 0; // G
          i++;
          pixels[i] = 0; // B
          i++;
          pixels[i] = 0; // A
          i++;
        }
      }
    }
    // Draw the curve itself.
    let lastY = Math.floor(this.ys[0]);
    for (let cx = 0; cx < w; cx++) {
      let cy = Math.floor(this.ys[cx]);
      let maxY = lastY;
      lastY = cy;
      if (cy > maxY) {
        let tmp = maxY;
        maxY = cy;
        cy = tmp;
      }
      for (; cy <= maxY; cy++) {
        let i = (cy * w + cx) * 4;
        pixels[i] = this.lineColor.r;
        i++;
        pixels[i] = this.lineColor.g;
        i++;
        pixels[i] = this.lineColor.b;
        i++;
        pixels[i] = this.lineColor.a;
      }
    }
    return new ImageData(pixels, w, h);
  }
}

export class CDFGraph {
  private cx2fx: Func;
  private fx2cx: Func;
  private fy2cy: Func;
  private cy2fy: Func;
  private minX: number;
  private maxX: number;
  public layers: Layer[];
  
  constructor(private canvas: HTMLCanvasElement) {
    this.layers = [];
    this.setRangeX(0,1);
  }

  public setRangeX(min: number, max: number) {
    this.minX = min;
    this.maxX = max;
    this.onResize();
  }
  
  public onResize(e?: Event) {
    let w = this.canvas.width;
    this.cx2fx = linearFunction(0, this.minX, w, this.maxX);
    this.fx2cx = linearFunction(this.minX, 0, this.maxX, w);

    const minY = 0;
    const maxY = 1.1; 
    let h = this.canvas.height;
    this.fy2cy = linearFunction(minY, h - 1, maxY, 0);
    this.cy2fy = linearFunction(h - 1, minY, 0, maxY);

    // reset all the images
    for (let i in this.layers) {
      this.layers[i].image = null;
    }

    // force a redraw
    this.draw();
  }

  // This is a hack
  /*
  public drawDiscontinuous(p: Plot) {
    let w = this.canvas.width;
    let h = this.canvas.height;
    let ctx = this.canvas.getContext('2d');
    ctx.clearRect(0, 0, w, h);
    this.ys = null;

    let deltas = new Array<Point>(sample.n);
    let maxDelta = 0;
    let y = 0;
    let i = 0;
    for (let i = 0, s = p.segments.next; s != null; s = s.next, i++) {
      let yy = s.f.eval(s.x);
      let delta = yy - y;
      if (delta > maxDelta) {
        maxDelta = delta;
      }
      deltas[i] = {
        x: s.x,
        y: delta
      };
      y = yy;
    }
    maxDelta *= 1.1;  // y margin
    let xMargin = (p.maxX - p.minX) * 0.1;

    // Create functions to map canvas coodrinates to function space.
    this.cx2fx = linearFunction(0, p.minX - xMargin, w, p.maxX + xMargin);
    let fx2cx = linearFunction(p.minX - xMargin, 0, p.maxX + xMargin, w);
    this.fy2cy = linearFunction(0, h - 1, maxDelta, 0);
    this.cy2fy = linearFunction(h - 1, 0, 0, maxDelta);

    // Draw the spikes
    ctx.strokeStyle = 'blue';
    ctx.beginPath();
    ctx.moveTo(0, h-1);
    ctx.lineTo(w, h-1);
    ctx.stroke();
    for (let i = 0; i < sample.n; i++) {
      let p = deltas[i];
      let x = fx2cx.eval(p.x);
      let y = this.fy2cy.eval(p.y);
      ctx.beginPath();
      ctx.moveTo(x, h-1);
      ctx.lineTo(x, y);
      ctx.stroke();
    }
  }
  */

  public draw() {
    let w = this.canvas.width;
    let h = this.canvas.height;
    let ctx = this.canvas.getContext('2d');
    ctx.clearRect(0, 0, w, h);
    for (let i in this.layers) {
      let layer = this.layers[i];
      if (!layer.image) {
        layer.image = layer.render(w, h, this.cx2fx, this.fy2cy);
      }
      if (+i == 0) {
        ctx.putImageData(layer.image, 0, 0);
      } else {
        // create a temporary canvas to hold the overlay
        var canvas2 = document.createElement("canvas");
        canvas2.width = w;
        canvas2.height = h;
        var ctx2 = canvas2.getContext("2d");
        ctx2.putImageData(layer.image,0,0);
        ctx.drawImage(canvas2,0,0);
      }
    }
  }
  
  public onMouseMove(e: MouseEvent) {
    var rect = this.canvas.getBoundingClientRect();  // get element's abs. position
    var cx = e.offsetX;              // get mouse x and adjust for el.
    var cy = e.offsetY;               // get mouse y and adjust for el.

    // console.log('rect - x: ' + rect.left + ', y: ' + rect.top + ', w: ' + rect.width + ', h: ' + rect.height);
    // console.log('client - x: ' + e.clientX + ', y: ' + e.clientY);
    // console.log('offset - x: ' + e.offsetX + ', y: ' + e.offsetY);
    
    // console.log('width: ' + this.canvas.width + ', client: ' + rect);
    // console.log('offset - x: ' + x + ', y: ' + y);
    let fx = this.cx2fx.eval(cx);
    let fy = this.cy2fy.eval(cy);
    
    // console.log('coord - x: ' +  + ', y: ' + this.cy2fy.eval(y));

    document.getElementById('mouse-text').innerText =
      "X: " + fx;
      //document.getElementById('y').innerHTML = "Y: " + this.cy2fy.eval(e.offsetY);

    // if (this.ecdfImage) {
    //   let w = this.canvas.width;
    //   let h = this.canvas.height;
    //   let ctx = this.canvas.getContext('2d');
    //   ctx.putImageData(this.ecdfImage, 0, 0);
    //   ctx.strokeStyle = 'yellow';
    //   let cx = x;
    //   let cy = this.ys[x];
    //   ctx.beginPath();
    //   ctx.moveTo(cx, h-1);
    //   ctx.lineTo(cx, cy);
    //   ctx.stroke();
    //   ctx.fillStyle = 'yellow';
    //   ctx.beginPath();
    //   ctx.arc(cx, cy, 2, 0, 2 * Math.PI);
    //   ctx.fill();
    // }
  }
}
