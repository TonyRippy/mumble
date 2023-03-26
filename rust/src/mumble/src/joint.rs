// Tools for measuring joint probability of random variables.
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

use crate::mesh::{Mesh, Point};

pub struct JointECDF {
    mesh: Mesh<f64, f64>,
}

impl JointECDF {
    pub fn builder() -> Builder {
        Builder {
            samples: Vec::new(),
            total: 0,
        }
    }

    /// Returns the probability distribution for `B` given that `A` is less than or equal to `a`.
    pub fn given_a<A, B>(&self, a: A) -> &impl Fn(B) -> f64
    where
        A: Into<f64>,
        B: Into<f64>,
    {
        &|_| 0.0
    }

    /// Returns the probability distribution `A` given an observed value `b`.
    pub fn given_b<A, B>(&self, b: B) -> &impl Fn(A) -> f64
    where
        A: Into<f64>,
        B: Into<f64>,
    {
        &|_| 0.0
    }
}

pub struct Builder {
    samples: Vec<(Point<f64>, usize)>,
    total: usize,
}

impl Builder {
    fn add_n(&mut self, sample: Point<f64>, count: usize) {
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
        self.total += count;
    }

    pub fn add<A, B>(&mut self, a: A, b: B)
    where
        A: Into<f64>,
        B: Into<f64>,
    {
        let p = Point::new(a.into(), b.into());
        self.add_n(p, 1)
    }

    pub fn build(self) -> JointECDF {
        let t = self.total as f64;
        let mut m = Mesh::default();
        for (p, v) in self.samples.into_iter() {
            m = m.add_vertex(p, (v as f64) / t);
        }
        JointECDF { mesh: m }
    }
}
