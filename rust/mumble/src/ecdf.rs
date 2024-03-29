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
use num_traits::{Float, Num};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::convert::From;
use std::fmt::Debug;
use std::iter::FusedIterator;

#[derive(Clone, Debug, Default)]
pub struct ECDF<V> {
    samples: Vec<(V, usize)>,
}

impl<V> ECDF<V>
where
    V: Num + ToPrimitive + PartialOrd + Copy + Debug,
{
    /// Removes all samples collected so far.
    pub fn clear(&mut self) {
        self.samples.clear()
    }

    /// The total number of samples used to construct this ECDF.
    pub fn len(&self) -> usize {
        self.samples.iter().map(|x| x.1).sum()
    }

    /// Returns `true` is this ECDF has no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Calculates sample mean, standard deviation, and count.
    pub fn stats(&self) -> (f64, f64, usize) {
        let mut sum = 0.0;
        let mut count = 0;
        for &(v, n) in &self.samples {
            let vf = v.to_f64().unwrap();
            sum += vf * (n as f64);
            count += n;
        }
        let mean = sum / (count as f64);
        sum = 0.0;
        for &(v, n) in &self.samples {
            let vf = v.to_f64().unwrap();
            let err = vf - mean;
            sum += err * err * (n as f64);
        }
        let stddev = (sum / ((count - 1) as f64)).sqrt();
        (mean, stddev, count)
    }

    fn add_n(&mut self, sample: V, count: usize) {
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

    /// Adds a single observation to this ECDF.
    pub fn add(&mut self, sample: V) {
        self.add_n(sample, 1)
    }

    pub fn merge_sorted(&mut self, it: impl Iterator<Item = (V, usize)>) {
        let mut i = 0;
        let mut n = self.samples.len();
        for (v, c) in it {
            loop {
                if i == n {
                    self.samples.push((v, c));
                    n += 1;
                    break;
                }
                match v.partial_cmp(&self.samples[i].0).unwrap() {
                    Ordering::Less => {
                        self.samples.insert(i, (v, c));
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

    pub fn compact(&mut self, target_size: usize) {
        self.compact_if(target_size, target_size)
    }

    pub fn compact_if(&mut self, over_size: usize, target_size: usize) {
        if target_size < 3 {
            return self.compact_if(over_size, 3);
        }
        let mut len = self.samples.len();
        if len <= over_size {
            // Hasn't hit the threshold that would trigger compaction.
            return;
        }
        if len <= target_size {
            // Already smaller than target size, nothing to do.
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
            let y = (x1 - x0).to_f64().unwrap() * ((y1 + y2) as f64) / (x2 - x0).to_f64().unwrap();
            errs.push((y1 as f64 - y).abs());
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
                    (x1 - x0).to_f64().unwrap() * ((y1 + y2) as f64) / (x2 - x0).to_f64().unwrap();
                errs[i] = (y1 as f64 - y).abs();
                x0 = x1;
                (x1, y1) = (x2, y2);
            } else {
                x0 = self.samples[0].0;
                (x1, y1) = self.samples[1];
            }
            if best_index < errs.len() {
                let (x2, y2) = self.samples[best_index + 2];
                let y =
                    (x1 - x0).to_f64().unwrap() * ((y1 + y2) as f64) / (x2 - x0).to_f64().unwrap();
                errs[best_index] = (y1 as f64 - y).abs();
            }
        }
    }

    /// Shrinks the capacity of the backing vector as much as possible, freeing memory.
    pub fn shrink_to_fit(&mut self) {
        self.samples.shrink_to_fit()
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
        let total = self.len() as f64;
        let mut max_diff = 0.0;
        let mut p = 0.0;
        let mut sum: usize = 0;
        for &(v, n) in self.samples.iter() {
            let p_dist = cdf(v);
            let mut diff = (p_dist - p).abs();
            if diff > max_diff {
                max_diff = diff;
            }
            sum += n;
            p = sum as f64 / total;
            diff = (p_dist - p).abs();
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
        let max_diff = self
            .zip(other)
            // find the difference between self and other at each point of the curve
            .map(|(_, a, b)| (a - b).abs())
            .reduce(|a, b| if a < b { b } else { a })
            .unwrap_or(0.0);
        let n = self.len();
        let m = other.len();
        let z = max_diff * ((n * m) as f64 / (n + m) as f64).sqrt();
        kstest::kprob(z)
    }

    /// Iterates through all points on the ECDF curve.
    /// The returned iterator generates (V, P(v <= V)) tuples.
    pub fn point_iter(&self) -> impl Iterator<Item = (V, f64)> + '_ {
        self.samples
            .iter()
            .scan((0, self.len() as f64), |(sum, total), &(v, n)| {
                *sum += n;
                Some((v, *sum as f64 / *total))
            })
    }

    /// Iterates through all points of comparison between two ECDF curves.
    /// The returned iterator generates (V, P(self <= V), P(other <= V)) tuples.
    fn zip<'a>(&'a self, other: &'a ECDF<V>) -> impl Iterator<Item = (V, f64, f64)> + 'a {
        let mut a_iter = self.point_iter();
        let a_item = a_iter.next();
        let mut b_iter = other.point_iter();
        let b_item = b_iter.next();
        Zip {
            a_iter,
            b_iter,
            a_item,
            b_item,
            a: 0.0,
            b: 0.0,
        }
    }

    /// Calculates the area difference between the two ECDFs.
    pub fn area_difference(&self, other: &ECDF<V>) -> f64 {
        let mut it = self
            .zip(other)
            // find the difference between self and other at each point of the curve
            .map(|(v, a, b)| (v, (a - b).abs()));
        let mut last: (V, f64);
        match it.next() {
            Some(x) => {
                last = x;
            }
            _ => {
                return 0.0;
            }
        }
        let mut sum = 0.0;
        for now in it {
            // The space between last and now makes a rectangle:
            //
            //                       +---------
            //                       |
            //            now.1 -->  |
            //                       |
            //             +---------+
            //             |         :
            //  last.1 --> |         :
            //             |         :
            //       ------+.........:
            //
            //       0   last.0     now.0
            //
            // The width of this rectangle is (now.0 - last.0), the height is last.1.
            let w = (now.0 - last.0).to_f64().unwrap();
            let area = w * last.1;
            sum += area;
            last = now;
        }
        sum
    }
}

impl<V> ECDF<V>
where
    V: Float + Debug,
{
    pub fn interpolate(&self) -> InterpolatedECDF<V> {
        InterpolatedECDF {
            samples: self.samples.iter().map(|&(v, n)| (v, n as f64)).collect(),
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
    type Item = (V, usize);

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
            Some((v, i))
        }
    }
}

struct Zip<A, B>
where
    A: Iterator,
    B: Iterator,
{
    a_iter: A,
    b_iter: B,
    a_item: Option<A::Item>,
    b_item: Option<B::Item>,
    a: f64,
    b: f64,
}

impl<A, B, V> Iterator for Zip<A, B>
where
    A: Iterator<Item = (V, f64)>,
    B: Iterator<Item = (V, f64)>,
    V: Copy + PartialOrd,
{
    type Item = (V, f64, f64);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.a_item, self.b_item) {
            (Some((a_v, a_p)), Some((b_v, b_p))) => {
                let cmp = a_v.partial_cmp(&b_v).unwrap();
                let v: V;
                if cmp.is_le() {
                    v = a_v;
                    self.a = a_p;
                    self.a_item = self.a_iter.next();
                } else {
                    v = b_v;
                }
                if cmp.is_ge() {
                    self.b = b_p;
                    self.b_item = self.b_iter.next();
                }
                Some((v, self.a, self.b))
            }
            (Some((a_v, a_p)), None) => {
                self.a_item = self.a_iter.next();
                Some((a_v, a_p, 1.0))
            }
            (None, Some((b_v, b_p))) => {
                self.b_item = self.b_iter.next();
                Some((b_v, 1.0, b_p))
            }
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.a_iter.size_hint();
        let (b_lower, b_upper) = self.b_iter.size_hint();
        let lower = std::cmp::max(a_lower, b_lower);
        let upper = match (a_upper, b_upper) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        };
        (lower, upper)
    }
}

impl<A, B, V> FusedIterator for Zip<A, B>
where
    A: Iterator<Item = (V, f64)>,
    B: Iterator<Item = (V, f64)>,
    V: Copy + PartialOrd,
{
}

#[derive(Clone, Debug, Default)]
pub struct InterpolatedECDF<V>
where
    V: Float + Debug,
{
    samples: Vec<(V, f64)>,
}

impl<V> InterpolatedECDF<V>
where
    V: Float + Debug,
{
    /// The total number of samples used to construct this ECDF.
    pub fn len(&self) -> f64 {
        self.samples.iter().map(|x| x.1).sum()
    }

    // TODO: Use a Result<V,?> for these functions rather than returing NaN.

    pub fn quantile(&self, q: f64) -> V {
        if q.is_nan() {
            return V::nan();
        }
        if q < 0.0 {
            return V::neg_infinity();
        }
        if q > 1.0 {
            return V::infinity();
        }
        if self.samples.is_empty() {
            return V::nan();
        }

        let mut rank = self.len() * q;
        let mut lv = self.samples[0].0;
        let first = self.samples[0].1;
        if first > rank {
            if self.samples.len() < 2 {
                return V::nan();
            }
            // Find the slope between samples 0 and 1, project backwards.
            let dv = (self.samples[1].0 - lv).to_f64().unwrap();
            let dc = self.samples[1].1;
            let m = dv / dc;
            return lv + V::from((rank - first) * m).unwrap();
        }
        rank -= first;
        for &(v, count) in self.samples.iter().skip(1) {
            let n = count;
            if n > rank {
                let fraction = V::from(rank / n).unwrap();
                return lv + (v - lv) * fraction;
            }
            lv = v;
            rank -= n;
        }
        lv
    }

    pub fn fraction(&self, v: V) -> f64 {
        if v.is_nan() {
            return f64::nan();
        }
        if self.samples.is_empty() {
            return f64::nan();
        }

        let rank;
        let mut sum;
        let mut iter = self.samples.iter();
        let (mut last_v, last_count) = match iter.next() {
            Some(&(v, n)) => {
                sum = n;
                (v, n)
            }
            _ => return f64::nan(),
        };
        if v < last_v {
            let (next_v, next_count) = match iter.next() {
                Some(&(v, n)) => {
                    sum += n;
                    (v, n)
                }
                _ => return f64::nan(),
            };
            // Find the slope between samples 0 and 1, project backwards.
            let dv = (next_v - last_v).to_f64().unwrap();
            let m = next_count / dv;
            rank = last_count + (v - last_v).to_f64().unwrap() * m;
        } else {
            loop {
                let (next_v, next_count) = match iter.next() {
                    Some(&(v, n)) => {
                        sum += n;
                        (v, n)
                    }
                    None => {
                        rank = sum;
                        break;
                    }
                };
                if v < next_v {
                    let dv = (next_v - last_v).to_f64().unwrap();
                    let m = next_count / dv;
                    rank = sum + (v - next_v).to_f64().unwrap() * m;
                    break;
                }
                last_v = next_v;
            }
        };
        for &(_, n) in iter {
            sum += n;
        }
        (rank / sum).clamp(0.0, 1.0)
    }

    // TODO: It should be possible to turn this into an iterator using flat_map.

    fn interpolate_counts<I: Iterator<Item = V>>(&self, mut points_iter: I) -> Vec<(V, f64)> {
        if self.samples.is_empty() {
            return points_iter.map(|v| (v, 0.0)).collect();
        }
        let mut points_item = points_iter.next();
        if points_item.is_none() {
            return self.samples.clone();
        }

        let mut out = Vec::with_capacity(
            self.samples.len()
                + match points_iter.size_hint() {
                    (_, Some(upper)) => upper,
                    (lower, None) => lower,
                },
        );
        // Establish the starting point for interpolation.
        let mut samples_iter = self.samples.iter().peekable();
        let mut lower_v = match (points_item, samples_iter.peek()) {
            (Some(v), Some(&&(v2, c))) => {
                if v < v2 {
                    out.push((v, 0.0));
                    points_item = points_iter.next();
                    v
                } else {
                    // Copy the first sample
                    out.push((v2, c));
                    samples_iter.next();
                    v2
                }
            }
            _ => {
                panic!("empty merge");
            }
        };

        // Walk the remaining samples, interpolating as needed.
        let mut points_between = Vec::new();
        for &sample in samples_iter {
            let upper_v = sample.0;

            // Find all points between lower_v inclusive and upper_v exclusive.
            points_between.clear();
            if let Some(v) = points_item {
                if v == lower_v {
                    points_item = points_iter.next();
                }
            }
            while let Some(v) = points_item {
                if v < sample.0 {
                    points_between.push(v);
                    points_item = points_iter.next();
                } else {
                    break;
                }
            }

            if points_between.is_empty() {
                // no interpolation needed, just add upper_v!
                out.push((upper_v, sample.1));
            } else {
                let dv = (upper_v - lower_v).to_f64().unwrap();
                let m = sample.1 / dv;
                let mut last_count = 0.0;
                for &v in points_between.iter() {
                    let new_count = (v - lower_v).to_f64().unwrap() * m;
                    out.push((v, new_count - last_count));
                    last_count = new_count;
                }
                out.push((upper_v, sample.1 - last_count));
            }
            lower_v = upper_v;
        }

        // Copy any points after the last sample.
        if let Some(v) = points_item {
            if v > lower_v {
                out.push((v, 0.0));
            }
            for v in points_iter {
                out.push((v, 0.0));
            }
        }
        out
    }

    pub fn merge(&self, other: &InterpolatedECDF<V>) -> InterpolatedECDF<V> {
        if self.samples.is_empty() {
            return other.clone();
        }
        if other.samples.is_empty() {
            return self.clone();
        }
        let self_counts = self.interpolate_counts(other.samples.iter().map(|&(v, _)| v));
        let other_counts = other.interpolate_counts(self.samples.iter().map(|&(v, _)| v));
        InterpolatedECDF {
            samples: self_counts
                .iter()
                .zip(other_counts.iter())
                .map(|(&(v1, c1), &(_, c2))| (v1, c1 + c2))
                .collect(),
        }
    }

    pub fn area_difference(&self, other: &InterpolatedECDF<V>) -> f64 {
        // Iterate over both ECDFs, iterating betwen points as necessary.
        let self_counts = self
            .interpolate_counts(other.samples.iter().map(|&(v, _)| v))
            .into_iter()
            .scan((0.0, self.len()), |(sum, total), (v, n)| {
                *sum += n;
                Some((v, *sum / *total))
            });
        let other_counts = other
            .interpolate_counts(self.samples.iter().map(|&(v, _)| v))
            .into_iter()
            .scan((0.0, other.len()), |(sum, total), (v, n)| {
                *sum += n;
                Some((v, *sum / *total))
            });

        // Zip the two iterators together. All points should have the same X values.
        let mut join = self_counts.zip(other_counts).map(|((v1, c1), (v2, c2))| {
            debug_assert_eq!(v1, v2);
            (v1, c1, c2)
        });

        // Calulate the area difference between each point and the next.
        let mut last = match join.next() {
            Some(x) => x,
            _ => return 0.0,
        };
        let mut sum = 0.0;
        for next in join {
            // Gather the points into two lines that share X coordinates:
            //   Line "A" = (x1, y1_a) -> (x2, y2_a)
            //   Line "B" = (x1, y1_b) -> (x2, y2_b)
            let (x1, mut y1_a, mut y1_b) = last;
            let (x2, mut y2_a, mut y2_b) = next;
            // Swap the two lines so that line "A" always starts above line "B".
            if y1_b > y1_a {
                std::mem::swap(&mut y1_a, &mut y1_b);
                std::mem::swap(&mut y2_a, &mut y2_b);
            }
            // Check whether line "A" also *finishes* above line "B".
            let area = if y2_b > y2_a {
                // When this happens it means the lines cross somewhere in the middle.
                // This results in a "bow-tie" shape; two triangles touching point-to-point.
                // The area formula for this is more complex.
                let x1 = x1.to_f64().unwrap();
                let x2 = x2.to_f64().unwrap();
                debug_assert!(x2 > x1);
                let dx = x2 - x1;

                let m_a = (y2_a - y1_a) / dx;
                debug_assert!(m_a >= 0.0);
                let m_b = (y2_b - y1_b) / dx;
                debug_assert!(m_a >= 0.0);

                let b_a = y1_a - m_a * x1;
                debug_assert!((y2_a - (m_a * x2 + b_a)).abs() < 1e-10);
                let b_b = y1_b - m_b * x1;
                debug_assert!((y2_b - (m_b * x2 + b_b)).abs() < 1e-10);

                let x_intersect = (b_b - b_a) / (m_a - m_b);
                debug_assert!(x_intersect - x1 > -1e-10); // x_intersect == x1 when y1_a == y1_b.
                debug_assert!(x_intersect < x2);

                let h1 = y1_a - y1_b;
                debug_assert!(h1 >= 0.0);
                let h2 = y2_b - y2_a;
                debug_assert!(h2 > 0.0);

                0.5 * ((x_intersect - x1) * h1 + (x2 - x_intersect) * h2)
            } else {
                // The area between the lines is a trapazoid.
                let dx = (x2 - x1).to_f64().unwrap();
                let dy1 = y1_a - y1_b;
                let dy2 = y2_a - y2_b;
                0.5 * dx * (dy1 + dy2)
            };
            sum += area;
            last = next;
        }
        sum
    }
}

impl<V> Serialize for InterpolatedECDF<V>
where
    V: Float + Debug + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.samples.serialize(serializer)
    }
}

impl<'de, V> Deserialize<'de> for InterpolatedECDF<V>
where
    V: Float + Debug + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(InterpolatedECDF::<V> {
            samples: Vec::deserialize::<D>(deserializer)?,
        })
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
        assert_eq!(x.len(), 0);
    }

    #[test]
    fn count_sorted() {
        let v: Vec<i32> = vec![1, 1, 2, 3, 3, 3];
        let c = Counter { slice: &v };
        itertools::assert_equal(c, [(1, 2), (2, 1), (3, 3)].into_iter());
    }

    #[test]
    fn from_unsorted_slice() {
        let x: ECDF<i32> = ECDF::from(vec![1, 1, 3, 3, 2, 10, 3, 2, 1]);
        assert_eq!(&x.samples.as_slice(), &[(1, 3), (2, 2), (3, 3), (10, 1)]);
        assert_eq!(x.len(), 9);
    }

    #[test]
    fn stats() {
        let x: ECDF<i32> = ECDF::from(vec![1, 1, 2, 3, 5, 8]);
        let (mean, stddev, count) = x.stats();
        assert_almost_eq!(mean, 3.33333, 0.00001);
        assert_almost_eq!(stddev, 2.73252, 0.00001);
        assert_eq!(count, 6);
    }

    #[test]
    fn insert() {
        let mut x: ECDF<i32> = ECDF::default();
        assert_eq!(&x.samples.as_slice(), &[]);
        assert_eq!(x.len(), 0);

        x.add(3);
        assert_eq!(&x.samples.as_slice(), &[(3, 1)]);
        assert_eq!(x.len(), 1);

        x.add_n(1, 2);
        assert_eq!(&x.samples.as_slice(), &[(1, 2), (3, 1)]);
        assert_eq!(x.len(), 3);

        x.add(5);
        assert_eq!(&x.samples.as_slice(), &[(1, 2), (3, 1), (5, 1)]);
        assert_eq!(x.len(), 4);
    }

    /// Verifies that insertions at the beginning of the list work as expected.
    #[test]
    fn insert_beginning() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (3, 1), (5, 1)],
        };
        x.add(0);
        assert_eq!(&x.samples.as_slice(), &[(0, 1), (1, 1), (3, 1), (5, 1)]);
        assert_eq!(x.len(), 4);
        x.add(0);
        assert_eq!(&x.samples.as_slice(), &[(0, 2), (1, 1), (3, 1), (5, 1)]);
        assert_eq!(x.len(), 5);
    }

    /// Verifies that insertions at the end of the list work as expected.
    #[test]
    fn insert_end() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (3, 1), (5, 1)],
        };
        assert_eq!(x.len(), 3);
        x.add(6);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 1), (5, 1), (6, 1)]);
        assert_eq!(x.len(), 4);
        x.add(6);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 1), (5, 1), (6, 2)]);
        assert_eq!(x.len(), 5);
    }

    #[test]
    fn insert_between_1_and_3() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (3, 2), (5, 2)],
        };
        assert_eq!(x.len(), 5);
        x.add(2);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (2, 1), (3, 2), (5, 2)]);
        assert_eq!(x.len(), 6);
        x.add(2);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (2, 2), (3, 2), (5, 2)]);
        assert_eq!(x.len(), 7);
    }

    #[test]
    fn merge_into_empty_ecdf() {
        let mut x: ECDF<i32> = ECDF::default();
        assert_eq!(x.len(), 0);

        x.merge_sorted(std::iter::empty());
        assert_eq!(x.len(), 0);

        let y: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1)],
        };
        assert_eq!(y.len(), 3);
        x.merge_sorted(y.samples.iter().cloned());
        assert_eq!(x.len(), 3);
        assert_eq!(&x.samples.as_slice(), &y.samples.as_slice());
    }

    #[test]
    fn merge_into_non_empty_ecdf() {
        let mut y: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1)],
        };
        assert_eq!(y.len(), 3);
        y.merge_sorted(std::iter::empty());
        assert_eq!(y.len(), 3);

        let mut not_empty = ECDF {
            samples: vec![(0, 1)],
        };
        y.merge_sorted(not_empty.samples.into_iter());
        assert_eq!(&y.samples.as_slice(), &[(0, 1), (1, 1), (2, 1), (3, 1)]);
        assert_eq!(y.len(), 4);

        not_empty = ECDF {
            samples: vec![(2, 2), (4, 1), (10, 2)],
        };
        y.merge_sorted(not_empty.samples.into_iter());
        assert_eq!(
            &y.samples.as_slice(),
            &[(0, 1), (1, 1), (2, 3), (3, 1), (4, 1), (10, 2)]
        );
        assert_eq!(y.len(), 9);
    }

    /// Verifies correct behavior when samples are in a straight line.
    #[test]
    fn compact_line() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)],
        };
        x.compact(4);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 2), (4, 1), (5, 1)]);
        assert_eq!(x.len(), 5);
    }

    /// Verifies that the minimum size post-compaction is 3: (min, ???, max)
    #[test]
    fn compact_min() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)],
        };
        x.compact(1);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (4, 3), (5, 1)]);
        assert_eq!(x.len(), 5);
    }

    /// Verifies that a compaction is a no-op if the target size is greater than the current size.
    #[test]
    fn compact_max() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)],
        };
        x.compact(5);
        assert_eq!(
            &x.samples.as_slice(),
            &[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]
        );
        assert_eq!(x.len(), 5);
        x.compact(100);
        assert_eq!(
            &x.samples.as_slice(),
            &[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]
        );
        assert_eq!(x.len(), 5);
    }

    /// Performs compactions with non-zero errors.
    #[test]
    fn compact_non_zero() {
        let mut x: ECDF<i32> = ECDF {
            samples: vec![(1, 1), (2, 1), (3, 2), (4, 4), (5, 10)],
        };
        x.compact(4);
        assert_eq!(&x.samples.as_slice(), &[(1, 1), (3, 3), (4, 4), (5, 10)]);
        assert_eq!(x.len(), 18);

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
        let before = x.len();
        x.compact(4);
        assert_eq!(
            &x.samples.as_slice(),
            &[(1, 10), (4, 9), (25, 11), (100, 100)]
        );
        assert_eq!(x.len(), before);
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

    #[test]
    fn point_iter() {
        let x = ECDF::from(vec![1, 2, 2, 3]);
        itertools::assert_equal(x.point_iter(), [(1, 0.25), (2, 0.75), (3, 1.0)].into_iter());
    }

    #[test]
    fn zip_ecdfs_interleave() {
        let a = ECDF::from(vec![1, 3, 3, 5]);
        let b = ECDF::from(vec![2, 2, 3, 4]);
        let mut it = a.zip(&b);
        assert_eq!(it.next(), Some((1, 0.25, 0.00)));
        assert_eq!(it.next(), Some((2, 0.25, 0.50)));
        assert_eq!(it.next(), Some((3, 0.75, 0.75)));
        assert_eq!(it.next(), Some((4, 0.75, 1.00)));
        assert_eq!(it.next(), Some((5, 1.00, 1.00)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn zip_ecdfs_empty() {
        let empty = ECDF::<i32>::default();
        let not = ECDF::from(vec![1, 2]);
        let mut it = empty.zip(&not);
        assert_eq!(it.next(), Some((1, 1.0, 0.5)));
        assert_eq!(it.next(), Some((2, 1.0, 1.0)));
        assert_eq!(it.next(), None);
        // It should work in the other direction too...
        it = not.zip(&empty);
        assert_eq!(it.next(), Some((1, 0.5, 1.0)));
        assert_eq!(it.next(), Some((2, 1.0, 1.0)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn zip_ecdfs_self() {
        let a = ECDF::from(vec![1, 2]);
        let mut it = a.zip(&a);
        assert_eq!(it.next(), Some((1, 0.5, 0.5)));
        assert_eq!(it.next(), Some((2, 1.0, 1.0)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn zip_ecdfs_no_overlap() {
        let a = ECDF::from(vec![1, 2]);
        let b = ECDF::from(vec![3, 4]);
        let mut it = a.zip(&b);
        assert_eq!(it.next(), Some((1, 0.5, 0.0)));
        assert_eq!(it.next(), Some((2, 1.0, 0.0)));
        assert_eq!(it.next(), Some((3, 1.0, 0.5)));
        assert_eq!(it.next(), Some((4, 1.0, 1.0)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn simple_diff() {
        let a = ECDF::from(vec![1, 2, 3, 4]);
        let b = ECDF::from(vec![1, 3, 3, 4]);
        let c = ECDF::from(vec![4, 4, 4, 4]);
        assert_eq!(a.area_difference(&a), 0.0);
        assert_eq!(a.area_difference(&b), 0.25);
        assert_eq!(a.area_difference(&c), 1.5);

        let d = ECDF::from(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let e = ECDF::from(vec![2, 4, 6, 8]);
        assert_eq!(d.area_difference(&e), 0.5);
        assert_eq!(e.area_difference(&d), 0.5);
    }

    #[test]
    fn identity_fraction() {
        let ecdf = ECDF::from(vec![0.5, 1.0]).interpolate();
        assert_eq!(ecdf.fraction(-1.0), 0.0);
        assert_eq!(ecdf.fraction(0.0), 0.0);
        assert_eq!(ecdf.fraction(0.125), 0.125);
        assert_eq!(ecdf.fraction(0.5), 0.5);
        assert_eq!(ecdf.fraction(0.75), 0.75);
        assert_eq!(ecdf.fraction(1.0), 1.0);
        assert_eq!(ecdf.fraction(2.0), 1.0);
    }

    #[test]
    fn identity_quantile() {
        let ecdf = ECDF::from(vec![0.5, 1.0]).interpolate();
        assert_eq!(ecdf.quantile(0.0), 0.0);
        assert_eq!(ecdf.quantile(0.125), 0.125);
        assert_eq!(ecdf.quantile(0.25), 0.25);
        assert_eq!(ecdf.quantile(0.5), 0.5);
        assert_eq!(ecdf.quantile(0.75), 0.75);
        assert_eq!(ecdf.quantile(1.0), 1.0);
    }

    #[test]
    fn bad_quantile_inputs() {
        let empty = ECDF::<f64>::default().interpolate();
        assert!(empty.quantile(0.5).is_nan());

        let one = ECDF::from(vec![1.0]).interpolate();
        assert!(one.quantile(0.75).is_nan()); // Not enough samples

        let two = ECDF::from(vec![1.0, 2.0]).interpolate();
        assert_eq!(two.quantile(0.75), 1.5);

        let ecdf = ECDF::from(vec![1.0, 2.0, 3.0, 4.0]).interpolate();
        assert!(ecdf.quantile(f64::nan()).is_nan());
        assert_eq!(ecdf.quantile(-0.5), f64::neg_infinity());
        assert_eq!(ecdf.quantile(0.75), 3.0);
        assert_eq!(ecdf.quantile(2.0), f64::infinity());
    }

    #[test]
    fn merge_interpolated() {
        let a = ECDF::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]).interpolate();
        let b = ECDF::from(vec![8.0, 8.0, 9.0]).interpolate();
        let c = a.merge(&b);
        assert_eq!(a.len() + b.len(), c.len());
        assert_eq!(
            c.samples.as_slice(),
            &[
                (0.0, 1.0),
                (1.0, 1.25),
                (2.0, 1.25),
                (3.0, 1.25),
                (4.0, 1.25),
                (8.0, 1.0),
                (9.0, 1.0),
            ]
        );
    }

    #[test]
    fn interpolated_area() {
        let a = ECDF::from(vec![1.0, 2.0]).interpolate();
        let b = ECDF::from(vec![0.5, 1.0, 2.0, 3.0]).interpolate();
        assert_eq!(a.area_difference(&a), 0.0);
        assert_eq!(b.area_difference(&b), 0.0);

        // a    = (0.5, 0.00) (1.0, 0.5) (2.0, 1.00) (3.0, 1.0)
        // b    = (0.5, 0.25) (1.0, 0.5) (2.0, 0.75) (3.0, 1.0)
        // ----------------------------------------------------
        // diff = (0.5, 0.25) (1.0, 0.0) (2.0, 0.25) (3.0, 0.0)
        //
        // This makes two triangles:
        //   0.5..1.0 : w = 0.5, h = 0.25, area = 0.0625
        //   1.0..2.0 : w = 1.0, h = 0.25, area = 0.1250
        //   2.0..3.0 : w = 1.0, h = 0.25, area = 0.1250
        //                                 -------------
        //                                        0.3125
        assert_eq!(a.area_difference(&b), 0.3125);
    }

    #[test]
    fn area_of_crossing_lines() {
        // Creates two interpolated ECDFs that cross over each other more than
        // once. Each parallelogram will have a height of 1/3, and a width of 3.
        //                     A __________
        //                      /     /
        //             B ______/_____/ B
        //              /     /
        //     A ______/_____/ A
        //      /     /
        // ____/_____/ B
        //
        // X:  0 1 2 3 4 5 6 7 8 9 0 1 2 3
        let a = InterpolatedECDF {
            samples: vec![(0.0, 0.0), (1.0, 1.0), (7.0, 0.0), (9.0, 2.0)],
        };
        let b = InterpolatedECDF {
            samples: vec![(3.0, 0.0), (5.0, 2.0), (11.0, 0.0), (12.0, 1.0)],
        };
        // let points = &[0.0, 1.0, 3.0, 4.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0];
        // println!("A = {:?}", a.interpolate_counts(points.iter().cloned()));
        // println!("B = {:?}", b.interpolate_counts(points.iter().cloned()));
        assert!((a.area_difference(&b) - 3.0).abs() < 1e-10);
        assert!((b.area_difference(&a) - 3.0).abs() < 1e-10);
    }
}
