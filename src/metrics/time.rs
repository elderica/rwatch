use chrono::{DateTime, Duration, Local, TimeZone};
use rsntp::SntpClient;
use std::time::Instant;

const NTP_SERVER: &str = "132.163.97.4:123";

pub struct NtpClock {
    start_time: DateTime<Local>,
    start_instant: Instant,
}

impl NtpClock {
    pub fn new() -> Option<Self> {
        let client = SntpClient::new();

        match client.synchronize(NTP_SERVER) {
            Ok(response) => {
                let duration = response.datetime().unix_timestamp().ok()?;

                let start_time = Local
                    .timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
                    .single()?;

                Some(Self {
                    start_time,
                    start_instant: Instant::now(),
                })
            }
            Err(_) => None,
        }
    }

    pub fn now(&self) -> DateTime<Local> {
        let elapsed = self.start_instant.elapsed();

        self.start_time
            + Duration::from_std(elapsed)
                .expect("elapsed duration should be valid")
    }
}