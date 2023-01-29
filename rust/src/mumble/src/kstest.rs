// An implementation of the Kolmogorov-Smirnov Test.
//
// Ported from CERN's Root data analysis framework.
// Specifically the TMath::KolmogorovProb() function, originally written in C++.
// Original source available at:
// (https://github.com/root-project/root) root/math/mathcore/src/TMath.cxx
//
// Authors: Rene Brun, Anna Kreshuk, Eddy Offermann, Fons Rademakers   29/07/95
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 2.1 of the License, or (at your option) any later version.

// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// The license is available online at:
// https://www.gnu.org/licenses/old-licenses/lgpl-2.1.en.html
// otherwise, write to:
//   Free Software Foundation, Inc.,
//   51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA

// FUTURE WORK:
// Is: https://en.wikipedia.org/wiki/Anderson%E2%80%93Darling_test
// Different? Better? In what ways?

/// Round to nearest integer. Rounds half integers to the nearest even integer.
fn nint(x: f64) -> i64 {
    let mut i: i64;
    if x.is_sign_positive() {
        i = (x + 0.5).trunc() as i64;
        if (i & 1) != 0 && x.fract() == 0.5 {
            i -= 1;
        }
    } else {
        i = (x - 0.5).trunc() as i64;
        if (i & 1) != 0 && x.fract() == -0.5 {
            i += 1;
        }
    }
    i
}

/// Calculates the Kolmogorov distribution function,
/// which gives the probability that Kolmogorov's test statistic will exceed
/// the value z assuming the null hypothesis. This gives a very powerful
/// test for comparing two one-dimensional distributions.
/// see, for example, Eadie et al, "Statistical Methods in Experimental
/// Physics', pp 269-270).
///
/// This function returns the confidence level for the null hypothesis, where:
///   z  = dn*sqrt(n), and
///   dn = is the maximum deviation between a hypothetical distribution
///        function and an experimental distribution with
///   n  = events
///
/// NOTE: To compare two experimental distributions with m and n events,
/// use z = sqrt(m*n/(m+n))*dn
///
/// Accuracy: The function is far too accurate for any imaginable application.
/// Probabilities less than 10^-15 are returned as zero.
/// However, remember that the formula is only valid for "large" n.
/// Theta function inversion formula is used for z <= 1
///
fn kprob(z: f64) -> f64 {
    if z < 0.2 {
        1.0
    } else if z < 0.755 {
        const W: f64 = 2.50662827;
        // c1 - -pi**2/8, c2 = 9*c1, c3 = 25*c1
        const C1: f64 = -1.2337005501361697;
        const C2: f64 = -11.103304951225528;
        const C3: f64 = -30.842513753404244;
        let v = 1.0 / (z * z);
        1.0 - W * ((C1 * v).exp() + (C2 * v).exp() + (C3 * v).exp()) / z
    } else if z < 6.8116 {
        const FJ: [f64; 4] = [-2.0, -8.0, -18.0, -32.0];
        let mut r = [0.0, 0.0, 0.0, 0.0];
        let v = z * z;
        let maxj = match nint(3.0 / z) {
            j if j < 1 => 1,
            j => j as u64 as usize,
        };
        for j in 0..maxj {
            r[j] = (FJ[j] * v).exp();
        }
        2.0 * (r[0] - r[1] + r[2] - r[3])
    } else {
        0.0
    }
}

/// Runs a Kolmogorov-Smirnov test against a given reference distribution.
///
/// The returned value is the calculated confidence level, an estimate of the
/// likelihood that the sample comes from the reference distribution.
///
/// See:
/// https://en.wikipedia.org/wiki/Kolmogorov%E2%80%93Smirnov_test
pub fn ks_test<'a, V, F, I>(cdf: F, samples: I, count: usize) -> f64
where
    V: 'a + Copy,
    F: Fn(V) -> f64,
    I: Iterator<Item = &'a V>,
{
    // Find the maximum difference between the sample and the reference distribution.
    let n = count as f64;
    let mut max = 0.0;
    let mut ip = 0.0;
    for (i, x) in samples.enumerate() {
        let p = cdf(*x);
        let mut diff = p - ip;
        if diff.is_sign_negative() {
            diff = -diff;
        }
        if diff > max {
            max = diff;
        }
        ip = (i + 1) as f64 / n;
        diff = ip - p;
        if diff.is_sign_negative() {
            diff = -diff;
        }
        if diff > max {
            max = diff;
        }
    }
    let z = max * n.sqrt();
    kprob(z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use statrs::{assert_almost_eq, distribution::ContinuousCDF, distribution::Normal};

    #[test]
    fn test_nint() {
        const TEST_CASES: [(f64, i64); 17] = [
            (0.0, 0),
            (1.0, 1),
            (1.1, 1),
            (1.5, 2),
            (1.9, 2),
            (2.1, 2),
            (2.5, 2),
            (2.50001, 3),
            (2.6, 3),
            (-1.0, -1),
            (-1.1, -1),
            (-1.5, -2),
            (-1.9, -2),
            (-2.1, -2),
            (-2.5, -2),
            (-2.50001, -3),
            (-2.6, -3),
        ];
        for (f, i) in TEST_CASES {
            assert_eq!(nint(f), i, "nint({}) != {}", f, i);
        }
    }

    #[test]
    #[ignore = "doesn't pass yet"] // TODO: Not sure why... Investigate!
    fn r_example() {
        // Evaluated in R as a way to check the correctness of this implementation.
        //   ks.test(c(1,2,3), "pnorm", 0, 1) -->  0.007987
        let normal = Normal::new(0.0, 1.0).unwrap();
        let samples = &[1.0, 2.0, 3.0];
        assert_almost_eq!(
            ks_test(|x| normal.cdf(x), samples.iter(), samples.len()),
            0.007987,
            0.000001
        );
    }
}
