// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::iter;

use contracts_try::debug_ensures;
use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use geo::{Coord, Line, Point};
use planar_incr_embed::RelaxedPath;
use rstar::AABB;

use crate::{
    drawing::{
        band::{BandTermsegIndex, BandUid},
        bend::{BendIndex, BendWeight, LooseBendWeight},
        dot::{
            DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, GeneralDotWeight, LooseDotIndex,
            LooseDotWeight,
        },
        gear::GearIndex,
        graph::{GetMaybeNet, IsInLayer, MakePrimitive, PrimitiveIndex, PrimitiveWeight},
        loose::LooseIndex,
        primitive::MakePrimitiveShape,
        rules::AccessRules,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
            SegWeight, SeqLooseSegIndex, SeqLooseSegWeight,
        },
        Cane, Collect, Drawing, DrawingEdit, DrawingException, Infringement,
    },
    geometry::{
        compound::ManageCompounds,
        edit::ApplyGeometryEdit,
        primitive::{AccessPrimitiveShape, PrimitiveShape, SegShape},
        shape::{AccessShape, Shape},
        GenericNode, GetLayer, GetSetPos,
    },
    graph::{GenericIndex, GetPetgraphIndex, MakeRef},
    layout::{
        poly::{add_poly_with_nodes_intern, MakePolygon, PolyWeight},
        via::{Via, ViaWeight},
    },
    math::{intersect_linestring_and_beam, LineIntersection, NormalLine, RotationSense},
};

/// Represents a weight for various compounds
#[derive(Clone, Copy, Debug)]
#[enum_dispatch(GetMaybeNet, IsInLayer)]
pub enum CompoundWeight {
    /// Represents the weight of a polygon compound, includes its basic [`Layout`] information
    Poly(PolyWeight),
    /// Represents Via weight properties, containing its [`Layout`] properties
    Via(ViaWeight),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompoundEntryLabel {
    Normal,
    NotInConvexHull,
}

/// The alias to differ node types
pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<CompoundWeight>>;
pub type LayoutEdit = DrawingEdit<CompoundWeight, CompoundEntryLabel>;

#[derive(Clone, Debug, Getters)]
/// Structure for managing the Layout design
pub struct Layout<R> {
    pub(super) drawing: Drawing<CompoundWeight, CompoundEntryLabel, R>,
}

impl<R> Layout<R> {
    pub fn new(drawing: Drawing<CompoundWeight, CompoundEntryLabel, R>) -> Self {
        Self { drawing }
    }
}

impl<R: AccessRules> Layout<R> {
    /// Insert [`Cane`] object into the [`Layout`]
    pub fn insert_cane(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        sense: RotationSense,
    ) -> Result<Cane, DrawingException> {
        self.drawing.insert_cane(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            sense,
        )
    }

    /// Remove [`Cane`] object from the [`Layout`]
    pub fn remove_cane(&mut self, recorder: &mut LayoutEdit, cane: &Cane, face: LooseDotIndex) {
        self.drawing.remove_cane(recorder, cane, face)
    }

    pub fn remove_termseg(&mut self, recorder: &mut LayoutEdit, termseg: BandTermsegIndex) {
        self.drawing.remove_termseg(recorder, termseg)
    }

    #[debug_ensures(ret.is_ok() -> self.drawing.node_count() == old(self.drawing.node_count()) + weight.to_layer - weight.from_layer + 2)]
    #[debug_ensures(ret.is_err() -> self.drawing.node_count() == old(self.drawing.node_count()))]
    /// Insert [`Via`] into the [`Layout`]
    pub fn add_via(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: ViaWeight,
    ) -> Result<GenericIndex<ViaWeight>, Infringement> {
        let compound = self.drawing.add_compound(recorder, weight.into());
        let mut dots = vec![];

        for layer in weight.from_layer..=weight.to_layer {
            match self.drawing.add_fixed_dot(
                recorder,
                FixedDotWeight(GeneralDotWeight {
                    circle: weight.circle,
                    layer,
                    maybe_net: weight.maybe_net,
                }),
            ) {
                Ok(dot) => {
                    self.drawing.add_to_compound(
                        recorder,
                        dot,
                        CompoundEntryLabel::Normal,
                        compound,
                    );
                    dots.push(dot);
                }
                Err(err) => {
                    // Remove inserted dots.
                    self.drawing.remove_compound(recorder, compound);

                    for dot in dots.iter().rev() {
                        self.drawing.remove_fixed_dot(recorder, *dot);
                    }

                    return Err(err);
                }
            }
        }

        Ok(GenericIndex::<ViaWeight>::new(compound.petgraph_index()))
    }

