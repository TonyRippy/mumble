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

use http::{Request, Response, StatusCode};
use std::convert::Infallible;

const INDEX_HTML: &str = include_str!("../ui/dist/index.html");
const INDEX_JS: &str = include_str!("../ui/dist/main.min.js");

pub async fn serve<R>(req: Request<R>) -> Result<Response<String>, Infallible> {
    let mut response = Response::default();
    match req.uri().query() {
        None => {
            *response.body_mut() = INDEX_HTML.to_string();
        }
        Some(qs) => {
            if qs == "js" {
                *response.body_mut() = INDEX_JS.to_string();
            } else {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }
    };
    Ok(response)
}
