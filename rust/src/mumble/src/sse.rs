// This code is forked from https://github.com/klemens/hyper-sse
// which was dual licensed under the MIT and Apache 2.0 Licenses.
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
use futures::channel::mpsc::{Receiver, Sender, TrySendError};
use http::{Request, Response};
use http_body::Frame;
use http_body_util::StreamBody;
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// TODO: Create an init event that sends a client id.
// TODO: persistent event queues that can be replayed when new clients connect.

type Chunk = Result<Frame<Bytes>, Infallible>;
type Clients = Vec<Client>;

/// Push server implementing Server-Sent Events (SSE).
pub struct Server {
    clients: Mutex<Clients>,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            clients: Mutex::new(Vec::new()),
        }
    }
}

impl Server {
    /// Push a message for the event to all clients.
    ///
    /// The message is first serialized and then send to all registered
    /// clients on the given channel, if any.
    ///
    /// Returns an error if the serialization fails.
    pub fn push<S: Serialize>(
        &self,
        event: &str,
        message: &S,
    ) -> Result<(), serde_json::error::Error> {
        let payload = serde_json::to_string(message)?;
        let message = format!("event: {}\ndata: {}\n\n", event, payload);
        self.send_event(message);
        Ok(())
    }

    /// Initiate a new SSE stream for the given request.
    pub fn create_stream<R>(
        &self,
        _request: Request<R>,
    ) -> http::Result<Response<StreamBody<Receiver<Chunk>>>> {
        let (tx, rx) = futures::channel::mpsc::channel(100);
        let mut client = Client {
            tx,
            first_error: None,
        };

        // TODO: Send target information
        let message = format!("event: {}\ndata: {}\n\n", "target", "{}");
        client
            .try_send_chunk(message)
            .expect("Unable to send initial target event!");

        self.clients.lock().unwrap().push(client);

        Response::builder()
            .header("Cache-Control", "no-cache")
            .header("X-Accel-Buffering", "no")
            .header("Content-Type", "text/event-stream")
            .header("Access-Control-Allow-Origin", "*")
            .body(StreamBody::new(rx))
    }

    /// Send hearbeat to all clients.
    ///
    /// This should be called regularly (e.g. every 15 minutes) to detect
    /// a disconnect of the underlying TCP connection.
    pub fn send_heartbeats(&self) {
        self.send_chunk(":\n\n".into());
    }

    /// Remove disconnected clients.
    ///
    /// This removes all clients from all channels that have closed the
    /// connection or are not responding to the heartbeats, which caused
    /// a TCP timeout.
    ///
    /// This function should be called regularly (e.g. together with
    /// `send_heartbeats`) to keep the memory usage low.
    pub fn remove_stale_clients(&self) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|client| {
            if let Some(first_error) = client.first_error {
                if first_error.elapsed() > Duration::from_secs(5) {
                    dbg!("Removing stale client");
                    return false;
                }
            }
            true
        });
    }

    /// Send a given event to all clients.
    fn send_event(&self, chunk: String) {
        debug!("Sending: {}", chunk);
        let mut clients = self.clients.lock().unwrap();
        for client in clients.iter_mut() {
            if let Err(e) = client.try_send_event(chunk.clone()) {
                error!("Unable to send event to client: {}", e);
            }
        }
    }
}

#[derive(Debug)]
struct Client {
    tx: Sender<Chunk>,
    first_error: Option<Instant>,
}

// TODO: Figure out how to implement a blocking send

impl Client {
    fn try_send_event(&mut self, chunk: String) -> Result<(), TrySendError<Chunk>> {
        let result = self.tx.try_send(Ok(Frame::data(Bytes::from(chunk))));
        match (&result, self.first_error) {
            (Err(_), None) => {
                // Store time when an error was first seen
                self.first_error = Some(Instant::now());
            }
            (Ok(_), Some(_)) => {
                // Clear error when write succeeds
                self.first_error = None;
            }
            _ => {}
        }
        result
    }
}
