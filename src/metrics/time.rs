use chrono::{DateTime, Duration, Local, TimeZone};
use rsntp::SntpClient;
use std::time::Instant;
use log::warn;

pub struct NtpClock {
    start_time: DateTime<Local>,
    start_instant: Instant,
}

impl NtpClock {
    pub fn new(ntp_servers:&[&str]) -> Option<Self> {
        let client = SntpClient::new();

        for server in ntp_servers{
            match client.synchronize(server){
                Ok(response)=>{
                let duration = response.datetime().unix_timestamp().ok()?;

                let start_time = Local
                    .timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
                    .single()?;

                return Some(Self {
                    start_time,
                    start_instant: Instant::now(),
                });        
            }
            Err(error) => {
            warn!("Failed to synchronize with NTP server {}: {}", server, error);
            }
        }
    warn!("Failed to synchronize with all NTP servers");
     None
}
    pub fn now(&self) -> DateTime<Local> {
        let elapsed = self.start_instant.elapsed();

        self.start_time
            + Duration::from_std(elapsed)
                .expect("elapsed duration should be valid")
    }
  }