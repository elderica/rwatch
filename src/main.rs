use env_logger::Env;
use log::info;
use sysinfo::{Disks, System};

mod metrics;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();

    const ntp_servers: [&str;2] = ["169.254.169.254:123", "ntp.nict.jp:123"];

    let ntp_time = match metrics::time::NtpClock::new(&ntp_servers) {
        Some(ntp_time) => ntp_time,
        None => {
            info!("Failed to synchronize with all NTP servers");
            return;
        }
    };

    loop {
        let cpu_usage = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);

        info!(
            "[{}] CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%",
            ntp_time.now().format("%Y-%m-%d %H:%M:%S%.3f"),
            cpu_usage,
            available_memory_percentage,
            disk_usage
        );
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
