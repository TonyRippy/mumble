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

use num_traits::{Num, ToPrimitive};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    marker::{self, PhantomData},
};

// Open Telemetry SDK Specification:
// https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/metrics/sdk.md

// TODO: Consider making this an enum that supports typed attributes.
pub type AttributeValue = String;

// TODO: Should this instead be an array of values that map to known attributes?
pub type Attributes = HashMap<String, AttributeValue>;

// TODO: Rename to InstrumentationScope for compatibility with OTel SDK?
type MeterKey = (String, Option<String>, Option<String>);

/// The key used to identify a single stream of measurements.
type StreamKey = (String, Option<Attributes>);

/// An implementation of Open Telemetry's MeterProvider.
///
/// For more information, see the
///[Open Telemetry specification](https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/metrics/api.md#meterprovider).
#[derive(Default)]
pub struct MeterProvider {
    map: HashMap<MeterKey, Meter>,
}

impl MeterProvider {
    pub fn get_meter(
        &mut self,
        name: &str,
        version: Option<String>,
        schema_url: Option<String>,
        attributes: Option<Attributes>,
    ) -> &mut Meter {
        let key: MeterKey = (name.to_string(), version, schema_url);
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
    key: MeterKey,
    attributes: Attributes,
    // streams: HashMap<StreamKey, Sender>,
}

impl Meter {
    pub fn name(&self) -> &str {
        self.key.0.as_str()
    }

    pub fn version(&self) -> Option<&str> {
        self.key.1.as_deref()
    }

    pub fn schema_url(&self) -> Option<&str> {
        self.key.2.as_deref()
    }

    pub fn create_histogram<'a, T>(&'a mut self, name: &str) -> HistogramBuilder<T>
    where
        T: Num + ToPrimitive + PartialOrd + Copy + Debug + Default,
    {
        HistogramBuilder::<'a, T> {
            meter: self,
            name: name.to_string(),
            description: None,
            _marker: PhantomData,
        }
    }
}

pub trait Instrument {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn push(&self);
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

    pub fn build(self) -> Histogram<T> {
        Histogram::<T> {
            name: self.name,
            description: self.description,
            ecdf: ecdf::ECDF::<T>::default(),
        }
    }
}

pub struct Histogram<T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug,
{
    name: String,
    description: Option<String>,
    ecdf: ecdf::ECDF<T>,
}

impl<T> Instrument for Histogram<T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug,
{
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn push(&self) {
        warn!("Push not implemented yet!");
        //ui::push("update", &update);
    }
}

impl<T> Histogram<T>
where
    T: Num + ToPrimitive + PartialOrd + Copy + Debug,
{
    pub fn record(&mut self, value: T, _labels: Option<&Attributes>) {
        self.ecdf.add(value)
    }
}
