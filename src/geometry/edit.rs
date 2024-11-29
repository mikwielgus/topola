use std::{collections::HashMap, hash::Hash, marker::PhantomData};

use crate::{
    drawing::graph::{GetLayer, Retag},
    graph::{GenericIndex, GetPetgraphIndex},
};

use super::{AccessBendWeight, AccessDotWeight, AccessSegWeight, GetWidth};

pub trait ApplyGeometryEdit<
    PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
    DW: AccessDotWeight<PW> + GetLayer,
    SW: AccessSegWeight<PW> + GetLayer,
    BW: AccessBendWeight<PW> + GetLayer,
    CW: Copy,
    PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Hash + Copy,
    DI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    SI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    BI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
>
{
    fn apply(&mut self, edit: GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>);
}

#[derive(Debug, Clone)]
pub struct GeometryEdit<
    PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
    DW: AccessDotWeight<PW> + GetLayer,
    SW: AccessSegWeight<PW> + GetLayer,
    BW: AccessBendWeight<PW> + GetLayer,
    CW: Copy,
    PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Hash + Copy,
    DI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    SI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    BI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
> {
    pub(super) dots: HashMap<DI, (Option<DW>, Option<DW>)>,
    pub(super) segs: HashMap<SI, (Option<((DI, DI), SW)>, Option<((DI, DI), SW)>)>,
    pub(super) bends: HashMap<BI, (Option<((DI, DI, DI), BW)>, Option<((DI, DI, DI), BW)>)>,
    pub(super) compounds: HashMap<GenericIndex<CW>, (Option<(Vec<PI>, CW)>, Option<(Vec<PI>, CW)>)>,
    primitive_weight_marker: PhantomData<PW>,
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: AccessDotWeight<PW> + GetLayer,
        SW: AccessSegWeight<PW> + GetLayer,
        BW: AccessBendWeight<PW> + GetLayer,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Hash + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Hash + Copy,
    > GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    pub fn new() -> Self {
        Self {
            dots: HashMap::new(),
            segs: HashMap::new(),
            bends: HashMap::new(),
            compounds: HashMap::new(),
            primitive_weight_marker: PhantomData,
        }
    }
}
