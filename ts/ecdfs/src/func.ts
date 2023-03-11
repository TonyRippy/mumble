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

export interface Func {
  eval(x: number): number
  deriv(): Func
}

export class ConstFunc implements Func {
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

export class PolyFunc implements Func {
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

export function cubicFunction(x1: number, y1: number, dy1: number,
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
export function FritschCarlsonTangents(xs: number[], ys: number[]): number[] {
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

