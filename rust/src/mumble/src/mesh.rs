// Routines for interpolating between a 2D mesh of samples.
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

// TODO: Mesh does a lot of copying. It should be possible to avoid this using scoped references.

use derivative::Derivative;
use num_traits::Float;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct Point<P> {
    pub x: P,
    pub y: P,
}

struct Circumcircle<P> {
    x: P,
    y: P,
    rr: P,
}

impl<P> Circumcircle<P>
where
    P: Float,
{
    fn new(a: &Point<P>, b: &Point<P>, c: &Point<P>) -> Circumcircle<P> {
        // Calculate the circumcircle
        // https://en.wikipedia.org/wiki/Circumscribed_circle
        // This uses the simplified formula described in the Wikipedia article above.
        // It translates the vertices so that A = (0,0).
        let bx = b.x - a.x;
        let by = b.y - a.y;
        let b2 = bx * bx + by * by;
        let cx = c.x - a.x;
        let cy = c.y - a.y;
        let c2 = cx * cx + cy * cy;
        let mut ux = cy * b2 - by * c2;
        let mut uy = bx * c2 - cx * b2;
        let mut d = bx * cy - by * cx;
        d = d + d;
        ux = ux / d;
        uy = uy / d;

        Circumcircle {
            x: ux + a.x,
            y: uy + a.y,
            rr: ux * ux + uy * uy,
        }
    }

    fn contains(&self, p: &Point<P>) -> bool {
        let mut dx = p.x - self.x;
        dx = dx * dx;
        let mut dy = p.y - self.y;
        dy = dy * dy;
        let rr = dx + dy;
        rr <= self.rr
    }
}

#[derive(Clone, Copy, Derivative)]
#[derivative(PartialEq, PartialOrd)]
struct Vertex<P, V>
where
    P: Copy + PartialEq + PartialOrd,
    V: Copy,
{
    p: Point<P>,
    #[derivative(PartialEq = "ignore", PartialOrd = "ignore")]
    v: V,
}

struct Edge<P, V>
where
    P: Copy + PartialEq + PartialOrd,
    V: Copy,
{
    a: Vertex<P, V>,
    b: Vertex<P, V>,
}

impl<P, V> Edge<P, V>
where
    P: Copy + PartialEq + PartialOrd,
    V: Copy,
{
    fn new(v1: Vertex<P, V>, v2: Vertex<P, V>) -> Edge<P, V> {
        if v2 < v1 {
            Edge { a: v2, b: v1 }
        } else {
            Edge { a: v1, b: v2 }
        }
    }
}

impl<P, V> PartialEq<Edge<P, V>> for Edge<P, V>
where
    P: Copy + PartialEq + PartialOrd,
    V: Copy,
{
    fn eq(&self, other: &Edge<P, V>) -> bool {
        self.a == other.a && self.b == other.b
    }
}

pub struct Triangle<P, V>
where
    P: Copy + PartialEq + PartialOrd,
    V: Copy,
{
    v1: Vertex<P, V>,
    v2: Vertex<P, V>,
    v3: Vertex<P, V>,
    cc: Circumcircle<P>,
}

impl<P, V> Triangle<P, V>
where
    P: Float,
    V: Copy,
{
    fn new(v1: Vertex<P, V>, v2: Vertex<P, V>, v3: Vertex<P, V>) -> Triangle<P, V> {
        let cc = Circumcircle::new(&v1.p, &v2.p, &v3.p);
        Triangle { v1, v2, v3, cc }
    }

    fn edges(&self) -> Vec<Edge<P, V>> {
        vec![
            Edge::new(self.v1, self.v2),
            Edge::new(self.v2, self.v3),
            Edge::new(self.v3, self.v1),
        ]
    }

    fn weights(&self, p: &Point<P>) -> (P, P, P) {
        // https://en.wikipedia.org/wiki/Barycentric_coordinate_system
        //
        // for three vertices Vn = {Xn, Yn}
        // And a point P = {x, y}
        //
        //       (Y2-Y3)(x-X3) + (X3-X2)(y-Y3)
        // W1 = -------------------------------
        //      (Y2-Y3)(X1-X3) + (X3-X2)(Y1-Y3)
        //
        //       (Y3-Y1)(x-X3) + (X1-X3)(y-Y3)
        // W2 = -------------------------------
        //      (Y2-Y3)(X1-X3) + (X3-X2)(Y1-Y3)
        //
        // W3 = 1 - W1 - W2
        //
        let x1 = self.v1.p.x;
        let y1 = self.v1.p.y;
        let x3 = self.v3.p.x;
        let y3 = self.v3.p.y;
        let mut w2 = (y3 - y1) * (p.x - x3) + (x1 - x3) * (p.y - y3);

        let dy23 = self.v2.p.y - y3;
        let dx32 = x3 - self.v2.p.x;
        let mut w1 = dy23 * (p.x - x3) + dx32 * (p.y - y3);

        let denom = dy23 * (x1 - x3) + dx32 * (y1 - y3);
        w1 = w1 / denom;
        w2 = w2 / denom;

        (w1, w2, P::one() - w1 - w2)
    }

    pub fn contains(&self, p: &Point<P>) -> bool {
        let (w1, w2, w3) = self.weights(p);
        w1 >= P::zero() && w2 >= P::zero() && w3 >= P::zero()
    }

    pub fn interpolate<F>(&self, p: &Point<P>, calc: F) -> F::Output
    where
        F: Fn((V, V, V), (P, P, P)),
    {
        calc((self.v1.v, self.v2.v, self.v3.v), self.weights(p))
    }
}

#[derive(Default)]
pub struct Mesh<P, V>
where
    P: Copy + PartialEq + PartialOrd,
    V: Copy,
{
    ts: Vec<Triangle<P, V>>,
}

impl<P, V> Mesh<P, V>
where
    P: Float,
    V: Copy,
{
    pub fn add_vertex(self, p: Point<P>, value: V) -> Mesh<P, V> {
        let v = Vertex { p, v: value };

        // Building a mesh:
        // https://en.wikipedia.org/wiki/Delaunay_triangulation
        // https://en.wikipedia.org/wiki/Bowyer%E2%80%93Watson_algorithm
        let mut bad_ts: Vec<Triangle<P, V>> = Vec::new();
        let mut good_ts: Vec<Triangle<P, V>> = Vec::new();
        // Loop through each triangle in current triangulation:
        for t in self.ts.into_iter() {
            // First find all the triangles that are no longer valid due
            // to the insertion.
            if t.cc.contains(&v.p) {
                bad_ts.push(t);
            } else {
                good_ts.push(t);
            }
        }
        let mut polygon: Vec<Edge<P, V>> = Vec::new();
        for t in bad_ts.into_iter() {
            // Find the boundary of the polygonal hole
            for e in t.edges().into_iter() {
                // if edge is not shared by any other triangles in badTriangles
                // add edge to polygon
                if !polygon.contains(&e) {
                    polygon.push(e);
                }
            }
        }
        // re-triangulate the polygonal hole
        for e in polygon.into_iter() {
            let t = Triangle::new(e.a, e.b, v);
            good_ts.push(t);
        }
        Mesh { ts: good_ts }
    }

    pub fn find(&self, p: &Point<P>) -> Option<&Triangle<P, V>> {
        for t in self.ts.iter() {
            if t.cc.contains(p) && t.contains(p) {
                return Some(t);
            }
        }
        None
    }
}
