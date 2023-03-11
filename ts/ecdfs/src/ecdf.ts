import { ConstFunc, linearFunction, cubicFunction, FritschCarlsonTangents }  from "./func";
import { Plot, startSegment, findFrontTail, findBackTail } from "./plot";

// Problem case
// x = [1, 1.5, 1.6, 2]
// h = [1, 1,   1,   3]

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
  
  public addSample(v: number) {
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

  public addJSON(json: Array<[number, number]>) {
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
