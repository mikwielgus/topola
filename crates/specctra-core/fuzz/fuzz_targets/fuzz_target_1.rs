#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let cursor = std::io::Cursor::new(data);

    use specctra_core::{read::ListTokenizer, structure::DsnFile};

    let mut tkz = ListTokenizer::new(cursor);

    let _: Result<_, _> = tkz.read_value::<DsnFile>();
});
