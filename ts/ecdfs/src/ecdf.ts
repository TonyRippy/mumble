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

import type { CDF } from './cdf'
import { ConstFunc, linearFunction, cubicFunction, PolyFunc, FritschCarlsonTangents } from './func'
import { type Plot, startSegment, findFrontTail, findBackTail } from './plot'

export class ECDF implements CDF {
  x: number[]
  h: number[]
  n: number

  constructor () {
    this.x = []
    this.h = []
    this.n = 0
  }

  public min (): number {
    if (this.n > 0) {
      return this.x[0]
    } else {
      return 0.0
    }
  }

  public max (): number {
    if (this.n > 0) {
      return this.x[this.n - 1]
    } else {
      return 0.0
    }
  }

  public mean (): number {
    let sum = 0
    let count = 0
    for (let i = 0; i < this.n; i++) {
      sum += this.x[i] * this.h[i]
      count += this.h[i]
    }
    return sum / count
  }

  public stddev (mean: number): number {
    let sum = 0
    let count = 0
    for (let i = 0; i < this.n; i++) {
      const err = this.x[i] - mean
      sum += err * err * this.h[i]
      count += this.h[i]
    }
    return Math.sqrt(sum / (count - 1))
  }

  public p (x: number): number {
    let i = 0
    let count = 0
    for (; i < this.n; i++) {
      if (this.x[i] > x) {
        break
      }
      count += this.h[i]
    }
    let total = count
    for (; i < this.n; i++) {
      total += this.h[i]
    }
    return count / total
  }

  public dx (x: number): number {
    let i = this.binarySearch(x, 0, this.n)
    if (i < 0) {
      return this.h[-i - 1]
    }
    i--
    if (i < 0) {
      return 0
    }
    return this.h[i]
  }

  public toHTML (): string {
    return ''
  }

  binarySearch (v: number, min: number, max: number): number {
    if (min >= max) {
      return min
    }
    const m = (min + max) >> 1
    const mv = this.x[m]
    if (v === mv) {
      return -(m + 1)
    }
    if (v < mv) {
      return this.binarySearch(v, min, m)
    }
    if (m === min) {
      return max
    }
    return this.binarySearch(v, m, max)
  }

  public addSample (v: number): void {
    const i = this.binarySearch(v, 0, this.n)
    if (i < 0) {
      // We found a match. Rather than inserting, we'll just increase the count.
      this.h[-i - 1] += 1
      return
    }
    // Insert a new sample at the correct position in the arrays.
    this.x.splice(i, 0, v)
    this.h.splice(i, 0, 1)
    this.n += 1
  }

  public merge (other: ECDF): void {
    if (other.n === 0) return
    const x = new Array<number>()
    const h = new Array<number>()
    let si = 0
    let oi = 0
    while (si < this.n && oi < other.n) {
      const sx = this.x[si]
      const ox = other.x[oi]
      if (sx < ox) {
        x.push(sx)
        h.push(this.h[si])
        si += 1
      } else if (sx > ox) {
        x.push(ox)
        h.push(other.h[oi])
        oi += 1
      } else {
        x.push(sx)
        h.push(this.h[si] + other.h[oi])
        si += 1
        oi += 1
      }
    }
    while (si < this.n) {
      x.push(this.x[si])
      h.push(this.h[si])
      si += 1
    }
    while (oi < other.n) {
      x.push(other.x[oi])
      h.push(other.h[oi])
      oi += 1
    }
    this.x = x
    this.h = h
    this.n = x.length
  }

