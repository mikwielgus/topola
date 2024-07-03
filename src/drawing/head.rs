use enum_dispatch::enum_dispatch;

use crate::geometry::shape::MeasureLength;

use super::{
    cane::Cane,
    dot::{DotIndex, FixedDotIndex, LooseDotIndex},
    primitive::MakePrimitiveShape,
    rules::AccessRules,
    Drawing,
};

#[enum_dispatch]
pub trait GetFace {
    fn face(&self) -> DotIndex;
}

#[enum_dispatch(GetFace)]
#[derive(Debug, Clone, Copy)]
pub enum Head {
    Bare(BareHead),
    Cane(CaneHead),
}

impl Head {
    pub fn ref_<'a, CW: Copy, R: AccessRules>(
        &self,
        drawing: &'a Drawing<CW, R>,
    ) -> HeadRef<'a, CW, R> {
        HeadRef::new(*self, drawing)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BareHead {
    pub face: FixedDotIndex,
}

impl GetFace for BareHead {
    fn face(&self) -> DotIndex {
        self.face.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CaneHead {
    pub face: LooseDotIndex,
    pub cane: Cane,
}

impl GetFace for CaneHead {
    fn face(&self) -> DotIndex {
        self.face.into()
    }
}

pub struct HeadRef<'a, CW: Copy, R: AccessRules> {
    head: Head,
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW: Copy, R: AccessRules> HeadRef<'a, CW, R> {
    pub fn new(head: Head, drawing: &'a Drawing<CW, R>) -> Self {
        Self { drawing, head }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetFace for HeadRef<'a, CW, R> {
    fn face(&self) -> DotIndex {
        self.head.face()
    }
}

impl<'a, CW: Copy, R: AccessRules> MeasureLength for HeadRef<'a, CW, R> {
    fn length(&self) -> f64 {
        match self.head {
            Head::Bare(..) => 0.0,
            Head::Cane(cane_head) => {
                self.drawing.primitive(cane_head.cane.seg).shape().length()
                    + self.drawing.primitive(cane_head.cane.bend).shape().length()
            }
        }
    }
}
