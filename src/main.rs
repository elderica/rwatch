use std::path::Path;

use env_logger::Env;
use sysinfo::{Disks, MINIMUM_CPU_UPDATE_INTERVAL, System};
use log::info;
fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let disks = Disks::new_with_refreshed_list();

    let mut disk_usage =  0.0;

    for disk in disks.list(){
        if disk.mount_point() == Path::new("/") {
            let total = disk.total_space() as f64 ;
            let available = disk.available_space() as f64;
            disk_usage = (total - available) / total * 100.0;
        }
    }
    let using_mem = sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0;
    info!("CPU: {:.2}%, Memory: {:.2}%, Disk: {:.2}%", sys.global_cpu_usage(), using_mem, disk_usage);
}