    pub fn add_fixed_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
    ) -> Result<FixedDotIndex, Infringement> {
        self.drawing.add_fixed_dot(recorder, weight)
    }

    pub fn add_fixed_dot_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
    ) -> FixedDotIndex {
        self.drawing.add_fixed_dot_infringably(recorder, weight)
    }

    pub fn add_fixed_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, DrawingException> {
        self.drawing.add_fixed_seg(recorder, from, to, weight)
    }

    pub fn add_fixed_seg_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> FixedSegIndex {
        self.drawing
            .add_fixed_seg_infringably(recorder, from, to, weight)
    }

    pub fn add_lone_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, DrawingException> {
        self.drawing.add_lone_loose_seg(recorder, from, to, weight)
    }

    pub fn add_seq_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, DrawingException> {
        self.drawing.add_seq_loose_seg(recorder, from, to, weight)
    }

    pub fn move_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        dot: DotIndex,
        to: Point,
    ) -> Result<(), Infringement> {
        self.drawing.move_dot(recorder, dot, to)
    }

    pub fn add_poly(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: PolyWeight,
    ) -> GenericIndex<PolyWeight> {
        GenericIndex::<PolyWeight>::new(
            self.drawing
                .add_compound(recorder, CompoundWeight::Poly(weight))
                .petgraph_index(),
        )
    }

    /// insert a polygon based upon the border nodes, and computes + returns the
    /// associated apex
    pub fn add_poly_with_nodes(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: PolyWeight,
        nodes: &[PrimitiveIndex],
    ) -> (GenericIndex<PolyWeight>, FixedDotIndex) {
        let layer = weight.layer();
        let maybe_net = weight.maybe_net();
        let poly = self.add_poly(recorder, weight);
        (
            poly,
            add_poly_with_nodes_intern(self, recorder, poly, nodes, layer, maybe_net),
        )
    }

    pub fn remove_band(
        &mut self,
        recorder: &mut LayoutEdit,
        band: BandTermsegIndex,
    ) -> Result<(), DrawingException> {
        self.drawing.remove_band(recorder, band)
    }

    pub fn poly_nodes(&self) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.drawing.rtree().iter().filter_map(|wrapper| {
            if let NodeIndex::Compound(compound) = wrapper.data {
                if let CompoundWeight::Poly(..) = self.drawing.compound_weight(compound) {
                    return Some(GenericIndex::<PolyWeight>::new(compound.petgraph_index()));
                }
            }

            None
        })
    }

    pub fn layer_poly_nodes(
        &self,
        layer: usize,
    ) -> impl Iterator<Item = GenericIndex<PolyWeight>> + '_ {
        self.drawing
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .filter_map(|wrapper| {
                if let NodeIndex::Compound(compound) = wrapper.data {
                    if let CompoundWeight::Poly(..) = self.drawing.compound_weight(compound) {
                        return Some(GenericIndex::<PolyWeight>::new(compound.petgraph_index()));
                    }
                }

                None
            })
    }

    pub fn poly_members(
        &self,
        poly: GenericIndex<PolyWeight>,
    ) -> impl Iterator<Item = (CompoundEntryLabel, PrimitiveIndex)> + '_ {
        self.drawing
            .geometry()
            .compound_members(GenericIndex::new(poly.petgraph_index()))
    }

    fn compound_shape(&self, compound: GenericIndex<CompoundWeight>) -> Shape {
        match self.drawing.compound_weight(compound) {
            CompoundWeight::Poly(_) => GenericIndex::<PolyWeight>::new(compound.petgraph_index())
                .ref_(self)
                .shape()
                .into(),
            CompoundWeight::Via(weight) => weight.shape().into(),
        }
    }

    pub fn node_shape(&self, index: NodeIndex) -> Shape {
        match index {
            NodeIndex::Primitive(primitive) => primitive.primitive(&self.drawing).shape().into(),
            NodeIndex::Compound(compound) => self.compound_shape(compound),
        }
    }

    /// Checks if a node is not a primitive part of a compound, and if yes, returns its apex and center
    pub fn apex_of_compoundless_node(
        &self,
        node: NodeIndex,
        active_layer: usize,
    ) -> Option<(FixedDotIndex, Point)> {
        fn handle_fixed_dot<R: AccessRules>(
            drawing: &Drawing<CompoundWeight, CompoundEntryLabel, R>,
            index: PrimitiveIndex,
        ) -> Option<(FixedDotIndex, &FixedDotWeight)> {
            let PrimitiveIndex::FixedDot(dot) = index else {
                return None;
            };
            if let GenericNode::Primitive(PrimitiveWeight::FixedDot(weight)) = drawing
                .geometry()
                .graph()
                .node_weight(dot.petgraph_index())
                .unwrap()
            {
                Some((dot, weight))
            } else {
                unreachable!()
            }
        }

        match node {
            NodeIndex::Primitive(primitive) => {
                if self
                    .drawing()
                    .geometry()
                    .compounds(GenericIndex::<()>::new(primitive.petgraph_index()))
                    .next()
                    .is_some()
                {
                    return None;
                }
                handle_fixed_dot(&self.drawing, primitive).map(|(dot, weight)| (dot, weight.pos()))
            }
            NodeIndex::Compound(compound) => Some(match self.drawing.compound_weight(compound) {
                CompoundWeight::Poly(_) => {
                    let poly =
                        GenericIndex::<PolyWeight>::new(compound.petgraph_index()).ref_(self);
                    (poly.apex(), poly.shape().center())
                }
                CompoundWeight::Via(weight) => {
                    let mut dots = self.drawing.geometry().compound_members(compound);
                    let apex = loop {
                        // this returns None if the via is not present on this layer
                        let (entry_label, dot) = dots.next()?;
                        if entry_label == CompoundEntryLabel::NotInConvexHull {
                            if let Some((dot, weight)) = handle_fixed_dot(&self.drawing, dot) {
                                if weight.layer() == active_layer {
                                    break dot;
                                }
                            }
                        }
                    };
                    (apex, weight.shape().center())
                }
            }),
        }
    }

    fn bands_between_positions_internal(
        &self,
        layer: usize,
        left_pos: Point,
        right_pos: Point,
    ) -> impl Iterator<Item = (f64, BandUid, LooseIndex)> + '_ {
        let ltr_line = geo::Line {
            start: left_pos.into(),
            end: right_pos.into(),
        };
        let fake_seg = SegShape {
            from: left_pos,
            to: right_pos,
            width: f64::EPSILON * 16.0,
        };
        let mut orig_hline = NormalLine::from(ltr_line);
        orig_hline.make_normal_unit();
        let orig_hline = orig_hline;
        let location_denom = orig_hline.segment_interval(&ltr_line);
        let location_start = *location_denom.start();
        let location_denom = *location_denom.end() - *location_denom.start();

        self.drawing
            .rtree()
            .locate_in_envelope_intersecting(&{
                let aabb_init = AABB::from_corners(
                    [left_pos.x(), left_pos.y()],
                    [right_pos.x(), right_pos.y()],
                );
                AABB::from_corners(
                    [aabb_init.lower()[0], aabb_init.lower()[1], layer as f64],
                    [aabb_init.upper()[0], aabb_init.upper()[1], layer as f64],
                )
            })
            // TODO: handle non-loose entries (bends, segs)
            .filter_map(|geom| match geom.data {
                NodeIndex::Primitive(prim) => LooseIndex::try_from(prim).ok(),
                NodeIndex::Compound(_) => None,
            })
            .map(|loose| {
                let prim: PrimitiveIndex = loose.into();
                let shape = prim.primitive(&self.drawing).shape();
                (loose, shape)
            })
            .filter_map(move |(loose, shape)| {
                let band_uid = self.drawing.loose_band_uid(loose).ok()?;
                let loose_hline = orig_hline.orthogonal_through(&match shape {
                    PrimitiveShape::Seg(seg) => {
                        let seg_hline = NormalLine::from(seg.middle_line());
                        match orig_hline.intersects(&seg_hline) {
                            LineIntersection::Empty => return None,
                            LineIntersection::Overlapping => shape.center(),
                            LineIntersection::Point(pt) => pt,
                        }
                    }
                    _ => {
                        if !fake_seg.intersects(&shape) {
                            return None;
                        }
                        shape.center()
                    }
                });
                let location = (loose_hline.offset - location_start) / location_denom;
                log::trace!(
                    "intersection ({:?}) with {:?} is at {:?}",
                    band_uid,
                    shape,
                    location
                );
                (0.0..=1.0)
                    .contains(&location)
                    .then_some((location, band_uid, loose))
            })
    }

    /// Finds all bands on `layer` between `left` and `right`
    /// (usually assuming `left` and `right` are neighbors in a Delaunay triangulation)
    /// and returns them ordered from `left` to `right`.
    pub fn bands_between_nodes(
        &self,
        layer: usize,
        left: NodeIndex,
        right: NodeIndex,
    ) -> impl Iterator<Item = (BandUid, LooseIndex)> {
        assert_ne!(left, right);
        let left_pos = self.node_shape(left).center();
        let right_pos = self.node_shape(right).center();
        let mut bands: Vec<_> = self
            .bands_between_positions_internal(layer, left_pos, right_pos)
            .filter(|(_, band_uid, _)| {
                // filter entries which are connected to either lhs or rhs (and possibly both)
                let (bts1, bts2) = band_uid.into();
                let (bts1, bts2) = (bts1.petgraph_index(), bts2.petgraph_index());
                let geometry = self.drawing.geometry();
                [(bts1, left), (bts1, right), (bts2, left), (bts2, right)]
                    .iter()
                    .all(|&(x, y)| !geometry.is_joined_with(x, y))
            })
            .collect();
        bands.sort_by(|a, b| f64::total_cmp(&a.0, &b.0));

        // TODO: handle "loops" of bands, or multiple primitives from the band crossing the segment
        // both in the case of "edge" of a primitive/loose, and in case the band actually goes into a segment
        // and then again out of it.

        bands
            .into_iter()
            .map(|(_, band_uid, loose)| (band_uid, loose))
    }

    /// Finds all bands on `layer` between direction `left` and node `right`
    /// and returns them ordered from `left` to `right`.
    pub fn bands_between_node_and_boundary(
        &self,
        layer: usize,
        left: Coord,
        right: NodeIndex,
    ) -> Option<impl Iterator<Item = (BandUid, LooseIndex)>> {
        // First, decode the `left` direction into a point on the boundary
        let right_pos = self.node_shape(right).center();
        let left_pos = intersect_linestring_and_beam(
            self.drawing.boundary().exterior(),
            &Line {
                start: right_pos.0,
                end: right_pos.0 + left,
            },
        )?;

        let mut bands: Vec<_> = self
            .bands_between_positions_internal(layer, left_pos, right_pos)
            .filter(|(_, band_uid, _)| {
                // filter entries which are connected to rhs
                let (bts1, bts2) = band_uid.into();
                let (bts1, bts2) = (bts1.petgraph_index(), bts2.petgraph_index());
                let geometry = self.drawing.geometry();
                [(bts1, right), (bts2, right)]
                    .iter()
                    .all(|&(x, y)| !geometry.is_joined_with(x, y))
            })
            .collect();
        bands.sort_by(|a, b| f64::total_cmp(&a.0, &b.0));

        // TODO: handle "loops" of bands, or multiple primitives from the band crossing the segment
        // both in the case of "edge" of a primitive/loose, and in case the band actually goes into a segment
        // and then again out of it.

        Some(
            bands
                .into_iter()
                .map(|(_, band_uid, loose)| (band_uid, loose)),
        )
    }

    fn does_compound_have_core(&self, primary: NodeIndex, core: DotIndex) -> bool {
        let core: PrimitiveIndex = core.into();
        match primary {
            GenericNode::Primitive(pi) => pi == core,
            GenericNode::Compound(compound) => self
                .drawing
                .geometry()
                .compound_members(compound)
                .any(|(_, pi)| pi == core),
        }
    }

    /// Finds all bands on `layer` between `left` and `right`
    /// (usually assuming `left` and `right` are neighbors in a Delaunay triangulation)
    /// and returns them ordered from `left` to `right`.
    pub fn bands_between_nodes_with_alignment(
        &self,
        layer: usize,
        left: NodeIndex,
        right: NodeIndex,
    ) -> impl Iterator<Item = RelaxedPath<BandUid, ()>> + '_ {
        let mut alignment_idx: u8 = 0;

        // resolve end-points possibly to compounds such that
        // `does_compound_have_core` produces correct results.
        let maybe_to_compound = |x: NodeIndex| match x {
            GenericNode::Primitive(pi) => self
                .drawing
                .geometry()
                .compounds(pi)
                .next()
                .map(|(_, idx)| idx),
            GenericNode::Compound(compound) => Some(compound),
        };
        let resolve_node =
            |x: NodeIndex| maybe_to_compound(x).map(GenericNode::Compound).unwrap_or(x);
        let (left, right) = (resolve_node(left), resolve_node(right));

        self.bands_between_nodes(layer, left, right)
            .map(move |(band_uid, loose)| {
                // first, tag entry with core
                let maybe_core = match loose {
                    LooseIndex::Bend(lbi) => Some(self.drawing.geometry().core(lbi.into())),
                    _ => None,
                };
                Some((band_uid, maybe_core))
            })
            .chain(iter::once(None))
            .flat_map(move |item| {
                // insert alignment pseudo-paths if necessary
                let alignment_incr = match item {
                    Some((_, maybe_core)) => match (alignment_idx, maybe_core) {
                        (0, Some(core)) if self.does_compound_have_core(left, core) => 0,
                        (_, Some(core)) if self.does_compound_have_core(left, core) => {
                            panic!("invalid band ordering")
                        }

                        (_, Some(core)) if self.does_compound_have_core(right, core) => 2u8
                            .checked_sub(alignment_idx)
                            .expect("invalid band ordering"),

                        (0, _) => 1,
                        (1, _) => 0,
                        _ => panic!("invalid band ordering"),
                    },
                    None => 2u8
                        .checked_sub(alignment_idx)
                        .expect("invalid band ordering"),
                };

                alignment_idx += alignment_incr;

                iter::repeat_n(RelaxedPath::Weak(()), alignment_incr.into()).chain(
                    item.map(|(band_uid, _)| RelaxedPath::Normal(band_uid))
                        .into_iter(),
                )
            })
    }

    pub fn rules(&self) -> &R {
        self.drawing.rules()
    }

    pub fn rules_mut(&mut self) -> &mut R {
        self.drawing.rules_mut()
    }

    pub fn via(&self, index: GenericIndex<ViaWeight>) -> Via<R> {
        Via::new(index, self.drawing())
    }
}

impl<R: AccessRules>
    ApplyGeometryEdit<
        DotWeight,
        SegWeight,
        BendWeight,
        CompoundWeight,
        CompoundEntryLabel,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > for Layout<R>
{
    fn apply(&mut self, edit: &LayoutEdit) {
        self.drawing.apply(edit);
    }
}
