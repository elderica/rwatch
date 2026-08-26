use clap::Parser;
use log::{error, info};
use sysinfo::{Disks, System};

mod health;
mod logging;
mod metrics;
mod notify;
mod signal;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("RWATCH_COMMIT"), ")");

/// 閾値異常の検出頻度を抑えるため、状態が「回復」した直後の 1 回だけ
/// 報告する(エッジトリガ)。常時超過時にログが流れ続けるのを防ぐ。
fn report_violations_if_recovered_or_first(violations: &[String], was_reported: &mut bool) {
    if violations.is_empty() {
        *was_reported = false;
        return;
    }
    if !*was_reported {
        for finding in violations {
            error!("HEALTH: {finding}");
        }
        *was_reported = true;
    }
}

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
            error!("Failed to initialize server logger: {error}");
            std::process::exit(1);
        }
    };

    const NTP_SERVERS: [&str; 2] = ["169.254.169.254:123", "ntp.nict.jp:123"];

    let mut sys = System::new();
    let mut disks = Disks::new_with_refreshed_list();

    let ntp_time = match metrics::time::NtpClock::new(&NTP_SERVERS) {
        Some(clock) => clock,
        None => {
            error!("Failed to initialize NTP client");
            std::process::exit(1);
        }
    };

    let shutdown = signal::setup_signal_handlers();

    // 閾値監視の設定。旧 watchdog.py と同じ既定値。
    const THRESHOLDS: health::Thresholds =
        health::Thresholds { disk_used_pct: 85.0, mem_used_pct: 90.0 };
    let mut violations_reported = false;

    loop {
        let cpu_usage: f64 = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);
        // メモリ使用率(total - available ベース。watchdog.py と同じ定義)
        let mem_used_percentage = 100.0 - available_memory_percentage;

        let timestamp = ntp_time.now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        info!(
            "[{}] CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%",
            timestamp, cpu_usage, available_memory_percentage, disk_usage
        );

        let record = format!(
            "{{\"timestamp\": \"{}\", \"cpu_usage\": {:.2}, \
            \"available_memory_percentage\": {:.2}, \
            \"disk_usage\": {:.2}}}",
            timestamp, cpu_usage, available_memory_percentage, disk_usage,
        );

        if let Err(error) = server_logger.append(&record) {
            // JSONL 書き込み失敗時は ping を送らない。
            // systemd watchdog が書けない状態ごと検知できるようにするため。
            error!("Failed to append to server log: {error}");
        } else {
            // 書き込み成功の証として ping を送る(NOTIFY_SOCKET 未設定なら何もしない)
            if let Err(error) = notify::watchdog_ping() {
                error!("Failed to send watchdog ping: {error}");
            }

            let sample = health::Sample {
                disk_used_pct: disk_usage,
                mem_used_pct: mem_used_percentage,
                memory_basis: health::MemoryBasis::UsedFromAvailable,
            };
            let violations = THRESHOLDS.violations(&sample);
            report_violations_if_recovered_or_first(&violations, &mut violations_reported);
        }

        if cli.once {
            break;
        }

        let (lock, cvar) = &*shutdown.condvar;
        let mut shutdown_requested = lock.lock().unwrap();

        if !*shutdown_requested {
            let (guard, _) =
                cvar.wait_timeout(shutdown_requested, std::time::Duration::from_secs(5)).unwrap();

            shutdown_requested = guard;
        }

        if *shutdown_requested {
            info!("Graceful shutdown completed");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 異常なし → 何も出力されず、報告済みフラグがリセットされる。
    #[test]
    fn no_violations_resets_reported_flag() {
        let mut reported = true;
        report_violations_if_recovered_or_first(&[], &mut reported);
        assert!(!reported);
    }

    /// 初めての異常は報告対象になり、フラグが立つ。
    #[test]
    fn first_violation_is_reported() {
        let violations = vec!["disk usage 99.0% exceeds threshold 85.0%".to_string()];
        let mut reported = false;
        report_violations_if_recovered_or_first(&violations, &mut reported);
        assert!(reported);
    }

    /// 連続する異常は 2 回目以降報告しない(エッジトリガ)。
    #[test]
    fn repeated_violation_is_suppressed() {
        let violations = vec!["disk usage 99.0% exceeds threshold 85.0%".to_string()];
        let mut reported = false;
        report_violations_if_recovered_or_first(&violations, &mut reported);
        report_violations_if_recovered_or_first(&violations, &mut reported); // 2回目
        assert!(reported); // フラグは立ったまま=追加出力なし
    }
}
