use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::sync::{Arc, Condvar, Mutex};

pub struct Shutdown {
    pub condvar: Arc<(Mutex<bool>, Condvar)>,
}



pub fn setup_signal_handlers() -> Shutdown {
   let condvar = Arc::new((Mutex::new(false), Condvar::new()));
   let condvar_clone =  Arc::clone(&condvar);

   let  mut signals =  Signals::new(&[SIGINT, SIGTERM]).expect("Failed to register signals");

    std::thread::spawn(move || {
        for _ in signals.forever() {
            let (lock, cvar) = &*condvar_clone;
            let mut shutdown = lock.lock().unwrap();
            *shutdown = true;
            cvar.notify_all();

            break;
        }
    });   

    Shutdown {
        condvar,
    } 
}