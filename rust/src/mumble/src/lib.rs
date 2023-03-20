// A client library for publishing monitoring data.
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
extern crate lazy_static;

#[macro_use]
extern crate log;

pub mod ecdf;
mod kstest;
mod sse;
pub mod ui;

use ecdf::ECDF;
use num_traits::{Num, ToPrimitive};
use serde::Serialize;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    marker::{self, PhantomData},
    time::{SystemTime, UNIX_EPOCH},
};

// Open Telemetry SDK Specification:
// https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/metrics/sdk.md

pub enum AttributeValue {
    String(String),
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> AttributeValue {
        AttributeValue::String(value.to_string())
    }
}

impl Serialize for AttributeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AttributeValue::String(v) => v.serialize(serializer),
        }
    }
}

// TODO: Should this instead be an array of values that map to known attributes?
pub type Attributes = HashMap<String, AttributeValue>;

/// A compound key that defines a namespace for [Instruments].
#[derive(Clone, Eq, Hash, PartialEq, Serialize)]
struct InstrumentationScope {
    name: String,
    version: Option<String>,
    schema_url: Option<String>,
}

/// An implementation of Open Telemetry's MeterProvider.
///
/// For more information, see the
///[Open Telemetry specification](https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/metrics/api.md#meterprovider).
#[derive(Default)]
pub struct MeterProvider {
    map: HashMap<InstrumentationScope, Meter>,
}

impl MeterProvider {
    pub fn get_meter(
        &mut self,
        name: &str,
        version: Option<String>,
        schema_url: Option<String>,
        attributes: Option<Attributes>,
    ) -> &mut Meter {
        let key = InstrumentationScope {
            name: name.to_string(),
            version,
            schema_url,
        };
        match self.map.entry(key) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let key = v.key().clone();
                v.insert(Meter {
                    key,
                    attributes: match attributes {
                        Some(attr) => attr,
                        None => Attributes::default(),
                    },
                })
            }
        }
    }
}

/// An implementation of Open Telemetry's Meter.
///
/// For more information, see the
/// [Open Telemetry specification](https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/metrics/api.md#meter).
pub struct Meter {
    key: InstrumentationScope,
    attributes: Attributes,
    // streams: HashMap<StreamKey, Sender>,
}

impl Meter {
    pub fn name(&self) -> &str {
        &self.key.name
    }

    pub fn version(&self) -> Option<&str> {
        self.key.version.as_deref()
    }

    pub fn schema_url(&self) -> Option<&str> {
        self.key.schema_url.as_deref()
    }

    pub fn create_histogram<'a, T>(&'a mut self, name: &str) -> HistogramBuilder<T>
    where
        T: Num + ToPrimitive + PartialOrd + Copy + Debug + Default,
    {
        HistogramBuilder::<'a, T> {
            meter: self,
            name: name.to_string(),
            description: None,
            attributes: Attributes::default(),
            _marker: PhantomData,
        }
    }
}

pub trait Instrument {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn push(&mut self, timestamp: u128);
}

#[derive(Serialize)]
struct Measurement<'a, T: Serialize> {
    timestamp: u128,
    name: &'a str,
    attributes: &'a Attributes,
    value: &'a T,
}

/*
pub trait Histogram: Instrument {
    type Item;
    fn record(&mut self, value: Self::Item, labels: Option<&Attributes>);
}

pub trait HistogramBuilder {
    type Impl;
    fn set_description(self, description: &str) -> Self;
    fn build(self) -> Self::Impl;
}
 */

pub struct HistogramBuilder<'a, T> {
    meter: &'a mut Meter,
    name: String,
    description: Option<String>,
    attributes: Attributes,
    _marker: marker::PhantomData<T>,
}

impl<'a, T> HistogramBuilder<'a, T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug + Default,
{
    pub fn set_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn add_attribute(mut self, name: &str, value: AttributeValue) -> Self {
        self.attributes.insert(name.to_string(), value);
        self
    }

    pub fn build(self) -> Histogram<T> {
        Histogram::<T> {
            name: self.name,
            description: self.description,
            attributes: self.attributes,
            ecdf: ECDF::default(),
        }
    }
}

pub struct Histogram<T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug,
{
    name: String,
    description: Option<String>,
    attributes: Attributes,
    ecdf: ECDF<T>,
}

/// Returns the current time, in a format appropriate for reporting.
pub fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

impl<T> Instrument for Histogram<T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug + Serialize,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn push(&mut self, timestamp: u128) {
        if self.ecdf.is_empty() {
            // Nothing to do...
            return;
        }
        ui::push(
            "update",
            &Measurement::<ECDF<T>> {
                timestamp,
                name: &self.name,
                attributes: &self.attributes,
                value: &self.ecdf,
            },
        );
        self.ecdf.clear();
    }
}

impl<T> Histogram<T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug + Default,
{
    pub fn record(&mut self, value: T) {
        self.ecdf.add(value)
    }
}
