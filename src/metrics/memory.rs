use sysinfo::System;

pub fn get_memory_usage(sys: &mut System) -> f64 {
    sys.refresh_memory();
    let total_memory = sys.total_memory() as f64;
    let available_memory = sys.available_memory() as f64;
    (available_memory / total_memory) * 100.0
}
