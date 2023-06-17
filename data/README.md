This directory contains data used to test monitoring concepts, and code used to process those datasets.

## Datasets

I have created the following datasets myself:

- [CPU Utilization](cpu.md)

The following data was available online in the public domain:

 - Rabenstein, Björn, 2020, "Cortex Logs from Grafana Labs"
   <br>https://github.com/beorn7/histogram_experiments/datasets

 - Zaker, Farzin, 2019, "Online Shopping Store - Web Server Logs",
   <br>https://doi.org/10.7910/DVN/3QBYB5, Harvard Dataverse, V1
   <br>https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/3QBYB5

 - gogobody, 2022, "gcloud latency trace"
   <br>https://www.kaggle.com/datasets/gogobody/gcloud-latency-trace

## Tools

The following tools were created to assist in preparing experiment data:
- `full-sample` - Reads a time series from a CSV file, combines it into a full-resolution ECDF, and stores it into a SQLite3 database. This is later used to measure the fidelity of histograms and other aggregations to the underlying data set.
- `partition-input` - Takes one big CSV file and breaks it up into one CSV file per time interval.
- `promhist` - A utility that uses the Prometheus Go client library to generate native histograms from CSV data and write them as serialize protobufs to a local sqlite3 database.
- `diff-normalized` - A tool that reads a SQLite3 database using the "normalized" schema and calculates the accuracy of the clusters as compared to the raw samples.

Other tools I used:
- [`csvq`](https://mithrandie.github.io/csvq/) - A tool for querying CSV files using SQL.
- [`sqlite3`](https://www.sqlite.org/) - A simple, file-based SQL database.
