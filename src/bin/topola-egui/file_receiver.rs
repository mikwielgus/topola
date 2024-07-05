use std::io::{BufReader, Cursor};
use std::sync::mpsc::{Receiver, TryRecvError};

pub struct FileReceiver<'a> {
    receiver: &'a Receiver<String>,
}

impl<'a> FileReceiver<'a> {
    pub fn new(receiver: &'a Receiver<String>) -> Self {
        Self { receiver }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_recv(&mut self) -> Result<BufReader<std::fs::File>, TryRecvError> {
        Ok(std::io::BufReader::new(
            std::fs::File::open(self.receiver.try_recv()?).unwrap(),
        ))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn try_recv(&mut self) -> Result<Cursor<Vec<u8>>, TryRecvError> {
        Ok(Cursor::new(self.receiver.try_recv().unwrap().into()))
    }
}
