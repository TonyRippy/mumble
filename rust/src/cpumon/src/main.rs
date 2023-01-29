// CPU monitor used to generate data for testing monitoring systems.
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

use hyper::{server::conn::http1, service::service_fn};
use mumble::ecdf::ECDF;
use mumble::ui;
use procfs::process::{Process, Stat};
use procfs::{CpuTime, KernelStats, ProcResult};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::signal;
use tokio::task;
use tokio::time::{Instant, MissedTickBehavior};

#[derive(Debug, Default)]
struct Metrics {
    last_kernel: Option<KernelStats>,
    last_process: Option<Stat>,
    cpu_user: ECDF<f64>,
    self_user: ECDF<u64>,
    self_system: ECDF<u64>,
}

fn total_ticks(cpu: &CpuTime) -> u64 {
    cpu.user
        + cpu.nice
        + cpu.system
        + cpu.idle
        + cpu.iowait.unwrap_or(0)
        + cpu.irq.unwrap_or(0)
        + cpu.softirq.unwrap_or(0)
        + cpu.steal.unwrap_or(0)
        + cpu.guest.unwrap_or(0)
        + cpu.guest_nice.unwrap_or(0)
}

impl Metrics {
    fn sample(&mut self) -> ProcResult<()> {
        let ks = KernelStats::new()?;
        if let Some(last_ks) = &self.last_kernel {
            // Kernel stats are given in ticks, which can be converted to seconds
            // using procfs::ticks_per_second().
            let ticks = total_ticks(&ks.total) - total_ticks(&last_ks.total);
            if ticks < 10 {
                return Ok(());
            }
            self.cpu_user.add(
                ((ks.total.user - last_ks.total.user) as f64) / (ticks as f64),
                1,
            );
        }
        self.last_kernel = Some(ks);

        let ps = Process::myself()?.stat()?;
        if let Some(last_ps) = &self.last_process {
            self.self_user.add(ps.utime - last_ps.utime, 1);
            self.self_system.add(ps.stime - last_ps.stime, 1);
        }
        self.last_process = Some(ps);
        Ok(())
    }
    fn compact(&mut self) {
        println!("{}", serde_json::to_string(&self.cpu_user).unwrap());
        self.cpu_user.clear();
    }
}

fn main() {
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async {
        let listener = TcpListener::bind("127.0.0.1:3000").await?;

        let mut metrics = Metrics::default();

        let mut sample_interval = tokio::time::interval(Duration::from_millis(500));
        sample_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        const COMPACT_DURATION: Duration = Duration::from_secs(5);
        let mut compact_interval =
            tokio::time::interval_at(Instant::now() + COMPACT_DURATION, COMPACT_DURATION);
        compact_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    break
                }
                _ = sample_interval.tick() => {
                    metrics.sample();
                }
                _ = compact_interval.tick() => {
                    metrics.compact();
                }
                Ok((tcp_stream, _)) = listener.accept() => {
                    tokio::spawn(
                        http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(tcp_stream, service_fn(ui::serve)));
                }
            }
            task::yield_now().await;
        }
        Ok::<(), std::io::Error>(())
    });
    println!("Shutdown");
}
