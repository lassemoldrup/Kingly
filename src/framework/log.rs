use std::{fs::File, sync::Mutex};
use std::io::Write;

lazy_static! {
    pub static ref LOG: Log = Log::new();
}

pub struct Log {
    file: Mutex<File>
}

impl Log {
    const PATH: &'static str = "./log.txt";

    fn new() -> Self {
        let file = File::create(Self::PATH)
            .expect("Failed to create log file.");
        let file = Mutex::new(file);

        Self {
            file
        }
    }

    pub fn append(&self, line: &str) {
        let mut file = self.file.lock().unwrap();
        file.write_all(line.as_bytes())
            //.and_then(|_| file.write_all(b"\n"))
            .unwrap_or_else(|err| println!("Failed to write to log: {}", err))
    }
}
