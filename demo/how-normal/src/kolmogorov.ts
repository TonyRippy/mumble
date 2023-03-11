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

import { CDF } from "./cdf";

// Round to nearest integer. Rounds half integers to the nearest even integer.
function nint(x: number): number {
	var i: number;
	if (x >= 0) {
		i = Math.trunc(x + 0.5);
		if ((i & 1) != 0 && (x+0.5) == i) {
			i -= 1;
		}
	} else {
		i = Math.trunc(x - 0.5);
		if ((i & 1) != 0 && (x - 0.5) == i) {
			i += 1;
		}
	}
	return i;
}

/*
Calculates the Kolmogorov distribution function,
which gives the probability that Kolmogorov's test statistic will exceed
the value z assuming the null hypothesis. This gives a very powerful
test for comparing two one-dimensional distributions.
see, for example, Eadie et al, "statistocal Methods in Experimental
Physics', pp 269-270).

This function returns the confidence level for the null hypothesis, where:
  z  = dn*sqrt(n), and
  dn = is the maximum deviation between a hypothetical distribution
       function and an experimental distribution with
  n  = events

NOTE: To compare two experimental distributions with m and n events,
use z = sqrt(m*n/(m+n))*dn

Accuracy: The function is far too accurate for any imaginable application.
Probabilities less than 10^-15 are returned as zero.
However, remember that the formula is only valid for "large" n.
Theta function inversion formula is used for z <= 1

Ported from CERN's Root data analysis framework. (https://root.cern.ch/)
Specifically the TMath::KolmogorovProb() function, originally written in C++.
Source here: https://root.cern.ch/root/html/src/TMath.cxx.html
*/
function kprob(z: number): number {
	var p: number;
	if (z < 0.2) {
		p = 1;
	} else if (z < 0.755) {
		const w = 2.50662827;
		// c1 - -pi**2/8, c2 = 9*c1, c3 = 25*c1
		const c1 = -1.2337005501361697;
		const c2 = -11.103304951225528;
		const c3 = -30.842513753404244;
		let v = 1.0 / (z * z);
		p = 1 - w*(Math.exp(c1*v)+Math.exp(c2*v)+Math.exp(c3*v))/z;
	} else if (z < 6.8116) {
		const fj = [-2, -8, -18, -32];
		let r = [0, 0, 0, 0];
		let v = z * z;
		let maxj = nint(3.0 / z);
		if (maxj < 1) {
			maxj = 1;
		}
		for (let j = 0; j < maxj; j += 1) {
			r[j] = Math.exp(fj[j] * v);
		}
		p = 2 * (r[0] - r[1] + r[2] - r[3]);
	} else {
		p = 0;
	}
	return p;
}

/*
Runs a Kolmogorov-Smirnov test for a given sample and reference distribution.

The returned value is the calculated confidence level, an estimate of the
likelihood that the given sample comes from the reference distribution.

See:
https://en.wikipedia.org/wiki/Kolmogorov%E2%80%93Smirnov_test
*/
export function KSTest(cdf: CDF, sample: number[]): number {
	// Find the maximum difference between the sample and the reference distribution.
  sample.sort((a,b) => a - b);
	let max = 0.0;
	let ip = 0.0;
	for (let i = 0; i < sample.length; i++) {
    const x = sample[i];
		const p = cdf.p(x);
		let diff = p - ip;
		if (diff < 0) {
			diff = -diff;
		}
		if (diff > max) {
			max = diff;
		}
		ip = (i + 1) / sample.length;
		diff = ip - p;
		if (diff < 0) {
			diff = -diff
		}
		if (diff > max) {
			max = diff
		}
	}
	let z = max * Math.sqrt(sample.length);
	let p = kprob(z);
	return p;
}
