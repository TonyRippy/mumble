// Tool for generating Prometheus Native Histograns from CSV files.
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

package main

import (
	"database/sql"
	"encoding/csv"
	"encoding/json"
	"flag"
	"io"
	"log"
	"os"
	"strconv"
	"time"

	"google.golang.org/protobuf/proto"

	client "github.com/prometheus/client_golang/prometheus"
	dto "github.com/prometheus/client_model/go"

	_ "github.com/mattn/go-sqlite3"
)

var help = flag.Bool("help", false, "Show help")
var timestamp = flag.Int64("timestamp", 0, "The time of the sample.")
var infile = flag.String("input", "", "The input CSV file.")
var dbfile = flag.String("database", "", "The database to write to.")
var name = flag.String("var", "", "The name of the metric.")
var label = flag.String("label", "", "The name of the metric label.")
var factor = flag.Float64("factor", 1.1, "The factor to use for the native histograms.")

// Parses the header of the CSV file and returns a list of label ids and histograms.
//
// The first two columns make up the timestamp.
// Column 1 is a UNIX timestamp, in seconds since the epoch.
// Column 2 is the fractional part of the timestamp, in nanoseconds.
//
// The remaining columns are labels. If there is only one column, then
// there will be no label added to the label set. If there is more
// than one column, then the column header will be used as value of the
// label specified on the command line.
//
// For example, if the CSV file contains the following headers:
//
//	"timestamp_secs,timestamp_nanos,user,nice,system,..."
//
// and assuming this program is invoked with the following flags:
//
//	--var cpu --label mode
//
// then the following label sets will be used:
//
//	{"__name__": "cpu", "mode": "user"}
//	{"__name__": "cpu", "mode": "nice"}
//	{"__name__": "cpu", "mode": "system"}
//	...
func parse_header(db *sql.DB, header []string) ([]int, []client.Histogram) {
	labels := make([][]byte, 0)
	samples := make([]client.Histogram, 0)
	if *label == "" {
		if header[2] != "value" {
			log.Fatal("Expected 3rd column to be 'value'")
		}
		var labelSet = make(map[string]string)
		labelSet["__name__"] = *name
		label, err := json.Marshal(labelSet)
		if err != nil {
			log.Fatal(err)
		}
		labels = append(labels, label)
		samples = append(samples, client.NewHistogram(client.HistogramOpts{
			NativeHistogramBucketFactor: *factor,
		}))
	} else {
		for i := 2; i < len(header); i++ {
			var labelSet = make(map[string]string)
			labelSet["__name__"] = *name
			labelSet[*label] = header[i]
			label, err := json.Marshal(labelSet)
			if err != nil {
				log.Fatal(err)
			}
			labels = append(labels, label)
			samples = append(samples, client.NewHistogram(client.HistogramOpts{
				NativeHistogramBucketFactor: *factor,
			}))
		}
	}
	label_ids := make([]int, len(labels))
	for i, label := range labels {
		var id int
		// Query for a value based on a single row.
		err := db.QueryRow("SELECT id FROM label_set WHERE labels = ?;", label).Scan(&id)
		if err == nil {
			label_ids[i] = id
			continue
		}
		if err != sql.ErrNoRows {
			log.Fatal(err)
		}
		err = db.QueryRow("INSERT INTO label_set (labels) VALUES (?) RETURNING id;", label).Scan(&id)
		if err != nil {
			log.Fatal(err)
		}
		label_ids[i] = id
	}
	return label_ids, samples
}

func main() {
	// Parse the flag
	flag.Parse()
	if *help || *timestamp == 0 || *infile == "" || *dbfile == "" {
		flag.Usage()
		os.Exit(0)
	}

	// Open the database where the histograms should be written.
	// This database should use the "denormalized.sql" schema.
	db, err := sql.Open("sqlite3", *dbfile)
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()

	// Open the CSV file.
	f, err := os.Open(*infile)
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()

	reader := csv.NewReader(f)
	reader.ReuseRecord = true
	header, _ := reader.Read()
	label_ids, samples := parse_header(db, header)
	for {
		// Read the next record from the CSV file.
		record, err := reader.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			log.Fatalf("Failed to read CSV: %v", err)
		}
		// Parse the value columns and add them as observaton to the histograms.
		for i, sample := range samples {
			str := record[i+2]
			v, err := strconv.ParseFloat(str, 64)
			if err != nil {
				log.Fatalf("Failed to parse value %v: %v", str, err)
			}
			sample.Observe(v)
		}
	}
	// Generate a serialized proto message for each histogram and write it to the database.
	for i, sample := range samples {
		metric := &dto.Metric{}
		sample.Write(metric)
		bytes, err := proto.Marshal(metric.Histogram)
		if err != nil {
			log.Fatal(err)
		}
		_, err = db.Exec("INSERT INTO monitoring_data VALUES(?,?,?);", time.Unix(*timestamp, 0), label_ids[i], bytes)
		if err != nil {
			log.Fatal(err)
		}
	}
}
