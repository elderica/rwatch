use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub struct Shutdown {
    condvar: Arc<(Mutex<bool>, Condvar)>,
}

impl Shutdown {
    /// 最大 `duration` まで待機する。シグナル受信で即座に起床する。
    /// 戻り値 true = 停止要求あり。
    pub fn wait_timeout(&self, duration: Duration) -> bool {
        let (lock, cvar) = &*self.condvar;
        let requested = lock.lock().unwrap();
        let (guard, _) =
            cvar.wait_timeout_while(requested, duration, |requested| !*requested).unwrap();
        *guard
    }
}

pub fn setup_signal_handlers() -> Shutdown {
    let condvar = Arc::new((Mutex::new(false), Condvar::new()));
    let condvar_clone = Arc::clone(&condvar);

    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("Failed to register signals");

    std::thread::spawn(move || {
        if signals.forever().next().is_some() {
            let (lock, cvar) = &*condvar_clone;
            let mut shutdown = lock.lock().unwrap();
            *shutdown = true;
            cvar.notify_all();
        }
    });

    Shutdown { condvar }
}
