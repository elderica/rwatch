use std::path::Path;
use sysinfo::Disks;

pub fn get_disk_usage(disks: &mut Disks) -> f64 {
    disks.refresh(true);

    for disk in disks.list() {
        if disk.mount_point() == Path::new("/") {
            let total = disk.total_space() as f64;
            let available = disk.available_space() as f64;
            return (total - available) / total * 100.0;
        }
    }
    0.0
}
