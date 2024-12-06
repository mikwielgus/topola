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
