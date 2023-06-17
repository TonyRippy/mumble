# CPU Utilization Data

This is a dataset I generated myself by scraping `/proc/stat` off and
on for a couple of weeks. The samples were collected every half-second
(or so).

## Format

This is a text-based CSV file, with the following columns:

| Column Name  |  Description  |
| ------------ | ------------- |
| unix\_time\_secs |  The time the sample was collected, stored as a UNIX timestamp. (Seconds since the epoch.)|
| unix\_time\_nanos |  The fractional part of the timestamp above, in nanoseconds. |
| user | Time spent in user mode. |
| nice | Time spent in user mode with low priority (nice). |
| system |  Time spent in system mode.|
| idle | Time spent in the idle task. |
| iowait | Time waiting for I/O to complete. |
| irq | Time servicing interrupts |
| softirq | Time servicing softirqs. |
| steal | Stolen time, which is the time spent in other operating systems when running in a virtualized environment. |
| guest | Time spent running a virtual CPU for guest operating systems under the control of the Linux kernel. |
| guest\_nice | Time spent running a niced guest (virtual CPU for guest operating systems under the control of the Linux kernel). |

The values were collected from the "cpu" line of `/proc/stat` from a
personal laptop. This represents CPU usage
across all cores on the machine. All values are cumulative,
recording CPU usage since laptop startup. The units are measured in
"Jiffies", which on this system was 1/100th of a second.

## System Information

Here is some information about the system this data was collected
from, if that helps:

```shell
$ uname -a
Linux trippy-xps 5.15.0-73-generic #80~20.04.1-Ubuntu SMP Wed May 17 14:58:14 UTC 2023 x86_64 x86_64 x86_64 GNU/Linux
```

You may also inspect the output of [/proc/cpuinfo](cpuinfo.txt).

## Using This Data

If you would like to use this data for your own project, please go
ahead! This data is released under the Apache 2.0 License, same as the
rest of the code in this project. This allows you to use the data 
for any purpose, to distribute it, modify it, and to distribute
modified versions.

I only ask that if it is useful to you, let me know!

### Citation

If you would like to cite this dataset, I recommend the following:

> Rippy, Tony, 2023, CPU Utilization of Laptop Computer
> <br>https://github.com/TonyRippy/mumble/data/cpu.csv.gz, V1

This citation is based on the following recommendations:
<br>https://dataverse.org/best-practices/data-citation
