use std::io::{BufReader, Cursor};
use std::sync::mpsc::Receiver;

pub struct FileReceiver<'a> {
    receiver: &'a Receiver<String>,
}

impl<'a> FileReceiver<'a> {
    pub fn new(receiver: &'a Receiver<String>) -> Self {
        Self { receiver }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_recv(&mut self) -> Option<Result<BufReader<std::fs::File>, std::io::Error>> {
        Some(std::fs::File::open(self.receiver.try_recv().ok()?).map(std::io::BufReader::new))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn try_recv(&mut self) -> Option<Result<Cursor<Vec<u8>>, std::io::Error>> {
        Some(Ok(Cursor::new(self.receiver.try_recv().ok()?.into())))
    }
}
