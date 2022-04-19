use std::io::Write;

use flume::Sender;
use once_cell::sync::Lazy;

static LOG: Lazy<Sender<String>> = Lazy::new(|| {
    let (tx, rx) = flume::unbounded::<String>();
    std::thread::spawn(move || {
        let mut buffer = String::new();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("log.txt")
            .unwrap();
        while let Ok(e) = rx.recv() {
            buffer.clear();
            buffer.push_str(&e);
            buffer.push('\n');
            while let Ok(e) = rx.try_recv() {
                buffer.push_str(&e);
                buffer.push('\n');
            }
            file.write_all(buffer.as_bytes()).unwrap();
            std::fs::write("log.txt", buffer.as_bytes()).unwrap();
        }
    });
    tx
});

#[allow(unused)]
pub fn log(message: impl Into<String>) {
    LOG.send(message.into()).unwrap();
}
