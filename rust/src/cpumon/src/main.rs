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

#[macro_use]
extern crate log;

use clap::Parser;
use env_logger::Env;
use hyper::{server::conn::http1, service::service_fn};
use mumble::{ui, Histogram, Instrument};
use procfs::process::{Process, Stat};
use procfs::{CpuTime, KernelStats, ProcResult};
use std::io::Error;
use std::process::ExitCode;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::signal;
use tokio::task;
use tokio::time::{Instant, MissedTickBehavior};

struct Metrics {
    last_kernel: Option<KernelStats>,
    last_process: Option<Stat>,
    kernel_cpu_user: Histogram<f64>,
    kernel_cpu_nice: Histogram<f64>,
    kernel_cpu_system: Histogram<f64>,
    kernel_cpu_idle: Histogram<f64>,
    kernel_cpu_iowait: Histogram<f64>,
    kernel_cpu_irq: Histogram<f64>,
    kernel_cpu_softirq: Histogram<f64>,
    kernel_cpu_steal: Histogram<f64>,
    kernel_cpu_guest: Histogram<f64>,
    kernel_cpu_guest_nice: Histogram<f64>,
    process_cpu_user: Histogram<f64>,
    process_cpu_system: Histogram<f64>,
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
    pub fn new(meter: &mut mumble::Meter) -> Metrics {
        Metrics {
            last_kernel: None,
            last_process: None,
            kernel_cpu_user: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "user".into())
                .build(),
            kernel_cpu_nice: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "nice".into())
                .build(),
            kernel_cpu_system: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "system".into())
                .build(),
            kernel_cpu_idle: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "idle".into())
                .build(),
            kernel_cpu_iowait: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "iowait".into())
                .build(),
            kernel_cpu_irq: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "irq".into())
                .build(),
            kernel_cpu_softirq: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "softirq".into())
                .build(),
            kernel_cpu_steal: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "steal".into())
                .build(),
            kernel_cpu_guest: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "guest".into())
                .build(),
            kernel_cpu_guest_nice: meter
                .create_histogram("kernel_cpu")
                .add_attribute("mode", "guest_nice".into())
                .build(),
            process_cpu_user: meter
                .create_histogram("process_cpu")
                .add_attribute("mode", "user".into())
                .build(),
            process_cpu_system: meter
                .create_histogram("process_cpu")
                .add_attribute("mode", "system".into())
                .build(),
        }
    }

    fn sample(&mut self) -> ProcResult<()> {
        let ks = KernelStats::new()?;
        if let Some(last_ks) = &self.last_kernel {
            // Kernel stats are given in ticks, which can be converted to seconds
            // using procfs::ticks_per_second().
            let ticks_raw = total_ticks(&ks.total) - total_ticks(&last_ks.total);
            if ticks_raw < 10 {
                return Ok(());
            }
            let ticks = ticks_raw as f64;
            self.kernel_cpu_user
                .record(((ks.total.user - last_ks.total.user) as f64) / ticks);
            self.kernel_cpu_nice
                .record(((ks.total.nice - last_ks.total.nice) as f64) / ticks);
            self.kernel_cpu_system
                .record(((ks.total.system - last_ks.total.system) as f64) / ticks);
            self.kernel_cpu_idle
                .record(((ks.total.idle - last_ks.total.idle) as f64) / ticks);
            self.kernel_cpu_iowait.record(
                ((ks.total.iowait.unwrap_or(0) - last_ks.total.iowait.unwrap_or(0)) as f64) / ticks,
            );
            self.kernel_cpu_irq.record(
                ((ks.total.irq.unwrap_or(0) - last_ks.total.irq.unwrap_or(0)) as f64) / ticks,
            );
            self.kernel_cpu_softirq.record(
                ((ks.total.softirq.unwrap_or(0) - last_ks.total.softirq.unwrap_or(0)) as f64)
                    / ticks,
            );
            self.kernel_cpu_steal.record(
                ((ks.total.steal.unwrap_or(0) - last_ks.total.steal.unwrap_or(0)) as f64) / ticks,
            );
            self.kernel_cpu_guest.record(
                ((ks.total.guest.unwrap_or(0) - last_ks.total.guest.unwrap_or(0)) as f64) / ticks,
            );
            self.kernel_cpu_guest_nice.record(
                ((ks.total.guest_nice.unwrap_or(0) - last_ks.total.guest_nice.unwrap_or(0)) as f64)
                    / ticks,
            );
        }
        self.last_kernel = Some(ks);

        let ps = Process::myself()?.stat()?;
        if let Some(last_ps) = &self.last_process {
            let ticks = procfs::ticks_per_second() as f64;
            self.process_cpu_user
                .record(((ps.utime - last_ps.utime) as f64) / ticks);
            self.process_cpu_system
                .record(((ps.stime - last_ps.stime) as f64) / ticks);
        }
        self.last_process = Some(ps);
        Ok(())
    }

    fn push(&mut self) {
        let t = mumble::get_timestamp();
        self.kernel_cpu_user.push(t);
        self.kernel_cpu_nice.push(t);
        self.kernel_cpu_system.push(t);
        self.kernel_cpu_idle.push(t);
        self.kernel_cpu_iowait.push(t);
        self.kernel_cpu_irq.push(t);
        self.kernel_cpu_softirq.push(t);
        self.kernel_cpu_steal.push(t);
        self.kernel_cpu_guest.push(t);
        self.kernel_cpu_guest_nice.push(t);
        self.process_cpu_user.push(t);
        self.process_cpu_system.push(t);
    }
}

async fn monitoring_loop(port: u16) -> Result<(), Error> {
    let mut mp = mumble::MeterProvider::default();
    let mut metrics = Metrics::new(mp.get_meter("cpumon", None, None, None));

    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    info!("Listening on port {}", port);

    let mut sample_interval = tokio::time::interval(Duration::from_millis(500));
    sample_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    const PUSH_DURATION: Duration = Duration::from_secs(5);
    let mut push_interval = tokio::time::interval_at(Instant::now() + PUSH_DURATION, PUSH_DURATION);
    push_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut maintenance_interval = tokio::time::interval(ui::MAINTENANCE_INTERVAL);
    maintenance_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Interrupt signal received.");
                break
            }
            _ = sample_interval.tick() => {
                metrics.sample();
            }
            _ = push_interval.tick() => {
                metrics.push();
            }
            _ = maintenance_interval.tick() => {
                ui::maintain();
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
    Ok(())
}

#[derive(Parser)]
struct Cli {
    /// Monitoring port to use.
    #[arg(short, long, default_value_t = 9100)]
    port: u16,
}

fn main() -> ExitCode {
    // Parse command-line arguments
    let args = Cli::parse();
    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    match runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .and_then(|rt| rt.block_on(monitoring_loop(args.port)))
    {
        Err(err) => {
            error!("{}", err);
            ExitCode::FAILURE
        }
        _ => ExitCode::SUCCESS,
    }
}
