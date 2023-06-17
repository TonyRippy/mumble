#!/bin/python3

# Converts the raw CPU data, which is a cumuluative count of jiffies,
# to the number of cores used over the sampled interval.
#
# Copyright (C) 2023, Tony Rippy
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License in the LICENSE file at the
# root of this repository, or online at

#     http://www.apache.org/licenses/LICENSE-2.0

# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import csv
import gzip
import sys
from datetime import datetime, timedelta

if len(sys.argv) != 2:
    print("Usage: cpu_core_usage.py <column>")
    sys.exit(1)
column = sys.argv[1]

PATH = "cpu.csv.gz"
JIFFY = timedelta(seconds=1 / 100)
ZERO = timedelta()
TWO_SECONDS = timedelta(seconds=2)

with gzip.open(PATH, "rt") as f:
    reader = csv.reader(f)

    # Read the header and locate the position of the desired column.
    header = next(reader)
    try:
        idx = header.index(column)
    except ValueError:
        sys.stderr.write(f'Column "{column}" not found.\n')
        sys.exit(1)

    # Start writing the output.
    writer = csv.writer(sys.stdout)
    writer.writerow(["timestamp_secs", "timestamp_nanos", "value"])

    # Read the first record which helps set the basis for the diff.
    row = next(reader)
    last_t = datetime.fromtimestamp(int(row[0])) + timedelta(
        microseconds=int(row[1]) / 1000
    )
    last_value = int(row[idx])

    # Loop over remaining records and calculate the CPU usage in cores.
    for row in reader:
        secs = int(row[0])
        nanos = int(row[1])
        t = datetime.fromtimestamp(secs) + timedelta(microseconds=nanos / 1000)
        dt = t - last_t
        value = int(row[idx])
        if ZERO < dt < TWO_SECONDS:
            # Convert the time to number of jiffies, which are the CPU accounting unit.
            jiffies = dt / JIFFY
            assert value >= last_value
            dv = float(value - last_value)
            cpu = dv / jiffies
            # Jiffies are measured in 1/100ths, so rounding to 3 significant digits is reasonable.
            writer.writerow([row[0], row[1], round(cpu, 3)])
        last_value = value
        last_t = t
