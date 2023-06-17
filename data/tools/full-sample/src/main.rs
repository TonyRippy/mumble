// Aggregates values into a full-resolution CDF.
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

use std::ops::Deref;

use chrono::{Local, TimeZone};
use clap::Parser;
use env_logger::Env;
use mumble::ecdf::ECDF;

#[derive(Parser)]
struct Cli {
    /// The path to the input data.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    input_path: String,

    /// The UNIX timestamp of the sample, in seconds since the epoch.
    #[arg(short, long, default_value_t = 0)]
    timestamp: i64,

    /// The path to the SQLite3 database where the full sample should be written.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    output_database: String,
}

fn main() {
    // Parse command-line arguments
    let args = Cli::parse();
    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let t = Local.timestamp_opt(args.timestamp, 0).unwrap();
    let tstr = t.format("%Y-%m-%d %H:%M:%S%:z").to_string();

    let reader = csvlib::open_gzip_or_regular_file(&args.input_path).expect("open input file");
    let values = csvlib::read_values(reader)
        .into_iter()
        .map(|v| v.value)
        .collect::<Vec<f64>>();
    let ecdf = ECDF::from(values);
    let rmp = rmp_serde::to_vec(&ecdf).unwrap();

    // Open the input database
    let connection = sqlite::open(/*&args.*/ args.output_database).expect("open output database");
    let mut statement = connection
        .prepare("INSERT INTO [full_sample] (timestamp, data) VALUES (?, ?)")
        .unwrap();
    statement.bind((1, tstr.as_str())).expect("bind timestamp");
    statement.bind((2, rmp.deref())).expect("bind data");
    statement.next().expect("insert");
}
