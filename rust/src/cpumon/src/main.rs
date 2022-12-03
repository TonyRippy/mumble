use mumble::ECDF;
use num_rational::Ratio;
use procfs::process::{Process, Stat};
use procfs::{CpuTime, KernelStats, ProcResult};
use std::time::Duration;
use tokio::runtime;

#[derive(Debug, Default)]
struct Metrics {
    last_kernel: Option<KernelStats>,
    last_process: Option<Stat>,
    cpu_user: ECDF<Ratio<u64>>,
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
            self.cpu_user
                .add(Ratio::new(ks.total.user - last_ks.total.user, ticks), 1);
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
        println!("{:?}", self.cpu_user);
    }
}

fn main() {
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut metrics = Metrics::default();
        let mut sample_interval = tokio::time::interval(Duration::from_millis(500));
        let mut compact_interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = sample_interval.tick() => {
                    metrics.sample();
                }
                _ = compact_interval.tick() => {
                    metrics.compact();
                }
            }
        }
    });
}
