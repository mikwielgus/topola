use crate::drawing::seg::{LoneLooseSegIndex, SeqLooseSegIndex};

pub enum BandIndex {
    Straight(LoneLooseSegIndex),
    Bended(SeqLooseSegIndex),
}
