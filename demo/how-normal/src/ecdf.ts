import { Point } from "./point";

// Problem case
// x = [1, 1.5, 1.6, 2]
// h = [1, 1,   1,   3]

export interface Func {
  eval(x: number): number
  deriv(): Func
}

class ConstFunc implements Func {
  value: number

  constructor(value: number) {
    this.value = value;
  }
  
  eval(x: number): number {
    return this.value;
  }

  deriv(): Func {
    return new ConstFunc(0);
  }
}

class PolyFunc implements Func {
  coef: number[]

  constructor(coef: number[]) {
    if (coef.length == 0) {
      throw "Can't create an empty polynomial.";
    }
    this.coef = coef;
  }
  
  eval(x: number): number {
    let v = this.coef[0];
    let n = this.coef.length;
    for (let i = 1; i < n; i++) {
      v *= x;
      v += this.coef[i];
    }
    return v;
  }

  deriv(): Func {
    let n = this.coef.length - 1;
    if (n == 0) {
      return new ConstFunc(0);
    }
    let newcoef = new Array<number>(n);
    for (let i = 0; i < n; i++) {
      newcoef[i] = (n-i) * this.coef[i];
    }
    return new PolyFunc(newcoef);
  }
}

export function linearFunction(x1: number, y1: number, x2: number, y2: number): Func {
	let M = (y2 - y1) / (x2 - x1);
	let B = y1 - M * x1;
  return new PolyFunc([M, B]);
}

function solveCubic(x1: number, y1: number, dy1: number,
                    x2: number, y2: number, dy2: number): number[] {
	let dx = x2 - x1;
	let xx1 = x1 * x1;
	let xx2 = x2 * x2;

	let a1 = (xx2 * x2) - (xx1 * x1);
	let a2 = a1;
	a1 -= 3 * xx1 * dx;
	a2 -= 3 * xx2 * dx;

	let b1 = xx2 - xx1;
	let b2 = b1;
	b1 -= (2 * x1 * dx);
	b2 -= (2 * x2 * dx);

	let e1 = y2 - y1;
	let e2 = e1;
	e1 -= dy1 * dx;
	e2 -= dy2 * dx;

	let f = b1 / b2;

	let A = (e1 - f*e2) / (a1 - f*a2);
	let B = (e1 - A*a1) / b1;
	let C = dy1 - (3 * A * xx1) - (2 * B * x1);
	let D = y1 - (A * xx1 * x1) - (B * xx1) - (C * x1);
  
  return [A, B, C, D];
}

function cubicFunction(x1: number, y1: number, dy1: number,
                       x2: number, y2: number, dy2: number): PolyFunc {
  return new PolyFunc(
    solveCubic(x1, y1, dy1, x2, y2, dy2));
}

/*
FritschCarlsonTangents calculates tangents for a set of points
that ensure monotonicity for a resulting Hermite spline.

This function makes some important assumptions:
  1. That the input arrays have the same length.
  2. That the data points are monotonic.
  3. That the input points are sorted on the x axis, ascending.
These assumptions are not verified by the method.
*/
function FritschCarlsonTangents(xs: number[], ys: number[]): number[] {
	// For implementation details, see:
	// https://en.wikipedia.org/wiki/Monotone_cubic_interpolation
	let n = xs.length;
	if (n == 0) {
		return [];
	}
	if (n == 1) {
		return [ys[0]];
	}
	// Compute the slopes of the secant lines between successive points
	let d = new Array<number>(n-1);
	for (let i = 0; i < n-1; i++) {
		d[i] = (ys[i+1] - ys[i]) / (xs[i+1] - xs[i]);
	}
	// Compute provisional tangents
	let m = new Array<number>(n);
	m[0] = d[0];
	m[n-1] = d[n-2];
	for (let i = 1; i < n-1; i++) {
		if (d[i] == 0.0) {
			m[i] = 0.0;
			i += 1;
			m[i] = 0.0;
			continue;
		}
		if (Math.sign(d[i-1]) != Math.sign(d[i])) {
			m[i] = 0.0;
		} else {
			m[i] = (d[i-1] + d[i]) / 2;
		}
	}
	// Adjust tangents to keep monoticity.
	for (let i = 0; i < n-1; i++) {
		let dk = d[i];
		let ak = m[i] / dk;
		let bk = m[i+1] / dk;
		let sqsum = ak*ak + bk*bk;
		if (sqsum > 9.0) {
			let tk = 3.0 / Math.sqrt(sqsum);
			m[i] = tk * ak * dk;
			m[i+1] = tk * bk * dk;
		}
	}
	return m;
}

