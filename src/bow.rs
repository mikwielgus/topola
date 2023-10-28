use crate::graph::{GetEnds, Index, Interior, LooseBendIndex, LooseDotIndex, LooseSegIndex};

#[derive(Debug, Clone, Copy)]
pub struct Bow {
    seg1_dot1: LooseDotIndex,
    seg1: LooseSegIndex,
    seg1_dot2: LooseDotIndex,
    bend: LooseBendIndex,
    seg2_dot1: LooseDotIndex,
    seg2: LooseSegIndex,
    seg2_dot2: LooseDotIndex,
}

/*impl Bow {
    pub fn from_bend(index: LooseBendIndex, graph: &StableDiGraph<Weight, Label, usize>) -> Self {
        let bend = index;

        let seg1_dot2 = LooseBend::new(bend, graph).ends().0;
        let seg1 = LooseDot::new(seg1_dot2, graph).seg().unwrap();
        let seg1_dot1 = LooseSeg::new(seg1, graph).other_end(seg1_dot2);

        let seg2_dot1 = LooseBend::new(bend, graph).ends().1;
        let seg2 = LooseDot::new(seg2_dot1, graph).seg().unwrap();
        let seg2_dot2 = LooseSeg::new(seg2, graph).other_end(seg2_dot1);

        Self {
            seg1_dot1,
            seg1,
            seg1_dot2,
            bend,
            seg2_dot1,
            seg2,
            seg2_dot2,
        }
    }
}*/

impl Interior<Index> for Bow {
    fn interior(&self) -> Vec<Index> {
        vec![
            self.seg1.into(),
            self.seg1_dot2.into(),
            self.bend.into(),
            self.seg2_dot1.into(),
            self.seg2.into(),
        ]
    }
}

impl GetEnds<LooseDotIndex, LooseDotIndex> for Bow {
    fn ends(&self) -> (LooseDotIndex, LooseDotIndex) {
        (self.seg1_dot1, self.seg2_dot2)
    }
}
