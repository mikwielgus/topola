use super::read::ParseError;

pub enum ListToken {
    Start { name: String },
    Leaf { value: String },
    End,
}

impl ListToken {
    pub fn expect_start(self, name: &'static str) -> Result<(), ParseError> {
        if let Self::Start { name: actual_name } = self {
            if name == actual_name {
                Ok(())
            } else {
                Err(ParseError::ExpectedStartOfList(name))
            }
        } else {
            Err(ParseError::ExpectedStartOfList(name))
        }
    }

    pub fn expect_any_start(self) -> Result<String, ParseError> {
        if let Self::Start { name } = self {
            Ok(name)
        } else {
            Err(ParseError::ExpectedStartOfList(""))
        }
    }

    pub fn expect_leaf(self) -> Result<String, ParseError> {
        if let Self::Leaf { value } = self {
            Ok(value)
        } else {
            Err(ParseError::Expected("leaf value"))
        }
    }

    pub fn expect_end(self) -> Result<(), ParseError> {
        if let Self::End = self {
            Ok(())
        } else {
            Err(ParseError::Expected("end of list"))
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
