// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Module about handling the Specctra based file format, and parsing + serializing it

pub mod error;
pub mod math;
pub mod mesadata;
pub mod read;
pub mod rules;
pub mod structure;
pub mod write;

pub enum ListToken {
    Start { name: String },
    Leaf { value: String },
    End,
}

impl ListToken {
    pub fn is_start_of(&self, valid_names: &[&'static str]) -> bool {
        if let Self::Start { name: actual_name } = self {
            valid_names
                .iter()
                .any(|i| i.eq_ignore_ascii_case(actual_name))
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        match &self {
            Self::Start { name } => 1 + name.len(),
            Self::Leaf { value } => value.len(),
            Self::End => 1,
        }
    }
}
