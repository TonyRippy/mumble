// Breaks input file up by time period.
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

#[macro_use]
extern crate log;

use clap::Parser;
use csvlib::Value;
use env_logger::Env;
use std::{fs::File, io::Error};

#[derive(Parser)]
struct Cli {
    /// The path to the input data.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    input_path: String,

    /// Amount of time covered by each partition, in seconds.
    #[arg(short, long, default_value_t = 1)]
    interval: u64,

    /// Path to where the partitioned files should be written.
    #[arg(short, long, default_value = ".", value_hint = clap::ValueHint::FilePath) ]
    output_path: String,
}

impl Cli {
    fn create_file(&self, timestamp: u64) -> Result<File, Error> {
        let path = format!("{}/{}.csv", self.output_path, timestamp);
        File::create(path)
    }
}

fn main() {
    // Parse command-line arguments
    let args = Cli::parse();
    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut start = 0u64;
    let mut end = start + args.interval;
    let mut partition: Vec<Value> = Vec::new();

    let reader = csvlib::open_gzip_or_regular_file(&args.input_path).expect("open input file");
    for v in csvlib::read_values(reader) {
        let t = v.timestamp_secs as u64;
        if t < start {
            warn!("input is not sorted; {} comes before {}", t, start);
            continue;
        }
        if t >= end {
            if !partition.is_empty() {
                let f = args.create_file(end).expect("create output file");
                csvlib::write_values(f, &partition).expect("write values");
                partition.clear();
            }
            start = t - (t % args.interval);
            end = start + args.interval;
        }
        partition.push(v);
    }
    if !partition.is_empty() {
        let f = args.create_file(end).expect("create output file");
        csvlib::write_values(f, &partition).expect("write values");
    }
}
