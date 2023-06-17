// Collects histogram samples and clusters them for efficient storage.
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

#[macro_use]
extern crate log;

mod clustering;

use crate::clustering::DataStore;
use chrono::{DateTime, Duration, TimeZone, Utc};
use clap::Parser;
use env_logger::Env;
use mumble_prometheus::{histogram_to_ecdf, parse_histogram};
use std::{fmt::Debug, process::ExitCode};

#[derive(Clone, Debug)]
pub struct Id {
    pub timestamp: String,
    pub label_set_id: i64,
}

#[derive(Parser)]
struct Cli {
    /// The path to a SQLite3 database with denormalized samples.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    input_database: String,

    /// The path to the SQLite3 database where normalized data should be written.
    #[arg(value_hint = clap::ValueHint::FilePath)]
    output_database: String,

    /// Minimum distance between samples in a cluster.
    #[arg(short, long, default_value_t = 1.0)]
    eps: f64,
}

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%:z";

fn parse_timestamp(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_str(s, TIMESTAMP_FORMAT)
        .expect("parse timestamp")
        .with_timezone(&Utc)
}

fn round_up(dt: DateTime<Utc>, period: Duration) -> DateTime<Utc> {
    let seconds = dt.timestamp();
    let period = period.num_seconds();
    Utc.timestamp_opt(seconds + period, 0).unwrap()
}

fn main() -> ExitCode {
    // Parse command-line arguments
    let args = Cli::parse();

    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Break per-second samples up unto 1 minute batches.
    let batch_size = chrono::Duration::minutes(30);

    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut batch_end = DateTime::<Utc>::MIN_UTC;

    // Open the input database
    let input_connection =
        sqlite::open(/*&args.*/ &args.input_database).expect("open input database");

    let query = "SELECT * FROM monitoring_data ORDER BY timestamp ASC;";
    for row in input_connection
        .prepare(query)
        .expect("prepare input query")
        .iter()
        .map(|row| row.expect("read input row"))
    {
        let id = Id {
            timestamp: row.read::<&str, _>(0).to_string(),
            label_set_id: row.read::<i64, _>(1),
        };
        let data = row.read::<&[u8], _>(2);

        let ecdf = histogram_to_ecdf(&parse_histogram(data).expect("deserialize histogram"));

        let t = parse_timestamp(&id.timestamp);
        if t >= batch_end {
            if !batch.is_empty() {
                batches.push(batch);
            }
            batch = Vec::new();
            batch_end = round_up(t, batch_size);
        }
        batch.push((id, ecdf));
    }
    // Don't forget to add the last batch!
    batches.push(batch);

    let mut ds = DataStore::open(&args.output_database, args.eps).expect("open data store");
    for batch in batches {
        ds.process_batch(batch);
    }

    ExitCode::SUCCESS
}
