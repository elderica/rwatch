use sysinfo::{MINIMUM_CPU_UPDATE_INTERVAL, System};

pub fn get_cpu_usage(sys: &mut System) -> f64 {
    sys.refresh_cpu_usage();
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();
    sys.global_cpu_usage().into()
}
