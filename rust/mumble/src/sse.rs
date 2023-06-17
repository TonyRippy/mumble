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
use futures::channel::mpsc::{Receiver, Sender};
use http::{Request, Response};
use http_body::Frame;
use http_body_util::StreamBody;
use serde::Serialize;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Mutex;
use std::time::{Duration, Instant};

type Chunk = Result<Frame<Bytes>, Infallible>;

/// Push server implementing Server-Sent Events (SSE).
pub struct Server {
    channels: Mutex<HashMap<String, Channel>>,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            channels: Mutex::new(HashMap::new()),
        }
    }
}

impl Server {
    /// Push an event to all clients subscribed to a channel.
    ///
    /// `message` is first serialized as JSON and then sent to all registered
    /// clients on `channel`, if any. If `replay` is `true`, the event will
    /// be kept in memory and replayed later to any future clients when they
    /// first connect.
    ///
    /// Returns an error if the serialization fails.
    pub fn push<S: Serialize>(
        &self,
        channel: &str,
        event: &str,
        message: &S,
        replay: bool,
    ) -> Result<(), serde_json::error::Error> {
        let payload = serde_json::to_string(message)?;
        let message = format!("event: {}\ndata: {}\n\n", event, payload);
        let mut channels = self.channels.lock().unwrap();
        let c = match channels.entry(channel.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Channel::default()),
        };
        if replay {
            c.send_replayable_event(message);
        } else {
            c.send_event(message);
        }
        Ok(())
    }

    /// Initiate a new SSE stream for the given request.
    pub fn create_stream<R>(
        &self,
        channel: &str,
        request: Request<R>,
    ) -> http::Result<Response<StreamBody<Receiver<Chunk>>>> {
        let last_id: usize = match request.headers().get("Last-Event-ID") {
            None => 0,
            Some(header) => header
                .to_str()
                .map(|s| s.parse::<usize>().unwrap_or(0))
                .unwrap_or(0),
        };

        let (tx, rx) = futures::channel::mpsc::channel(100);
        let client = Client {
            tx,
            first_error: None,
        };

        match self.channels.lock().unwrap().entry(channel.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Channel::default()),
        }
        .add_client(client, last_id);

        Response::builder()
            .header("Cache-Control", "no-cache")
            .header("X-Accel-Buffering", "no")
            .header("Content-Type", "text/event-stream")
            .header("Access-Control-Allow-Origin", "*")
            .body(StreamBody::new(rx))
    }

    pub fn perform_maintenance(&self) {
        for channel in self.channels.lock().unwrap().values_mut() {
            channel.perform_maintenance();
        }
    }
}

#[derive(Default)]
struct Channel {
    clients: Vec<Client>,
    replayable_events: Vec<String>,
}

impl Channel {
    pub fn add_client(&mut self, mut client: Client, last_event: usize) {
        for chunk in self.replayable_events.iter().skip(last_event) {
            client.send_event(chunk.clone());
        }
        self.clients.push(client);
    }

    pub fn perform_maintenance(&mut self) {
        self.send_heartbeats();
        self.remove_stale_clients();
    }

    /// Send hearbeat to all clients.
    ///
    /// This should be called regularly (e.g. every 15 minutes) to detect
    /// a disconnect of the underlying TCP connection.
    fn send_heartbeats(&mut self) {
        self.send_event(":\n\n".into());
    }

    /// Remove disconnected clients.
    ///
    /// This removes all clients from all channels that have closed the
    /// connection or are not responding to the heartbeats, which caused
    /// a TCP timeout.
    ///
    /// This function should be called regularly (e.g. together with
    /// `send_heartbeats`) to keep the memory usage low.
    fn remove_stale_clients(&mut self) {
        self.clients.retain(|client| {
            if let Some(first_error) = client.first_error {
                if first_error.elapsed() > Duration::from_secs(5) {
                    info!("Removing stale client");
                    return false;
                }
            }
            true
        });
    }

    /// Send an event to all clients.
    pub fn send_replayable_event(&mut self, chunk: String) {
        let id = self.replayable_events.len() + 1;
        let new_chunk = format!("id: {}\n{}", id, &chunk);
        self.replayable_events.push(new_chunk.clone());
        self.send_event(new_chunk);
    }

    /// Send an event to all clients.
    pub fn send_event(&mut self, chunk: String) {
        debug!("Sending: {}", &chunk);
        for client in self.clients.iter_mut() {
            client.send_event(chunk.clone());
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
    fn send_event(&mut self, chunk: String) {
        let result = self.tx.try_send(Ok(Frame::data(Bytes::from(chunk))));
        match (&result, self.first_error) {
            (Err(e), None) => {
                error!("Unable to send event to client: {}", e);
                // Store time when an error was first seen
                self.first_error = Some(Instant::now());
            }
            (Ok(_), Some(_)) => {
                // Clear error when write succeeds
                self.first_error = None;
            }
            _ => {}
        }
    }
}
