// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unexpected end of file")]
    Eof,
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("expected {0}")]
    Expected(&'static str),
    #[error("expected \"({0}\"")]
    ExpectedStartOfList(&'static str),
    #[error("expected one of: {0:?}")]
    ExpectedStartOfListOneOf(&'static [&'static str]),
    #[error("expected \")\"")]
    ExpectedEndOfList,
    #[error("expected leaf value")]
    ExpectedLeaf,

    #[error("found a space inside a quoted string, but file didn't declare this possibility")]
    UnexpectedSpaceInQuotedStr,
}

impl ParseError {
    pub fn add_context(self, context: (usize, usize)) -> ParseErrorContext {
        ParseErrorContext {
            error: self,
            context,
        }
    }
}

#[derive(Error, Debug)]
#[error("line {}, column {}: {error}", .context.0, .context.1)]
pub struct ParseErrorContext {
    pub error: ParseError,
    pub context: (usize, usize),
}
