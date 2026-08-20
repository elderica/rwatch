use log::{error, info};
use sysinfo::{Disks, System};

mod logging;
mod metrics;
mod signal;

fn main() {
    logging::init_logger();
    let mut server_logger = match logging::ServerLogger::new("server.jsonl") {
        Ok(logger) => logger,
        Err(error) => {
            error!("Failed to create server logger: {error}");
            return;
        }
    };

    const NTP_SERVERS: [&str; 2] = ["169.254.169.254:123", "ntp.nict.jp:123"];

    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();
    let ntp_time = match metrics::time::NtpClock::new(&NTP_SERVERS) {
        Some(ntp_time) => ntp_time,
        None => {
            error!("Failed to get NTP time");
            return;
        }
    };

    let args: Vec<String> = std::env::args().collect();
    let once = args.iter().any(|arg| arg == "--once");

    let shutdown = signal::setup_signal_handlers();

    loop {
      

        let cpu_usage: f64 = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);

        let timestamp = ntp_time
            .now()
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();

        info!(
            "[{}] CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%",
            timestamp, cpu_usage, available_memory_percentage, disk_usage
        );

        let record = format!(
            "{{\"timestamp\": \"{}\", \"cpu_usage\": {:.2}, \"available_memory_percentage\": {:.2}, \"disk_usage\": {:.2}}}",
            timestamp, cpu_usage, available_memory_percentage, disk_usage,
        );

        if let Err(error) = server_logger.append(&record) {
            error!("Failed to append to server log: {error}");
        }

        if once {
            break;
        }

        let (lock, cvar) = &*shutdown.condvar;
        let shutdown_requested = lock.lock().unwrap();

        let (shutdown_requested, _) = cvar
            .wait_timeout(
                shutdown_requested,
                std::time::Duration::from_secs(5),
            )
            .unwrap();

        if *shutdown_requested {
            info!("Graceful shutdown completed");
            break;
        }
        
    }
}
