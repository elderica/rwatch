use std::path::Path;

use env_logger::Env;
use log::error;
use sysinfo::{Disks, MINIMUM_CPU_UPDATE_INTERVAL, System};
use log::info;
use log::warn;
fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    loop{
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu_usage = sys.global_cpu_usage();
    let total_memory = sys.total_memory() as f64;
    let available_memory = sys.available_memory() as f64;
    let available_memory_percentage = (available_memory / total_memory) * 100.0;

    let disks = Disks::new_with_refreshed_list();

    let mut disk_usage =  0.0;

    for disk in disks.list(){
        if disk.mount_point() == Path::new("/") {
            let total = disk.total_space() as f64 ;
            let available = disk.available_space() as f64;
            disk_usage = (total - available) / total * 100.0;
        }
    }
    
    info!("CPU: {:.2}%, Memory available: {:.2}%, Disk: {:.2}%", cpu_usage, available_memory_percentage, disk_usage);
    std::thread::sleep(std::time::Duration::from_secs(5));
    }

}
