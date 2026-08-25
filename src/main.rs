use clap::Parser;
use log::{error, info};
use sysinfo::{Disks, System};

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

    // 前回のネットワーク累計バイト数。初回は差分が取れないため None。
    let mut previous_traffic: Option<metrics::network::TrafficBytes> = None;
    let mut previous_instant: Option<std::time::Instant> = None;

    loop {
        let cpu_usage: f64 = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);

        let current_traffic = metrics::network::TrafficBytes::read();
        let network_mbps = match (previous_traffic, current_traffic) {
            (Some(previous), Some(current)) => {
                let elapsed = previous_instant.map_or(0.0, |t| t.elapsed().as_secs_f64());
                metrics::network::TrafficBytes::megabits_per_sec(previous, current, elapsed)
            }
            _ => f64::NAN,
        };
        previous_traffic = current_traffic;
        previous_instant = Some(std::time::Instant::now());

        let timestamp = ntp_time.now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        info!(
            "[{}] CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%, Network: {} Mbps",
            timestamp,
            cpu_usage,
            available_memory_percentage,
            disk_usage,
            if network_mbps.is_nan() { "N/A".to_string() } else { format!("{:.3}", network_mbps) },
        );

        // network_mbps は初回のみ NaN となり JSON の null として記録される
        //（既存フィールドの並びは変えず、末尾に追記）。
        let network_json =
            if network_mbps.is_nan() { "null".to_string() } else { format!("{network_mbps:.3}") };

        let record = format!(
            "{{\"timestamp\": \"{}\", \"cpu_usage\": {:.2}, \
            \"available_memory_percentage\": {:.2}, \
            \"disk_usage\": {:.2}, \
            \"network_mbps\": {}}}",
            timestamp, cpu_usage, available_memory_percentage, disk_usage, network_json,
        );

        if let Err(error) = server_logger.append(&record) {
            error!("Failed to append to server log: {error}");
        }

        if cli.once {
            break;
        }

        let (lock, cvar) = &*shutdown.condvar;
        let mut shutdown_requested = lock.lock().unwrap();

        if !*shutdown_requested {
            let (guard, _) = cvar
                .wait_timeout(shutdown_requested, std::time::Duration::from_secs(INTERVAL_SECONDS))
                .unwrap();

            shutdown_requested = guard;
        }

        if *shutdown_requested {
            info!("Graceful shutdown completed");
            break;
        }
    }
}
