//! Module containing the informations about handling the Specctra
//! based file format, and parsing it into Topola's objects
#![forbid(unused_must_use)]
#![forbid(clippy::panic_in_result_fn, clippy::unwrap_in_result)]

pub use specctra_core::error::{ParseError, ParseErrorContext};
pub use specctra_core::mesadata;
use specctra_core::*;

pub mod design;
