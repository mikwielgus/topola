//! Module containing the informations about handling the Specctra
//! based file format, and parsing it into Topola's objects
#![forbid(unused_must_use)]
#![forbid(clippy::panic_in_result_fn, clippy::unwrap_in_result)]

mod common;
pub mod design;
pub mod mesadata;
mod read;
mod structure;
mod write;
