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

// Adapted from:
// https://hewgill.com/picomath/javascript/erf.js.html
function erf (x: number): number {
  // constants
  const a1 = 0.254829592
  const a2 = -0.284496736
  const a3 = 1.421413741
  const a4 = -1.453152027
  const a5 = 1.061405429
  const p = 0.3275911

  // Save the sign of x
  let sign = 1
  if (x < 0) {
    sign = -1
  }
  x = Math.abs(x)

  // A&S formula 7.1.26
  const t = 1.0 / (1.0 + p * x)
  const y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-x * x)

  return sign * y
}

const INV_SQRT_2PI = 1.0 / Math.sqrt(Math.PI + Math.PI)

// The functions for the standard normal distribution.
// (μ=0 and σ=1)
function pdf (x: number): number {
  return Math.exp(-0.5 * x * x) * INV_SQRT_2PI
}

function cdf (x: number): number {
  return 0.5 * (1.0 + erf(x / Math.SQRT2))
}

// See: https://en.wikipedia.org/wiki/Normal_distribution
class _Normal implements CDF {
  constructor (public mean: number, public stddev: number) {
  }

  public p (x: number): number {
    return cdf((x - this.mean) / this.stddev)
  }

  public dx (x: number): number {
    return pdf((x - this.mean) / this.stddev) / this.stddev
  }

  public toHTML (): string {
    return 'Normal(&mu; = ' + String(this.mean) + ', &sigma; = ' + String(this.stddev) + ')'
  }
}

export function Normal (mean: number, stddev: number): CDF {
  return new _Normal(mean, stddev)
}

// See: https://en.wikipedia.org/wiki/Log-normal_distribution
class _LogNormal implements CDF {
  private readonly mean: number
  private readonly stddev: number

  constructor (sampleMean: number, sampleStddev: number) {
    const m2 = sampleMean * sampleMean
    const s2 = sampleStddev * sampleStddev
    this.mean = Math.log(m2 / Math.sqrt(m2 + s2))
    this.stddev = Math.sqrt(Math.log(1 + s2 / m2))
  }

  private x (x: number): number {
    return (Math.log(x) - this.mean) / this.stddev
  }

  public p (x: number): number {
    return cdf(this.x(x))
  }

  public dx (x: number): number {
    return pdf(this.x(x) / (this.stddev * x))
  }

  public toHTML (): string {
    return 'LogNormal(&mu; = ' + String(this.mean) + ', &sigma; = ' + String(this.stddev) + ')'
  }
}

// A special case fo handling case where mean is zero.
class _LogZero implements CDF {
  public p (x: number): number {
    return 0
  }

  public dx (x: number): number {
    return 0
  }

  public toHTML (): string {
    return 'LogNormal(&mu; = &infin;, &sigma; = -&infin;)'
  }
}

export function LogNormal (mean: number, stddev: number): CDF {
  if (mean <= 0) {
    return new _LogZero()
  }
  return new _LogNormal(mean, stddev)
}
