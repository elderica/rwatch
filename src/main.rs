use env_logger::Env;
use log::info;
use sysinfo::{Disks, System};

mod metrics;
fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();

    loop {
        let cpu_usage = metrics::cpu::get_cpu_usage(&mut sys);
        let available_memory_percentage = metrics::memory::get_memory_usage(&mut sys);
        let disk_usage = metrics::disk::get_disk_usage(&mut disks);

        info!(
            "CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%",
            cpu_usage, available_memory_percentage, disk_usage
        );
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
