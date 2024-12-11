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
/// band. Both bare and cane heads have a face, which is the dot that terminates
/// the running end.
#[enum_dispatch(GetFace)]
#[derive(Debug, Clone, Copy)]
pub enum Head {
    Bare(BareHead),
    Cane(CaneHead),
}

impl<'a, CW, R> MakeRef<'a, HeadRef<'a, CW, R>, Drawing<CW, R>> for Head {
    fn ref_(&self, drawing: &'a Drawing<CW, R>) -> HeadRef<'a, CW, R> {
        HeadRef::new(*self, drawing)
    }
}

/// The head is bare when the routed band is not pulled out (i.e. is of zero
/// length). This happens on the first routing step and when the routed band was
/// completely contracted due to the routing algorithm backtracking. In these
/// situations a cane head cannot be used because there is obviously no cane
/// behind the face, and the face itself is fixed instead of loose.
#[derive(Debug, Clone, Copy)]
pub struct BareHead {
    pub face: FixedDotIndex,
}

impl GetFace for BareHead {
    fn face(&self) -> DotIndex {
        self.face.into()
    }
}

/// The head is a cane head when the routed band is pulled out (i.e. is of
/// non-zero length). It differs from a bare head by having a `cane` member,
/// which is the terminal cane on the running end of the currently routed band.
///
/// You can think of the cane head's cane as of a very long neck, though in
/// anatomy the neck is not considered to be a part of the head, and the face,
/// which here would be the head proper, itself has the same width as the cane,
/// also unlike a real head.
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

pub struct HeadRef<'a, CW, R> {
    head: Head,
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW, R> HeadRef<'a, CW, R> {
    pub fn new(head: Head, drawing: &'a Drawing<CW, R>) -> Self {
        Self { drawing, head }
    }
}

impl<'a, CW, R> GetFace for HeadRef<'a, CW, R> {
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
