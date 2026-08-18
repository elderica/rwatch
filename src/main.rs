use log::{error, info};
use sysinfo::{Disks, System};

mod logging;
mod metrics;

fn main() {
    logging::init_logger();
    let run_once = std::env::args().skip(1).any(|argument| argument == "--once");

    let mut server_logger = match logging::ServerLogger::new("server.jsonl") {
        Ok(logger) => logger,
        Err(error) => {
            error!("Failed to create server logger: {error}");
            return;
        }
    };

    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();

    let ntp_time = match metrics::time::NtpClock::new() {
        Some(ntp_time) => ntp_time,
        None => {
            error!("Failed to get NTP time");
            return;
        }
    };

    loop {
        let cpu_usage = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);

        let timestamp = ntp_time.now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

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

        if run_once {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
