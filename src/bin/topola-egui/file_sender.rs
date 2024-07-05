use std::sync::mpsc::{SendError, Sender};

pub struct FileSender {
    sender: Sender<String>,
}

impl FileSender {
    pub fn new(sender: Sender<String>) -> Self {
        Self { sender }
    }

    pub async fn send(&self, file_handle: rfd::FileHandle) -> Result<(), SendError<String>> {
        self.sender.send(self.handle_text(&file_handle).await)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn handle_text(&self, file_handle: &rfd::FileHandle) -> String {
        file_handle.path().to_str().unwrap().to_string()
    }

    #[cfg(target_arch = "wasm32")]
    async fn handle_text(&self, file_handle: &rfd::FileHandle) -> String {
        std::str::from_utf8(&file_handle.read().await)
            .unwrap()
            .to_string()
    }
}
