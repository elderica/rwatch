use env_logger::Env;
use std::fs::{File, OpenOptions};

pub struct ServerLogger {
    file: File,
}

impl ServerLogger {
    pub fn new(path: &str) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file })
    }
    pub fn append(&mut self, message: &str) -> Result<(), std::io::Error> {
        use std::io::Write;
        self.file.write_all(message.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }
}

pub fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
}
