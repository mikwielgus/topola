use enum_dispatch::enum_dispatch;

use crate::{geometry::shape::MeasureLength, graph::MakeRef};

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

/// The head is the working part of the running end of the currently routed
/// band. Both bare and cane heads have a face, which is the fixed dot that
/// terminates the running end.
#[enum_dispatch(GetFace)]
#[derive(Debug, Clone, Copy)]
pub enum Head {
    Bare(BareHead),
    Cane(CaneHead),
}

impl<'a, CW: Copy, R: AccessRules> MakeRef<'a, HeadRef<'a, CW, R>, Drawing<CW, R>> for Head {
    fn ref_(&self, drawing: &'a Drawing<CW, R>) -> HeadRef<'a, CW, R> {
        HeadRef::new(*self, drawing)
    }
}

/// The head is bare when the routed band is not pulled out (i.e. is of zero
/// length). This happens on the first routing step and when the routed band was
/// contracted due to the routing algorithm backtracking. In these situations a
/// cane head cannot be used because there is obviously no cane behind the face.
#[derive(Debug, Clone, Copy)]
pub struct BareHead {
    pub face: FixedDotIndex,
}

impl GetFace for BareHead {
    fn face(&self) -> DotIndex {
        self.face.into()
    }
}

/// The head is a cane head when the routed band is of nonzero length (i.e. is
/// pulled out). It differs from the bare head by having a `cane` member, which
/// is the terminal cane on the running end of the currently routed band.
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
