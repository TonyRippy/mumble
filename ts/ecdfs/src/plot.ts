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

import { type Func, ConstFunc, linearFunction, cubicFunction } from './func'

interface SegmentList {
  x: number
  f: Func
  next: SegmentList | null
}

export function startSegment (): SegmentList {
  return {
    x: Number.NEGATIVE_INFINITY,
    f: new ConstFunc(0),
    next: null
  }
}

export function findFrontTail (x1: number, y1: number, dy1: number): SegmentList {
  const x0 = x1 - 2 * y1 / dy1
  return {
    x: x0,
    f: cubicFunction(x0, 0, 0, x1, y1, dy1),
    next: null
  }
}

export function findBackTail (x1: number, y1: number, dy1: number): SegmentList {
  const x2 = 2 * (1 - y1) / dy1 + x1
  return {
    x: x1,
    f: cubicFunction(x1, y1, dy1, x2, 1, 0),
    next: {
      x: x2,
      f: new ConstFunc(1),
      next: null
    }
  }
}

export interface Plot {
  segments: SegmentList
  minX: number
  maxX: number
}

/*
function emptyPlot (): Plot {
  return {
    segments: startSegment(),
    minX: 0,
    maxX: 1,
    maxY: 1
  }
}
*/

interface RGBA {
  r: number
  g: number
  b: number
  a: number
}

// A function that, given an image mapping, generates min, max, and array of y values.
type RenderFunc = (w: number, cx2fx: Func) => [number, number, number[]]

export function getRenderFuncForPlot (p: Plot): RenderFunc {
  return function (w: number, cx2fx: Func): [number, number, number[]] {
    let minY = Number.POSITIVE_INFINITY
    let maxY = Number.NEGATIVE_INFINITY

    // For each X in the image, find the Y value.
    const ys = new Array<number>(w)
    let s = p.segments
    for (let cx = 0; cx < w; cx++) {
      // What is the value of x in function space?
      const fx = cx2fx.eval(cx)
      // Have we started the next segment?
      if (s.next != null && fx >= s.next.x) {
        s = s.next
      }
      // Calculate the value of y in function space.
      const fy = s.f.eval(fx)
      if (fy < minY) minY = fy
      if (fy > maxY) maxY = fy
      ys[cx] = fy
    }
    return [minY, maxY, ys]
  }
}

export function getRenderFuncForFunction (f: (number) => number): RenderFunc {
  return function (w: number, cx2fx: Func): [number, number, number[]] {
    let minY = Number.POSITIVE_INFINITY
    let maxY = Number.NEGATIVE_INFINITY

    // For each X in the image, find the Y value.
    const ys = new Array<number>(w)
    for (let cx = 0; cx < w; cx++) {
      const fx = cx2fx.eval(cx)
      const fy = f(fx)
      if (fy < minY) minY = fy
      if (fy > maxY) maxY = fy
      ys[cx] = fy
    }
    return [minY, maxY, ys]
  }
}

export class Layer {
  public ys: number[]
  public image: ImageData | null

  constructor (private readonly f: RenderFunc, private readonly lineColor: RGBA, private readonly fillColor: RGBA) {
    this.ys = []
  }

  public scan (w: number, cx2fx: Func): [number, number] {
    // For each X in the image, find the Y value.
    let [min, max, ys] = this.f(w, cx2fx)
    this.ys = ys
    return [min, max]
  }

  public render (w: number, h: number, cx2fx: Func, fy2cy: Func): ImageData {
    // Convert y values to coordinate space
    const ys = this.ys.map(fy => fy2cy.eval(fy))
    // Render the fill
    const pixels = new Uint8ClampedArray(w * h * 4)
    for (let cy = 0; cy < h; cy++) {
      let i = cy * w * 4
      for (let cx = 0; cx < w; cx++) {
        if (ys[cx] <= cy) {
          // color under the graph
          pixels[i] = this.fillColor.r
          i++
          pixels[i] = this.fillColor.g
          i++
          pixels[i] = this.fillColor.b
          i++
          pixels[i] = this.fillColor.a
          i++
        } else {
          // color over the graph
          pixels[i] = 0 // R
          i++
          pixels[i] = 0 // G
          i++
          pixels[i] = 0 // B
          i++
          pixels[i] = 0 // A
          i++
        }
      }
    }
    // Draw the curve itself.
    let lastY = Math.floor(ys[0])
    const limY = h - 1
    for (let cx = 0; cx < w; cx++) {
      // For each x value, find the y values that should be colored in.
      let cy2 = Math.floor(ys[cx])
      let cy1 = lastY
      lastY = cy2
      if (cy1 > cy2) {
        const tmp = cy2
        cy2 = cy1
        cy1 = tmp
      }
      // Make sure the lines are within the bounds of the image
      if (cy2 < 0) continue
      if (cy1 < 0) cy1 = 0
      if (cy1 > limY) continue
      if (cy2 > limY) cy2 = limY
      // Color in the pixels
      for (; cy1 <= cy2; cy1++) {
        let i = (cy1 * w + cx) * 4
        pixels[i] = this.lineColor.r
        i++
        pixels[i] = this.lineColor.g
        i++
        pixels[i] = this.lineColor.b
        i++
        pixels[i] = this.lineColor.a
      }
    }
    return new ImageData(pixels, w, h)
  }
}

