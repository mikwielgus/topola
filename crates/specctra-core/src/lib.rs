//! Module containing the informations about handling the Specctra
//! based file format, and parsing it into Topola's objects

mod common;
pub use common::*;
pub mod error;
pub mod math;
pub mod mesadata;
pub mod read;
pub mod rules;
pub mod structure;
pub mod write;
