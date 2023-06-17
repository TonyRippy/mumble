// Helper functions for reading Prometheus data.
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

mod histogram;
mod protos;

use crate::histogram::get_bound;
use protos::metrics::BucketSpan;
pub use protos::metrics::Histogram;

use mumble::ecdf::{InterpolatedECDF, ECDF};

use protobuf::Message;

fn positive_counts(spans: &Vec<BucketSpan>, deltas: &Vec<i64>, schema: i32) -> Vec<(f64, usize)> {
    let mut out = Vec::with_capacity(deltas.len() + spans.len());

    let mut last_schema_idx: i32 = 0;
    let mut bucket_idx: usize = 0;
    let mut bucket_sum: i64 = 0;
    for span in spans.iter() {
        let start_schema_idx = last_schema_idx + span.offset();
        let end_schema_idx = start_schema_idx + span.length() as i32;
        last_schema_idx = end_schema_idx;

        out.push((get_bound(start_schema_idx - 1, schema), 0));
        for schema_idx in start_schema_idx..end_schema_idx {
            bucket_sum += deltas[bucket_idx];
            bucket_idx += 1;
            out.push((get_bound(schema_idx, schema), bucket_sum as usize));
        }
    }
    out
}

fn negative_counts(spans: &Vec<BucketSpan>, deltas: &Vec<i64>, schema: i32) -> Vec<(f64, usize)> {
    let mut last_schema_idx: i32 = 0;
    let mut last_bucket_idx: usize = 0;
    for span in spans.iter() {
        last_schema_idx += span.offset() + span.length() as i32;
        last_bucket_idx += span.length() as usize;
    }
    assert_eq!(last_bucket_idx, deltas.len());
    let mut bucket_sum: i64 = deltas.iter().sum();

    let mut out = Vec::with_capacity(deltas.len() + spans.len());

    for span in spans.iter().rev() {
        let end_bucket_idx = last_bucket_idx;
        let start_bucket_idx = end_bucket_idx - span.length() as usize;
        last_bucket_idx = start_bucket_idx;

        let end_schema_idx = last_schema_idx;
        let start_schema_idx = end_schema_idx - span.length() as i32;
        last_schema_idx = end_schema_idx - span.offset();

        out.push((-get_bound(end_schema_idx, schema), 0));

        for (buckets_idx, schema_idx) in (start_bucket_idx..end_bucket_idx)
            .rev()
            .zip((start_schema_idx..end_schema_idx).rev())
        {
            out.push((-get_bound(schema_idx, schema), bucket_sum as usize));
            bucket_sum -= deltas[buckets_idx];
        }
    }
    out
}

pub fn parse_histogram(data: &[u8]) -> Result<Histogram, protobuf::Error> {
    let mut h = Histogram::new();
    h.merge_from_bytes(data)?;
    Ok(h)
}

pub fn histogram_to_ecdf(h: &Histogram) -> InterpolatedECDF<f64> {
    // Sanity check the deserialized histogram.
    assert!(h.bucket.is_empty());
    assert!(h.positive_count.is_empty());
    assert!(h.negative_count.is_empty());

    let positive_counts = positive_counts(&h.positive_span, &h.positive_delta, h.schema());
    let mut negative_counts = negative_counts(&h.negative_span, &h.negative_delta, h.schema());
    let zero_count = (h.zero_threshold(), h.zero_count() as usize);

    // Adjust the bounds of the last negative bucket to avoid overlap with the zero bucket.
    if !negative_counts.is_empty() {
        let first_neg = negative_counts.len() - 1;
        if negative_counts[first_neg].0 < -h.zero_threshold() {
            // We need to adjust the bounds of the last negative bucket because it overlaps with the zero bucket.
            let count = negative_counts[first_neg].1;
            negative_counts[first_neg] = (-h.zero_threshold(), count);
        }
    }

    let mut ecdf = ECDF::default();
    ecdf.merge_sorted(
        negative_counts
            .into_iter()
            .chain(std::iter::once(zero_count))
            .chain(positive_counts.into_iter()),
    );
    ecdf.interpolate()
}
