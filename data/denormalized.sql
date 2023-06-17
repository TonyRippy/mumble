-- Simple SQLite3 schema for storing Prometheus-like monitoring data.
-- Copyright (C) 2023, Tony Rippy
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License in the LICENSE file at the
-- root of this repository, or online at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

CREATE TABLE IF NOT EXISTS [label_set] (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   labels TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS [monitoring_data] (
   timestamp DATETIME NOT NULL,
   label_set_id INTEGER NOT NULL,
   data BLOB NOT NULL,  -- Prometheus Histogram as a serialized protobuf.

   PRIMARY KEY (timestamp, label_set_id),

   FOREIGN KEY (label_set_id) REFERENCES [label_set] (id) 
     ON DELETE CASCADE ON UPDATE NO ACTION
);