  public getRawCDF (): Plot {
    const root = startSegment()
    let s = root
    let n = 0
    for (let i = 0; i < this.n; i++) {
      n += this.h[i]
    }
    let h = 0
    for (let i = 0; i < this.n; i++) {
      const xx = this.x[i]
      h += this.h[i]
      const hh = h / n
      s.next = { x: xx, f: new ConstFunc(hh), next: null }
      s = s.next
    }
    let minx = this.n === 0 ? 0 : this.x[0]
    let maxx = minx
    if (this.n <= 1) {
      maxx += 2
      minx -= 2
    } else {
      maxx = this.x[this.n - 1]
      const margin = (maxx - minx) * 0.15
      minx -= margin
      maxx += margin
    }
    return {
      segments: root,
      minX: minx,
      maxX: maxx
    }
  }

  public getLinearCDF (): Plot {
    const root = startSegment()
    if (this.n === 0) {
      return {
        segments: root,
        minX: 0,
        maxX: 1
      }
    }
    let s = root

    // Find the total number of samples.
    let n = 0
    for (let i = 0; i < this.n; i++) {
      n += this.h[i]
    }

    // Project the slope of the first segment backwards to find where it meets zero.
    // This segment will go from (x=?,y=0) to x[1].
    let lx = this.x[1]
    let h = this.h[0] + this.h[1]
    let ly = h / n
    let m = (this.h[1] / n) / (this.x[1] - this.x[0])
    var firstX;
    if (m === 0) {
      // No slope, just start at x[0].
      firstX = this.x[0]
      s.next = {
        x: firstX,
        f: new ConstFunc(ly),
        next: null
      }
    } else {
      let b = ly - m * lx
      firstX = -b / m
      s.next = {
        x: firstX,
        f: new PolyFunc([m, b]),
        next: null
      }
    }
    s = s.next

    // Now find the line segments for the rest of the points.
    for (let i = 2; i < this.n; i++) {
      const xx = this.x[i]
      h += this.h[i]
      const yy = h / n
      s.next = {
        x: lx,
        f: linearFunction(lx, ly, xx, yy),
        next: null
      }
      s = s.next
      lx = xx
      ly = yy
    }

    // End with a horizontal line at y=1.
    s.next = {
      x: lx,
      f: new ConstFunc(1),
      next: null
    }
    return {
      segments: root,
      minX: firstX,
      maxX: lx
    }
  }

  public getCubicCDF (): Plot {
    if (this.n < 2) {
      return this.getLinearCDF()
    }
    let n = 0
    for (let i = 0; i < this.n; i++) {
      n += this.h[i]
    }
    const ys = new Array<number>(this.x.length)
    let h = 0
    for (let i = 0; i < this.n; i++) {
      h += this.h[i]
      ys[i] = h / (n + 1)
    }
    const dys = FritschCarlsonTangents(this.x, ys)
    const root = startSegment()
    let s = root
    s.next = findFrontTail(this.x[0], ys[0], dys[0])
    s = s.next
    const frontX = s.x
    let i = 0
    for (; i < this.n - 1; i++) {
      s.next = {
        x: this.x[i],
        f: cubicFunction(this.x[i], ys[i], dys[i],
          this.x[i + 1], ys[i + 1], dys[i + 1]),
        next: null
      }
      s = s.next
    }
    s.next = findBackTail(this.x[i], ys[i], dys[i])
    if (s.next != null) {
      s = s.next
    }
    if (s.next != null) {
      s = s.next
    }
    const backX = s.x
    return {
      segments: root,
      minX: frontX,
      maxX: backX
    }
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

export function fromJSON (json: Array<[number, number]>): ECDF {
  const x = new Array<number>(json.length)
  const h = new Array<number>(json.length)
  for (let i = 0; i < json.length; i++) {
    const sample = json[i]
    x[i] = sample[0]
    h[i] = sample[1]
  }
  const out = new ECDF()
  out.x = x
  out.h = h
  out.n = json.length
  return out
}

export function toJSON (ecdf: ECDF): Array<[number, number]> {
  const out = new Array<[number, number]>(ecdf.n)
  for (let i = 0; i < ecdf.n; i++) {
    out[i] = [ecdf.x[i], ecdf.h[i]]
  }
  return out
}
