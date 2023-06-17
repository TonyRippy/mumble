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

use bytes::Bytes;
use futures::channel::mpsc::Receiver;
use http::{Request, Response, StatusCode};
use http_body::{Body, Frame};
use http_body_util::StreamBody;
use serde::Serialize;
use std::convert::Infallible;
use std::time::Duration;

const INDEX_HTML: &[u8] = include_bytes!("../ui/dist/index.html");
const INDEX_JS: &[u8] = include_bytes!("../ui/dist/main.min.js");

pub const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(15);

lazy_static! {
    static ref PUSH_SERVER: crate::sse::Server = crate::sse::Server::default();
}

type Chunk = Result<Frame<Bytes>, Infallible>;

fn oneshot_send(data: Bytes) -> StreamBody<Receiver<Chunk>> {
    let (mut tx, rx) = futures::channel::mpsc::channel::<Chunk>(0);
    tx.try_send(Ok(Frame::data(data)))
        .expect("Failed to send oneshot data.");
    StreamBody::new(rx)
}

// TODO: Box<dyn Body>

pub async fn serve<R>(
    req: Request<R>,
) -> http::Result<Response<impl Body<Data = Bytes, Error = Infallible>>> {
    match req.uri().path() {
        "/" => Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .status(StatusCode::OK)
            .body(oneshot_send(Bytes::from_static(INDEX_HTML))),
        "/js" => Response::builder()
            .header("Content-Type", "text/javascript; charset=utf-8")
            .status(StatusCode::OK)
            .body(oneshot_send(Bytes::from_static(INDEX_JS))),
        "/push" => PUSH_SERVER.create_stream("push", req),
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(oneshot_send(Bytes::default())),
    }
}

pub fn push<S: Serialize>(
    event: &str,
    message: &S,
    permanent: bool,
) -> Result<(), serde_json::error::Error> {
    PUSH_SERVER.push("push", event, message, permanent)
}

pub fn perform_maintenance() {
    PUSH_SERVER.perform_maintenance();
}