class FrontExpFunc implements Func {
  x: number
  m: number

  constructor(x: number, m: number) {
    this.x = x;
    this.m = m;
  }
  
  eval(x: number): number {
    return Math.exp(x - this.x) * this.m;
  }

  deriv(): Func {
    return this;
  }
}

class BackExpFunc implements Func {
  x: number
  m: number

  constructor(x: number, m: number) {
    this.x = x;
    this.m = m;
  }
  
  eval(x: number): number {
    return 1 - Math.exp(this.x - x) * this.m;
  }

  deriv(): Func {
    return new BackExpDerivFunc(this.x, this.m);
  }
}

class BackExpDerivFunc implements Func {
  x: number
  m: number

  constructor(x: number, m: number) {
    this.x = x;
    this.m = m;
  }
  
  eval(x: number): number {
    return Math.exp(this.x - x) * this.m;
  }

  deriv(): Func {
    return new BackExpDerivFunc(this.x, -this.m);
  }
}

interface SegmentList {
  x: number
  f: Func
  next: SegmentList
}

function startSegment(): SegmentList {
  return {
    x: Number.NEGATIVE_INFINITY,
    f: new ConstFunc(0),
    next: null
  };
}

function findFrontTail(x1: number, y1: number, dy1: number): SegmentList {
  let x0 = x1 - 2 * y1 / dy1;
  return {
    x: x0,
    f: cubicFunction(x0, 0, 0, x1, y1, dy1),
    next : null
  };
}

