#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let cursor = std::io::Cursor::new(data);

    use specctra_core::{read::ListTokenizer, write::ListWriter, structure::Structure};

    let mut tkz = ListTokenizer::new(cursor);

    let res: Result<_, _> = tkz.read_value::<Structure>();

    if let Ok(val) = res {
        let mut dat = Vec::new();
        let mut lw = ListWriter::new(&mut dat);
        let _ = lw.write_value(&val);
    }
});
