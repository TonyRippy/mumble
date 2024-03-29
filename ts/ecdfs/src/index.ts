// A typescript library for loading and manipulating ECDFs.
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

export {
  type Func,
  linearFunction
} from './func'

export type {
  CDF
} from './cdf'

export {
  Normal,
  LogNormal
} from './gaussian'

export {
  ECDF,
  fromJSON,
  toJSON
} from './ecdf'

export {
  Layer,
  CDFGraph,
  getRenderFuncForPlot,
  getRenderFuncForFunction,
  resizeCanvasToDisplaySize,
  createGraph
} from './plot'

export {
  KSTest
} from './kolmogorov'
