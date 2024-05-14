use crate::drawing::seg::{LoneLooseSegIndex, SeqLooseSegIndex};

#[derive(Debug, Clone, Copy)]
pub enum BandIndex {
    Straight(LoneLooseSegIndex),
    Bended(SeqLooseSegIndex),
}
