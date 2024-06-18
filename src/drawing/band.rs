use crate::drawing::seg::{LoneLooseSegIndex, SeqLooseSegIndex};

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum BandFirstSegIndex {
    Straight(LoneLooseSegIndex),
    Bended(SeqLooseSegIndex),
}
