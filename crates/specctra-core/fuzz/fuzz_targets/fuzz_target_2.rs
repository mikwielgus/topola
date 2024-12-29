#![no_main]

// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let cursor = std::io::Cursor::new(data);

    use specctra_core::{read::ListTokenizer, structure::Structure, write::ListWriter};

    let mut tkz = ListTokenizer::new(cursor);

    let res: Result<_, _> = tkz.read_value::<Structure>();

    if let Ok(val) = res {
        let mut dat = Vec::new();
        {
            let mut lw = ListWriter::new(&mut dat);
            let _ = lw.write_value(&val).unwrap();
        }

        let cursor = std::io::Cursor::new(dat);
        let mut tkz = ListTokenizer::new(cursor);
        let val2 = tkz.read_value::<Structure>().unwrap();

        // make sure that serialization+parsing after parsing is identity
        assert_eq!(val, val2);
    }
});
