// Tools for collecting Emperical Cumulative Distribution Functions. (ECDFs)
// Copyright (C) 2022, Tony Rippy
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

use crate::kstest;
use num_traits::cast::ToPrimitive;
use num_traits::Num;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::convert::From;
use std::fmt::Debug;
use std::slice::Iter;

type SampleCount = u32;

#[derive(Clone, Debug, Default)]
pub struct ECDF<V> {
    samples: Vec<(V, SampleCount)>,
}

impl<V> ECDF<V>
where
    V: Num + ToPrimitive + PartialOrd + Copy + Debug,
{
    /// Removes all samples collected so far.
    pub fn clear(&mut self) {
        self.samples.clear()
    }

    /// Calculates sample mean, standard deviation, and count.
    pub fn stats(&self) -> (f64, f64, usize) {
        let mut sum = 0.0;
        let mut count: SampleCount = 0;
        for (v, n) in &self.samples {
            let vf = v.to_f64().unwrap();
            sum += vf * f64::from(*n);
            count += n;
        }
        let mean = sum / f64::from(count);
        sum = 0.0;
        for (v, n) in &self.samples {
            let vf = v.to_f64().unwrap();
            let err = vf - mean;
            sum += err * err * f64::from(*n);
        }
        let stddev = (sum / f64::from(count - 1)).sqrt();
        (mean, stddev, count as usize)
    }

    /// The total number of observations used to construct this ECDF.
    pub fn count(&self) -> SampleCount {
        self.samples.iter().fold(0, |sum, (_, n)| sum + *n)
    }

    /*
     Cases with Observe:

     For highest fidelity, we should try and keep the buffer as
     full as possible, and only compact down to the sample size
     when needed.  That implies that when the buffer is full and
     you observe a new value, then only one sample should be
     replaced.  OTOH, finding the value to replace may be an
     expensive operation, in which case for performance reasons we
     may want to compact when full in order to reduce the number
     of "compaction pauses." The downsize of that is that a lot of
     fidelity of the orginial data set is lost. (Or is it? We
     should measure.)

    */

    pub fn add(&mut self, sample: V, count: SampleCount) {
        match self
            .samples
            .binary_search_by(|(v, _)| v.partial_cmp(&sample).unwrap())
        {
            Ok(i) => {
                self.samples[i].1 += count;
            }
            Err(i) => {
                self.samples.insert(i, (sample, count));
            }
        }
    }

    pub fn merge_sorted(&mut self, it: Iter<(V, SampleCount)>) {
        let mut i: usize = 0;
        let mut n = self.samples.len();
        for (v, c) in it {
            loop {
                if i == n {
                    self.samples.push((*v, *c));
                    break;
                }
                match v.partial_cmp(&self.samples[i].0).unwrap() {
                    Ordering::Less => {
                        self.samples.insert(i, (*v, *c));
                        n += 1;
                        break;
                    }
                    Ordering::Equal => {
                        self.samples[i].1 += c;
                        break;
                    }
                    Ordering::Greater => {
                        i += 1;
                    }
                }
            }
            i += 1;
        }
    }

    pub fn compact_to(&mut self, mut target_size: usize) {
        if target_size < 3 {
            target_size = 3;
        }
        let mut len = self.samples.len();
        if len <= target_size {
            return;
        }

        // TODO:
        // errs could be stored as (index, err) pairs, like the enumerate() below.
        // Then you can do a pass that min sorts the list by err, takes the first
        // N items, extracts the indices, sort that, and use that to remove the
        // items in a way that minimizes copies in self.samples.

        // Calculate the errors for all elements except the ends.
        let mut errs = Vec::<f64>::with_capacity(len - 1);
        let mut x0 = self.samples[0].0;
        let (mut x1, mut y1) = self.samples[1];
        for i in 2..len {
            let (x2, y2) = self.samples[i];
            // Find expected y for x1, given linear interpolation between x0 and x2.
            let y = (x1 - x0).to_f64().unwrap() * f64::from(y1 + y2) / (x2 - x0).to_f64().unwrap();
            errs.push((f64::from(y1) - y).abs());
            x0 = x1;
            (x1, y1) = (x2, y2);
        }

        // Drop points one at a time until we reach the desired size.
        while len > target_size {
            // Find the sample with the lowest error.
            let mut best_index: usize = 0;
            let mut best_err = errs[0];
            if best_err > 0.0 {
                for (i, err) in errs.iter().enumerate().skip(1) {
                    if *err < best_err {
                        best_index = i;
                        if *err == 0.0 {
                            break;
                        }
                        best_err = *err;
                    }
                }
            }
            // Drop the chosen sample, add the sample count to the next greater sample.
            errs.remove(best_index);
            let (_, c) = self.samples.remove(best_index + 1);
            self.samples[best_index + 1].1 += c;
            len -= 1;

            // Recompute the error of points next to the removed sample.
            if best_index > 0 {
                let i = best_index - 1;
                x0 = self.samples[i].0;
                (x1, y1) = self.samples[best_index];
                let (x2, y2) = self.samples[best_index + 1];
                let y =
                    (x1 - x0).to_f64().unwrap() * f64::from(y1 + y2) / (x2 - x0).to_f64().unwrap();
                errs[i] = (f64::from(y1) - y).abs();
                x0 = x1;
                (x1, y1) = (x2, y2);
            } else {
                x0 = self.samples[0].0;
                (x1, y1) = self.samples[1];
            }
            if best_index < errs.len() {
                let (x2, y2) = self.samples[best_index + 2];
                let y =
                    (x1 - x0).to_f64().unwrap() * f64::from(y1 + y2) / (x2 - x0).to_f64().unwrap();
                errs[best_index] = (f64::from(y1) - y).abs();
            }
        }
    }

    /// Shrinks the capacity of the backing vector as much as possible, freeing memory.
    pub fn shrink_to_fit(&mut self) {
        self.samples.shrink_to_fit()
    }

    pub fn observe(&mut self, sample: V) {
        self.add(sample, 1)
    }

    // TODO: Would using an Anderson-Darling test be better? In what ways?
    // Is: https://en.wikipedia.org/wiki/Anderson%E2%80%93Darling_test

    /// Runs a Kolmogorov-Smirnov test against a given reference distribution.
    ///
    /// The returned value is the calculated confidence level, an estimate of the
    /// likelihood that the sample comes from the reference distribution.
    ///
    /// See:
    /// https://en.wikipedia.org/wiki/Kolmogorov%E2%80%93Smirnov_test
    pub fn drawn_from_distribution<F>(&self, cdf: F) -> f64
    where
        F: Fn(V) -> f64,
    {
        // Find the maximum difference between the sample and the reference distribution.
        let total = f64::from(self.count());
        let mut max_diff = 0.0;
        let mut p = 0.0;
        let mut sum: SampleCount = 0;
        for (v, n) in self.samples.iter() {
            let p_dist = cdf(*v);
            let mut diff = p_dist - p;
            if diff.is_sign_negative() {
                diff = -diff;
            }
            if diff > max_diff {
                max_diff = diff;
            }
            sum += *n;
            p = sum as f64 / total;
            diff = p_dist - p;
            if diff.is_sign_negative() {
                diff = -diff;
            }
            if diff > max_diff {
                max_diff = diff;
            }
        }
        let z = max_diff * total.sqrt();
        kstest::kprob(z)
    }

    /// Runs a two-sample Kolmogorov-Smirnov test.
    ///
    /// The returned value is the calculated confidence level, an estimate of the
    /// likelihood that the two samples were drawn from the same distribution.
    ///
    /// See:
    /// https://en.wikipedia.org/wiki/Kolmogorov%E2%80%93Smirnov_test#Two-sample_Kolmogorov%E2%80%93Smirnov_test
    pub fn drawn_from_same_distribution_as(&self, other: &ECDF<V>) -> f64 {
        let s_t = f64::from(self.count());
        let mut s_sum: SampleCount = 0;
        let mut s_iter = self.samples.iter();

        let o_t = f64::from(other.count());
        let mut o_sum: SampleCount = 0;
        let mut o_iter = other.samples.iter();

        let mut max_diff = 0.0;
        let mut s_i = s_iter.next();
        let mut o_i = o_iter.next();
        let mut s_p = 0.0;
        let mut o_p = 0.0;
        while let (Some((s_v, s_n)), Some((o_v, o_n))) = (s_i, o_i) {
            let cmp = s_v.partial_cmp(o_v).unwrap();
            if cmp.is_le() {
                s_sum += s_n;
                s_p = f64::from(s_sum) / s_t;
                s_i = s_iter.next();
            }
            if cmp.is_ge() {
                o_sum += o_n;
                o_p = f64::from(o_sum) / o_t;
                o_i = o_iter.next();
            }
            let mut diff: f64 = s_p - o_p;
            if diff.is_sign_negative() {
                diff = -diff;
            }
            if diff > max_diff {
                max_diff = diff;
            }
        }
        let z = max_diff * (s_t * o_t / (s_t + o_t)).sqrt();
        kstest::kprob(z)
    }
}

