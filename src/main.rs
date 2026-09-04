use clap::Parser;
use log::{error, info};
use sysinfo::{Disks, Networks, System};

mod logging;
mod metrics;
mod signal;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("RWATCH_COMMIT"), ")");

/// ループの基本インターバル [秒]。
const INTERVAL_SECONDS: u64 = 5;

#[derive(Parser)]
#[command(
    name = "rwatch",
    version = VERSION,
)]
struct Cli {
    /// Run once and exit
    #[arg(long)]
    once: bool,
}

fn main() {
    let cli = Cli::parse();

    logging::init_logger();

    let mut server_logger = match logging::ServerLogger::new("server.jsonl") {
        Ok(logger) => logger,
        Err(error) => {
            error!("Failed to create server logger: {error}");
            return;
        }
    };

    const NTP_SERVERS: [&str; 2] = ["169.254.169.254:123", "ntp.nict.jp:123"];

    let mut sys = System::new();
    let mut disks = Disks::new_with_refreshed_list();

    let ntp_time = match metrics::time::NtpClock::new(&NTP_SERVERS) {
        Some(ntp_time) => ntp_time,
        None => {
            error!("Failed to get NTP time");
            return;
        }
    };

    let shutdown = signal::setup_signal_handlers();

    // loopback を除外した全 NIC の区間トラフィックを sysinfo で取得する。
    let mut networks = Networks::new_with_refreshed_list();
    networks.refresh(true);

    loop {
        let cpu_usage: f64 = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);

        // sysinfo の received()/transmitted() は「前回 refresh からの差分バイト数」。
        // 計測窓の待機はシグナルで中断可能にする。中断時は窓が不完全なので記録せず終了する。
        let start = std::time::Instant::now();
        if shutdown.wait_timeout(std::time::Duration::from_secs(INTERVAL_SECONDS)) {
            info!("Graceful shutdown completed");
            break;
        }
        networks.refresh(true);
        let elapsed_seconds = start.elapsed().as_secs_f64();

        let (delta_bytes, interface_count) = networks
            .iter()
            .filter(|(name, _)| name.as_str() != "lo")
            .fold((0u64, 0usize), |(bytes, count), (_, data)| {
                (bytes + data.received() + data.transmitted(), count + 1)
            });
        let network_mbps =
            delta_bytes as f64 * 8.0 / 1_000_000.0 / elapsed_seconds.max(f64::EPSILON);

        let timestamp = ntp_time.now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        let network_display = format!("{network_mbps:.3}");
        info!(
            "[{}] CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%, Network: {network_display} Mbps ({interface_count} IF)",
            timestamp, cpu_usage, available_memory_percentage, disk_usage,
        );

        let record = format!(
            "{{\"timestamp\": \"{}\", \"cpu_usage\": {:.2}, \
            \"available_memory_percentage\": {:.2}, \
            \"disk_usage\": {:.2}, \
            \"network_mbps\": {network_mbps:.3}}}",
            timestamp, cpu_usage, available_memory_percentage, disk_usage,
        );

        if let Err(error) = server_logger.append(&record) {
            error!("Failed to append to server log: {error}");
        }

        if cli.once {
            break;
        }
    }
}
