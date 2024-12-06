use crate::error::ParseError;

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

    pub fn expect_start(self, valid_names: &[&'static str]) -> Result<(), ParseError> {
        assert!(!valid_names.is_empty());
        if self.is_start_of(valid_names) {
            Ok(())
        } else {
            Err(ParseError::ExpectedStartOfList(valid_names[0]))
        }
    }

    pub fn expect_any_start(self) -> Result<String, ParseError> {
        if let Self::Start { name } = self {
            Ok(name.to_ascii_lowercase())
        } else {
            Err(ParseError::ExpectedStartOfList(""))
        }
    }

    pub fn expect_leaf(self) -> Result<String, ParseError> {
        if let Self::Leaf { value } = self {
            Ok(value)
        } else {
            Err(ParseError::ExpectedLeaf)
        }
    }

    pub fn expect_end(self) -> Result<(), ParseError> {
        if let Self::End = self {
            Ok(())
        } else {
            Err(ParseError::ExpectedEndOfList)
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