impl<V> Serialize for ECDF<V>
where
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.samples.serialize(serializer)
    }
}

impl<'de, V> Deserialize<'de> for ECDF<V>
where
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(ECDF::<V> {
            samples: Vec::deserialize::<D>(deserializer)?,
        })
    }
}

struct Counter<'a, V: 'a> {
    slice: &'a [V],
}

impl<'a, V: 'a> Iterator for Counter<'a, V>
where
    V: 'a + PartialEq + Copy,
{
    type Item = (V, SampleCount);

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            None
        } else {
            let v = self.slice[0];
            let mut i: usize = 1;
            let n = self.slice.len();
            loop {
                if i == n {
                    self.slice = &self.slice[n..n];
                    break;
                }
                if self.slice[i] != v {
                    self.slice = &self.slice[i..];
                    break;
                }
                i += 1;
            }
            Some((v, i.try_into().unwrap()))
        }
    }
}

impl<V> From<Vec<V>> for ECDF<V>
where
    V: PartialOrd + Copy,
{
    fn from(mut samples: Vec<V>) -> Self {
        samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let s = Counter { slice: &samples }.collect();
        ECDF { samples: s }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Distribution;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;
    use statrs::assert_almost_eq;
    use statrs::distribution::{ContinuousCDF, Normal};

    #[test]
    fn from_empty_slice() {
        let x: ECDF<i32> = ECDF::from(vec![]);
        assert_eq!(&x.samples.as_slice(), &[]);
        assert_eq!(x.count(), 0);
    }

    #[test]
    fn count_sorted() {
        let v: Vec<i32> = vec![1, 1, 2, 3, 3, 3];
        let mut c = Counter { slice: &v };
        assert_eq!(c.next(), Some((1, 2)));
        assert_eq!(c.next(), Some((2, 1)));
        assert_eq!(c.next(), Some((3, 3)));
        assert_eq!(c.next(), None);
        assert_eq!(c.next(), None);
    }

    #[test]
    fn from_unsorted_slice() {
        let x: ECDF<i32> = ECDF::from(vec![1, 1, 3, 3, 2, 10, 3, 2, 1]);
        assert_eq!(&x.samples.as_slice(), &[(1, 3), (2, 2), (3, 3), (10, 1)]);
        assert_eq!(x.count(), 9);
    }

    #[test]
    fn insert() {
        let mut x: ECDF<i32> = ECDF::default();
        assert_eq!(&x.samples.as_slice(), &[]);
        assert_eq!(x.count(), 0);

        x.observe(3);
        assert_eq!(&x.samples.as_slice(), &[(3, 1)]);
        assert_eq!(x.count(), 1);

        x.observe(1);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 1)]);
        assert_eq!(x.count(), 2);

        x.observe(5);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 1), (5, 1)]);
        assert_eq!(x.count(), 3);
    }

    /// Verifies that insertions at the beginning of the list work as expected.
    #[test]
    fn insert_beginning() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (3, 1), (5, 1)],
        };
        x.observe(0);
        assert_eq!(&x.samples.as_slice(), &[(0, 1), (1, 1), (3, 1), (5, 1)]);
        assert_eq!(x.count(), 4);
        x.observe(0);
        assert_eq!(&x.samples.as_slice(), &[(0, 2), (1, 1), (3, 1), (5, 1)]);
        assert_eq!(x.count(), 5);
    }

    /// Verifies that insertions at the end of the list work as expected.
    #[test]
    fn insert_end() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (3, 1), (5, 1)],
        };
        x.observe(6);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 1), (5, 1), (6, 1)]);
        assert_eq!(x.count(), 4);
        x.observe(6);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 1), (5, 1), (6, 2)]);
        assert_eq!(x.count(), 5);
    }

    #[test]
    fn insert_between_1_and_3() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (3, 2), (5, 2)],
        };
        x.observe(2);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (2, 1), (3, 2), (5, 2)]);
        assert_eq!(x.count(), 6);
        x.observe(2);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (2, 2), (3, 2), (5, 2)]);
        assert_eq!(x.count(), 7);
    }

    #[test]
    fn merge() {
        let mut x: ECDF<i32> = ECDF::default();
        assert_eq!(x.count(), 0);

        let empty: ECDF<i32> = ECDF::default();
        x.merge_sorted(empty.samples.iter());
        assert_eq!(x.count(), 0);

        let mut y: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1)],
        };
        assert_eq!(y.count(), 3);
        y.merge_sorted(empty.samples.iter());
        assert_eq!(y.count(), 3);

        let mut not_empty = ECDF {
            samples: vec![(0, 1)],
        };
        y.merge_sorted(not_empty.samples.iter());
        assert_eq!(&y.samples.as_slice(), &[(0, 1), (1, 1), (2, 1), (3, 1)]);
        assert_eq!(y.count(), 4);
        not_empty = ECDF {
            samples: vec![(4, 1)],
        };
        y.merge_sorted(not_empty.samples.iter());
        assert_eq!(
            &y.samples.as_slice(),
            &[(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)]
        );
        assert_eq!(y.count(), 5);
    }

    /// Verifies correct behavior when samples are in a straight line.
    #[test]
    fn compact_line() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)],
        };
        x.compact_to(4);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 2), (4, 1), (5, 1)]);
        assert_eq!(x.count(), 5);
    }

    /// Verifies that the minimum size post-compaction is 3: (min, ???, max)
    #[test]
    fn compact_min() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)],
        };
        x.compact_to(1);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (4, 3), (5, 1)]);
        assert_eq!(x.count(), 5);
    }

    /// Verifies that a compaction is a no-op if the target size is greater than the current size.
    #[test]
    fn compact_max() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)],
        };
        x.compact_to(5);
        assert_eq!(
            &x.samples.as_slice(),
            &[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]
        );
        assert_eq!(x.count(), 5);
        x.compact_to(100);
        assert_eq!(
            &x.samples.as_slice(),
            &[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]
        );
        assert_eq!(x.count(), 5);
    }

    /// Performs compactions with non-zero errors.
    #[test]
    fn compact_non_zero() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 2), (4, 4), (5, 10)],
        };
        x.compact_to(4);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 3), (4, 4), (5, 10)]);
        assert_eq!(x.count(), 18);

        x = ECDF {
            samples: vec![
                (1, 10),
                (2, 4),
                (3, 3),
                (4, 2),
                (5, 1),
                (25, 10),
                (100, 100),
            ],
        };
        let before = x.count();
        x.compact_to(4);
        assert_eq!(
            &x.samples.as_slice(),
            &[(1, 10), (4, 9), (25, 11), (100, 100)]
        );
        assert_eq!(x.count(), before);
    }

    #[test]
    fn good_fit() {
        let x = ECDF::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let (mean, stddev, _) = x.stats();
        let normal = Normal::new(mean, stddev).unwrap();
        let p = x.drawn_from_distribution(|x| normal.cdf(x));
        assert!(p > 0.99, "Expected p > 0.99, was {}", p);
    }

    #[test]
    fn matches_itself() {
        let x = ECDF::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        //let p =
        assert_eq!(x.drawn_from_same_distribution_as(&x), 1.0); //;p > 0.8, "Expected p > 0.8, was {}", p);
    }

    #[test]
    fn doesnt_match_disjoint_sample() {
        let x = ECDF::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let y = ECDF::from(vec![11.0, 12.0, 13.0, 14.0, 15.0]);
        let p = x.drawn_from_same_distribution_as(&y);
        assert!(p < 0.02, "Expected p < 0.02, was {}", p);
    }

    #[test]
    #[ignore = "flaky due to random sampling"]
    fn drawn_from_same_distribution() {
        let mut rng = SmallRng::from_entropy();
        let normal = Normal::new(2.0, 3.0).unwrap();
        let x = ECDF::from(normal.sample_iter(&mut rng).take(20).collect::<Vec<f64>>());
        println!(
            "P(x drawn from dist) = {}",
            x.drawn_from_distribution(|x| normal.cdf(x))
        );
        let y = ECDF::from(normal.sample_iter(&mut rng).take(15).collect::<Vec<f64>>());
        println!(
            "P(y drawn from dist2) = {}",
            y.drawn_from_distribution(|x| normal.cdf(x))
        );
        let p = x.drawn_from_same_distribution_as(&y);
        assert!(p > 0.8, "Expected p > 0.8, was {}", p);
    }

    #[test]
    #[ignore = "doesn't pass due to different method of calculating p-value"]
    fn r_example() {
        // Evaluated in R as a way to check the correctness of this implementation.
        //   ks.test(c(1,2,3), "pnorm", 0, 1) -->  0.007987
        let normal = Normal::new(2.0, 3.0).unwrap();
        let x = ECDF::from(vec![1.0, 2.0, 3.0]);
        assert_almost_eq!(
            x.drawn_from_distribution(|x| normal.cdf(x)),
            0.007987,
            0.000001
        );
    }
}
