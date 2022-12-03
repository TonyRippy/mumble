import { CDF } from "./cdf";

// https://hewgill.com/picomath/javascript/erf.js.html
function erf(x) {
    // constants
    var a1 =  0.254829592;
    var a2 = -0.284496736;
    var a3 =  1.421413741;
    var a4 = -1.453152027;
    var a5 =  1.061405429;
    var p  =  0.3275911;

    // Save the sign of x
    var sign = 1;
    if (x < 0) {
        sign = -1;
    }
    x = Math.abs(x);

    // A&S formula 7.1.26
    var t = 1.0/(1.0 + p*x);
    var y = 1.0 - (((((a5*t + a4)*t) + a3)*t + a2)*t + a1)*t*Math.exp(-x*x);

    return sign*y;
}

const INV_SQRT_2PI = 1.0 / Math.sqrt(Math.PI + Math.PI);

// The functions for the standard normal distribution.
// (μ=0 and σ=1)
function pdf(x: number): number {
	return Math.exp(-0.5*x*x) * INV_SQRT_2PI;
}

function cdf(x: number): number {
	return 0.5 * (1.0 + erf(x / Math.SQRT2));
}

// See: https://en.wikipedia.org/wiki/Normal_distribution
class _Normal implements CDF {
  constructor(public mean: number, public stddev: number) {
  }

  public p(x: number): number {
	  return cdf((x - this.mean) / this.stddev)
  }

  public dx(x: number): number {
	  return pdf((x - this.mean) / this.stddev) / this.stddev;
  }

  public toHTML(): string {
    return 'Normal(&mu; = ' + this.mean + ', &sigma; = ' + this.stddev + ')';
  }
}

export function Normal(mean: number, stddev: number): CDF {
  return new _Normal(mean, stddev);
}

// See: https://en.wikipedia.org/wiki/Log-normal_distribution
class _LogNormal implements CDF {
  private mean: number;
  private stddev: number;
  
  constructor(sampleMean: number, sampleStddev: number) {
    let m2 = sampleMean * sampleMean;
    let s2 = sampleStddev * sampleStddev;
    this.mean = Math.log(m2 / Math.sqrt(m2 + s2));
    this.stddev = Math.sqrt(Math.log(1 + s2 / m2));
  }

  private x(x: number): number {
    return (Math.log(x) - this.mean) / this.stddev;
  }
  
  public p(x: number): number {
	  return cdf(this.x(x));
  }

  public dx(x: number): number {
	  return pdf(this.x(x) / (this.stddev * x));
  }

  public toHTML(): string {
    return 'LogNormal(&mu; = ' + this.mean + ', &sigma; = ' + this.stddev + ')';
  }
}

// A special case fo handling case where mean is zero.
class _LogZero implements CDF {
  
  constructor() {}
  
  public p(x: number): number {
	  return 0;
  }

  public dx(x: number): number {
	  return 0;
  }

  public toHTML(): string {
    return 'LogNormal(&mu; = &infin;, &sigma; = -&infin;)';
  }
}

export function LogNormal(mean: number, stddev: number): CDF {
  if (mean <= 0) {
    return new _LogZero();
  }
  return new _LogNormal(mean, stddev);
}
