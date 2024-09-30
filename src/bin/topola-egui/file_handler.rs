use std::io;
use std::sync::mpsc::{SendError, Sender};

#[repr(transparent)]
pub enum FileHandlerData {
    #[cfg(not(target_arch = "wasm32"))]
    File(io::BufReader<std::fs::File>),

    #[cfg(target_arch = "wasm32")]
    Contents(io::Cursor<Vec<u8>>),
}

macro_rules! fhd_forward {
    (match $self:expr => $fname:ident($($arg:expr,)*)) => {{
        match $self {
            #[cfg(not(target_arch = "wasm32"))]
            FileHandlerData::File(brf) => brf.$fname($($arg,)*),

            #[cfg(target_arch = "wasm32")]
            FileHandlerData::Contents(curs) => curs.$fname($($arg,)*),
        }
    }};

    (fn $fname:ident(&mut self $(,$arg:ident : $argty:ty)*)) => {
        #[inline]
        fn $fname(&mut self $(,$arg : $argty)*) {
            fhd_forward!(match self => $fname($($arg,)*))
        }
    };
    (fn $fname:ident(&mut self $(,$arg:ident : $argty:ty)*) -> $ret:ty) => {
        #[inline]
        fn $fname(&mut self $(,$arg : $argty)*) -> $ret {
            fhd_forward!(match self => $fname($($arg,)*))
        }
    }
}

impl io::Read for FileHandlerData {
    fhd_forward!(fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>);
    fhd_forward!(fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize>);
    fhd_forward!(fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize>);
}

impl io::BufRead for FileHandlerData {
    fhd_forward!(fn fill_buf(&mut self) -> io::Result<&[u8]>);
    fhd_forward!(fn consume(&mut self, amt: usize));
}

#[inline]
pub async fn push_file_to_read<R, E, C>(
    file_handle: &rfd::FileHandle,
    sender: Sender<Result<R, E>>,
    callback: C,
) where
    E: From<std::io::Error>,
    C: FnOnce(FileHandlerData) -> Result<R, E>,
{
    let _ = sender.send(handle_text(&file_handle, callback).await);
}

async fn handle_text<R, E, C>(file_handle: &rfd::FileHandle, callback: C) -> Result<R, E>
where
    E: From<std::io::Error>,
    C: FnOnce(FileHandlerData) -> Result<R, E>,
{
    #[cfg(not(target_arch = "wasm32"))]
    let res = FileHandlerData::File(io::BufReader::new(std::fs::File::open(file_handle.path())?));

    #[cfg(target_arch = "wasm32")]
    let res = FileHandlerData::Contents(io::Cursor::new(file_handle.read().await));

    callback(res)
}
