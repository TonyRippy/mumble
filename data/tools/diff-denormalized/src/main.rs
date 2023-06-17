// Calculates statistics about the accuracy of histograms
// as compared to the underlying data.
//
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

use clap::Parser;
use env_logger::Env;
use mumble::ecdf::ECDF;
use mumble_prometheus::{histogram_to_ecdf, parse_histogram};

use std::fmt::{self, Display};

struct MinMeanMax {
    samples: Vec<f64>,
    sum: f64,
}

impl MinMeanMax {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            sum: 0.0,
        }
    }

    fn update(&mut self, x: f64) {
        self.samples.push(x);
        self.sum += x;
    }

    fn min(&self) -> f64 {
        self.samples
            .iter()
            .cloned()
            .reduce(|a, b| if b < a { b } else { a })
            .unwrap_or(0.0)
    }

    fn max(&self) -> f64 {
        self.samples
            .iter()
            .cloned()
            .reduce(|a, b| if b > a { b } else { a })
            .unwrap_or(0.0)
    }

    fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.sum / self.samples.len() as f64
    }

    fn lo_stdev(&self, mean: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut count = 0;
        for &x in self.samples.iter() {
            if x > mean {
                continue;
            }
            let diff = mean - x;
            sum += diff * diff;
            count += 1;
        }
        if count == 0 {
            return 0.0;
        }
        mean - (sum / count as f64).sqrt()
    }

    fn hi_stdev(&self, mean: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut count = 0;
        for &x in self.samples.iter() {
            if x < mean {
                continue;
            }
            let diff = x - mean;
            sum += diff * diff;
            count += 1;
        }
        if count == 0 {
            return 0.0;
        }
        mean + (sum / count as f64).sqrt()
    }
}

impl Display for MinMeanMax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mean = self.mean();
        write!(
            f,
            "{:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {}, ",
            self.min(),
            self.lo_stdev(mean),
            mean,
            self.hi_stdev(mean),
            self.max(),
            self.samples.len()
        )
    }
}

#[derive(Parser)]
struct Cli {
    /// The path to the input database.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    input_database: String,
}

fn main() {
    // Parse command-line arguments
    let args = Cli::parse();

    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut err = MinMeanMax::new();

    // Open the input database
    let connection = sqlite::open(/*&args.*/ args.input_database).expect("open output database");

    // Iterate over all samples, calculating the area difference with the histogram.
    for row in connection
        .prepare(
            "SELECT md.timestamp, f.data, md.data
            FROM monitoring_data md
            INNER JOIN full_sample f ON f.timestamp = md.timestamp;",
        )
        .expect("prepare input query")
        .iter()
        .map(|row| row.expect("read input row"))
    {
        // let timestamp = row.read::<&str, _>(0);
        let full: ECDF<f64> =
            rmp_serde::from_slice(row.read::<&[u8], _>(1)).expect("deserialize full sample");
        let h = parse_histogram(row.read::<&[u8], _>(2)).expect("parse histogram");
        let other = histogram_to_ecdf(&h);
        err.update(full.interpolate().area_difference(&other));
    }
    println!("{}", &err);
}
