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
- `promhist` - A utility that uses the Prometheus Go client library to generate native histograms from CSV data and write them as serialize protobufs to a local sqlite3 database.