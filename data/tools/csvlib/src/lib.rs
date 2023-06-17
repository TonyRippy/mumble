// Utilities for reading and writing CSV files in a known format.
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

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, Error, Read, Write},
};

/// A record used to store a single time series.
#[derive(Debug, Serialize, Deserialize)]
pub struct Value {
    pub timestamp_secs: i64,
    pub timestamp_nanos: i32,
    pub value: f64,
}

impl AsRef<Value> for Value {
    fn as_ref(&self) -> &Self {
        self
    }
}

/// A record used to store an ECDF.
#[derive(Debug, Serialize, Deserialize)]
pub struct Fraction {
    pub value: f64,
    pub fraction: f64,
}

impl AsRef<Fraction> for Fraction {
    fn as_ref(&self) -> &Self {
        self
    }
}

/// Opens a file for reading, automatically decompressing it if it ends in ".gz".
pub fn open_gzip_or_regular_file(path: &str) -> Result<BufReader<Box<dyn Read>>, Error> {
    let f = File::open(path)?;
    Ok(if path.ends_with(".gz") {
        BufReader::new(Box::new(GzDecoder::new(f)))
    } else {
        BufReader::new(Box::new(f))
    })
}

/// Reads a time series samples from a CSV file.
pub fn read_values<R: Read>(reader: R) -> Vec<Value> {
    csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader)
        .deserialize::<Value>()
        .filter_map(|r| {
            if let Ok(v) = r {
                Some(v)
            } else {
                warn!("{:?}", r.unwrap_err());
                None
            }
        })
        .collect()
}

/// Writes time series samples to a CSV file.
pub fn write_values<W, I, V>(writer: W, values: I) -> Result<(), Error>
where
    W: Write,
    V: AsRef<Value>,
    I: IntoIterator<Item = V>,
{
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(writer);
    for v in values {
        writer.serialize(v.as_ref())?;
    }
    writer.flush()?;
    Ok(())
}

/// Writes points from an ECDF to a CSV file.
pub fn write_fractions<W, I, V>(writer: W, fractions: I) -> Result<(), Error>
where
    W: Write,
    V: AsRef<Fraction>,
    I: IntoIterator<Item = V>,
{
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(writer);
    for f in fractions {
        writer.serialize(f.as_ref())?;
    }
    writer.flush()?;
    Ok(())
}