function findBackTail(x1: number, y1: number, dy1: number): SegmentList {
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

interface Plot {
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

export class ECDF {
  x: number[];
  h: number[];
  n: number;
  
  constructor() {
    this.x = [];
    this.h = [];
    this.n = 0;
  }

  public mean(): number {
    let sum = 0;
    let count = 0;
    for (let i = 0; i < this.n; i++) {
      sum += this.x[i] * this.h[i];
      count += this.h[i];
    }
    return sum / count;
  }

  public stddev(mean: number): number {
    let sum = 0;
    let count = 0;
    for (let i = 0; i < this.n; i++) {
      let err = this.x[i] - mean;
      sum += err * err * this.h[i];
      count += this.h[i];
    }
    return Math.sqrt(sum / (count - 1));
  }
  
  binarySearch(v: number, min: number, max: number): number {
    if (min >= max) {
      return min;
    }
    let m = (min + max) >> 1;
    let mv = this.x[m];
    if (v == mv) {
      return -(m+1);
    }
    if (v < mv) {
      return this.binarySearch(v, min, m);
    }
    if (m == min) {
      return max;
    }
    return this.binarySearch(v, m, max);
  }
  
  addSample(v: number) {
    let i = this.binarySearch(v, 0, this.n);
    if (i < 0) {
      // We found a match. Rather than inserting, we'll just increase the count.
      this.h[-i-1] += 1;
      return;
    }
    // Insert a new sample at the correct position in the arrays.
    this.x.splice(i, 0, v);
    this.h.splice(i, 0, 1);
    this.n += 1;
  }

  public getRawCDF(): Plot {
    let root = startSegment();
    let s = root;
    let n = 0;
    for (let i = 0; i < this.n; i++) {
      n += this.h[i];
    }
    let h = 0;
    for (let i = 0; i < this.n; i++) {
      let xx = this.x[i];
      h += this.h[i];
      let hh = h / n;
      s.next = {x: xx, f: new ConstFunc(hh), next: null};
      s = s.next;
    }
    let minx = this.n == 0 ? 0 : this.x[0];
    let maxx = minx;
    if (this.n <= 1) {
      maxx += 2;
      minx -= 2;
    } else {
      maxx = this.x[this.n-1];
      let margin  = (maxx - minx) * 0.15;
      minx -= margin;
      maxx += margin;
    }
    return {
      segments: root,
      minX: minx,
      maxX: maxx,
      maxY: 1
    };
  }
  
  public getLinearCDF(): Plot {
    let root = startSegment();
    if (this.n == 0) {
      return {
        segments: root,
        minX: 0,
        maxX: 1,
        maxY: 1
      };
    }
    let s = root;
    let n = 0;
    for (let i = 0; i < this.n; i++) {
      n += this.h[i];
    }
    let lx = this.x[0] - 2;
    let ly = 0;
    let h = 0;
    for (let i = 0; i < this.n; i++) {
      let xx = this.x[i];
      h += this.h[i];
      let yy = h / (n+1);
      s.next = {
        x: lx,
        f: linearFunction(lx, ly, xx, yy),
        next: null
      };
      s = s.next;
      lx = xx;
      ly = yy;
    }
    s.next = {
      x: lx,
      f: linearFunction(lx, ly, lx + 2, 1),
      next: {
        x: lx + 2,
        f: new ConstFunc(1),
        next: null
      }
    };
    return {
      segments: root,
      minX: this.x[0] - 2,
      maxX: lx + 2,
      maxY: 1
    };
  }
  
  public getCubicCDF(): Plot {
    if (this.n < 2) {
      return this.getLinearCDF();
    }
    let n = 0;
    for (let i = 0; i < this.n; i++) {
      n += this.h[i];
    }
    let ys = new Array<number>(this.x.length);
    let h = 0;
    for (let i = 0; i < this.n; i++) {
      h += this.h[i];
      ys[i] = h / (n+1);
    }
    let dys = FritschCarlsonTangents(this.x, ys);
    let root = startSegment();
    let s = root;
    s.next = findFrontTail(this.x[0], ys[0], dys[0]);
    s = s.next;
    let frontX = s.x;
    let i = 0;
    for (; i < this.n - 1; i++) {
      s.next = {
        x: this.x[i],
        f: cubicFunction(this.x[i], ys[i], dys[i],
                         this.x[i+1], ys[i+1], dys[i+1]),
        next: null
      };
      s = s.next;
    }
    s.next = findBackTail(this.x[i], ys[i], dys[i]);
    let backX = s.next.next.x;
    return {
      segments: root,
      minX: frontX,
      maxX: backX,
      maxY: 1
    };
  }
  
  // // Caluclates the point where the linear interpolation hits zero.
  // getMin(): number {
  //   /*
  //   let n = this.n + 1;
  //   if (n <= 1) {
  //     return 0;
  //   }
  //   let t = 1 / n;
  //   let y = this.h[0] / n;
  //   let d = 2 * t / y;
  //   return this.x[0] - d;
  //   */
  // }

  // // Caluclates the point where the linear interpolation hits one.
  // getMax(): number {
  //   let n = this.n - 1;
  //   if (n <= 1) {
  //     return 0;
  //   }
  //   let t = 1 / n;
  //   let y = this.h[0] / n;
  //   let d = 2 * t / y;
  //   return this.x[0] - d;
  // }
}

var sample = new ECDF();

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

    if (this.layers[0]) {
      return;
    }
    // console.log('rect - x: ' + rect.left + ', y: ' + rect.top + ', w: ' + rect.width + ', h: ' + rect.height);
    // console.log('client - x: ' + e.clientX + ', y: ' + e.clientY);
    // console.log('offset - x: ' + e.offsetX + ', y: ' + e.offsetY);
    
    // console.log('width: ' + this.canvas.width + ', client: ' + rect);
    // console.log('offset - x: ' + x + ', y: ' + y);
    let fx = this.cx2fx.eval(cx);
    let fy = this.cy2fy.eval(cy);
    
    // console.log('coord - x: ' +  + ', y: ' + this.cy2fy.eval(y));

    //document.getElementById('x').innerHTML = "X: " + this.cx2fx.eval(e.offsetX);
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
