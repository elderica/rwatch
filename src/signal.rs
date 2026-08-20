use signal_hook::consts::signal::*;
use signal_hook::flag;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex,
};

pub struct Shutdown {
    pub running: Arc<AtomicBool>,
    pub condvar: Arc<(Mutex<bool>, Condvar)>,
}



pub fn setup_signal_handlers() -> Shutdown {
    let running = Arc::new(AtomicBool::new(true));
    let condvar = Arc::new((Mutex::new(false), Condvar::new()));

    let running_clone = Arc::clone(&running);
    let condvar_clone = Arc::clone(&condvar);

    flag::register(SIGINT, Arc::clone(&running_clone))
        .expect("Failed to register SIGINT handler");

    flag::register(SIGTERM, Arc::clone(&running_clone))
        .expect("Failed to register SIGTERM handler");

    std::thread::spawn(move || {
        while running_clone.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let (lock, cvar) = &*condvar_clone;
        let mut shutdown = lock.lock().unwrap();
        *shutdown = true;
        cvar.notify_one();
    });

    Shutdown { running, condvar }
}