export class CDFGraph {
  private cx2fx: Func
  private fx2cx: Func
  private fy2cy: Func
  private cy2fy: Func
  private minX: number
  private maxX: number
  private minY: number
  private maxY: number
  private layers: Layer[]

  constructor (private readonly canvas: HTMLCanvasElement) {
    this.layers = []
    this.setRangeX(0, 1)
  }

  public setRangeX (min: number, max: number): void {
    this.minX = min
    this.maxX = max
    this.minY = 0
    this.onResize()
  }

  public setLayer (i: number, layer: Layer): void {
    this.layers[i] = layer
    layer.image = null
    this.onResize()
  }
  
  public onResize (e?: Event): void {
    const w = this.canvas.width
    this.cx2fx = linearFunction(0, this.minX, w, this.maxX)
    this.fx2cx = linearFunction(this.minX, 0, this.maxX, w)

    // reset all the layers
    if (this.layers.length == 0) {
      this.maxY = 1
    } else {
      for (const layer of this.layers) {
        layer.image = null
        let [minY, maxY] = layer.scan(w, this.cx2fx)
        maxY *= 1.05
        if (maxY > this.maxY) this.maxY = maxY
      }
    }
    const h = this.canvas.height
    this.fy2cy = linearFunction(this.minY, h - 1, this.maxY, 0)
    this.cy2fy = linearFunction(h - 1, this.minY, 0, this.maxY)

    // force a redraw
    this.draw()
  }

  public draw (): void {
    const w = this.canvas.width
    const h = this.canvas.height
    const ctx = this.canvas.getContext('2d')
    if (ctx == null) throw new Error('Unable to create 2d context!')
    ctx.clearRect(0, 0, w, h)
    for (let i = 0; i < this.layers.length; i++) {
      const layer = this.layers[i]
      if (layer.image == null) {
        layer.image = layer.render(w, h, this.cx2fx, this.fy2cy)
      }
      if (+i === 0) {
        ctx.putImageData(layer.image, 0, 0)
      } else {
        // create a temporary canvas to hold the overlay
        const canvas2 = document.createElement('canvas')
        canvas2.width = w
        canvas2.height = h
        const ctx2 = canvas2.getContext('2d')
        if (ctx2 == null) throw new Error('Unable to create 2d context!')
        ctx2.putImageData(layer.image, 0, 0)
        ctx.drawImage(canvas2, 0, 0)
      }
    }
  }
}

export function resizeCanvasToDisplaySize (canvas: HTMLCanvasElement): boolean {
  // Lookup the size the browser is displaying the canvas in CSS pixels.
  const dpr = window.devicePixelRatio
  const displayWidth = Math.floor(canvas.clientWidth * dpr)
  const displayHeight = Math.floor(canvas.clientHeight * dpr)

  // Check if the canvas is the same size as before:
  if (canvas.width === displayWidth && canvas.height === displayHeight) {
    // Nothing to do!
    return false
  }

  // Otherwise, adjust the canvas:
  canvas.width = displayWidth
  canvas.height = displayHeight
  return true
}

export function createGraph (id: string): CDFGraph {
  const cdf = document.getElementById(id) as HTMLCanvasElement
  resizeCanvasToDisplaySize(cdf)
  const graph = new CDFGraph(cdf)
  cdf.addEventListener('resize', function (e) {
    if (resizeCanvasToDisplaySize(cdf)) {
      graph.onResize(e)
    }
  }, false)
  graph.onResize()
  return graph
}
