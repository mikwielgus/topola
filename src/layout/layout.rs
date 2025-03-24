// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::debug_ensures;
use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use geo::Point;
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
        graph::{GetMaybeNet, IsInLayer, MakePrimitive, PrimitiveIndex},
        loose::LooseIndex,
        primitive::{GetWeight, MakePrimitiveShape, Primitive},
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
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{
        poly::{is_apex, MakePolygon, Poly, PolyWeight},
        via::{Via, ViaWeight},
    },
    math::{Circle, LineIntersection, NormalLine},
};

/// Represents a weight for various compounds
#[derive(Debug, Clone, Copy)]
#[enum_dispatch(GetMaybeNet, IsInLayer)]
pub enum CompoundWeight {
    /// Represents the weight of a polygon compound, includes its basic [`Layout`] information
    Poly(PolyWeight),
    /// Represents Via weight properties, containing its [`Layout`] properties
    Via(ViaWeight),
}

/// The alias to differ node types
pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<CompoundWeight>>;
pub type LayoutEdit = DrawingEdit<CompoundWeight>;

#[derive(Clone, Debug, Getters)]
/// Structure for managing the Layout design
pub struct Layout<R> {
    drawing: Drawing<CompoundWeight, R>,
}

impl<R> Layout<R> {
    pub fn new(drawing: Drawing<CompoundWeight, R>) -> Self {
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
        cw: bool,
    ) -> Result<Cane, DrawingException> {
        self.drawing.insert_cane(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
        )
    }

    /// Remove [`Cane`] object from the [`Layout`]
    pub fn remove_cane(&mut self, recorder: &mut LayoutEdit, cane: &Cane, face: LooseDotIndex) {
        self.drawing.remove_cane(recorder, cane, face)
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
                    self.drawing.add_to_compound(recorder, dot, compound);
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

    pub fn add_poly_fixed_dot(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> Result<FixedDotIndex, Infringement> {
        let maybe_dot = self.drawing.add_fixed_dot(recorder, weight);

        if let Ok(dot) = maybe_dot {
            self.drawing.add_to_compound(recorder, dot, poly.into());
        }

        maybe_dot
    }

    pub fn add_poly_fixed_dot_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedDotIndex {
        let dot = self.drawing.add_fixed_dot_infringably(recorder, weight);
        self.drawing.add_to_compound(recorder, dot, poly.into());
        dot
    }

    pub fn add_fixed_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
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

    pub fn add_poly_fixed_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> Result<FixedSegIndex, Infringement> {
        let maybe_seg = self.add_fixed_seg(recorder, from, to, weight);

        if let Ok(seg) = maybe_seg {
            self.drawing.add_to_compound(recorder, seg, poly.into());
        }

        maybe_seg
    }

    pub fn add_poly_fixed_seg_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedSegIndex {
        let seg = self.add_fixed_seg_infringably(recorder, from, to, weight);
        self.drawing.add_to_compound(recorder, seg, poly.into());
        seg
    }

    pub fn add_lone_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        self.drawing.add_lone_loose_seg(recorder, from, to, weight)
    }

    pub fn add_seq_loose_seg(
        &mut self,
        recorder: &mut LayoutEdit,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
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
        let poly_compound = poly.into();

        for i in nodes {
            self.drawing.add_to_compound(
                recorder,
                GenericIndex::<()>::new(i.petgraph_index()),
                poly_compound,
            );
        }

        let shape = self.poly(poly).shape();
        let apex = self.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: shape.center(),
                    r: 100.0,
                },
                layer,
                maybe_net,
            }),
        );

        // maybe this should be a different edge kind
        self.drawing.add_to_compound(recorder, apex, poly_compound);

        assert!(is_apex(&self.drawing, apex));

        (poly, apex)
    }

    pub fn remove_band(
        &mut self,
        recorder: &mut LayoutEdit,
        band: BandTermsegIndex,
    ) -> Result<(), DrawingException> {
        self.drawing.remove_band(recorder, band)
    }

    pub fn polys<W: 'static>(
        &self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<CompoundWeight>> + '_ {
        self.drawing.compounds(node)
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
    ) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.drawing
            .geometry()
            .compound_members(GenericIndex::new(poly.petgraph_index()))
    }

    pub fn node_shape(&self, index: NodeIndex) -> Shape {
        match index {
            NodeIndex::Primitive(primitive) => primitive.primitive(&self.drawing).shape().into(),
            NodeIndex::Compound(compound) => match self.drawing.compound_weight(compound) {
                CompoundWeight::Poly(_) => self
                    .poly(GenericIndex::<PolyWeight>::new(compound.petgraph_index()))
                    .shape()
                    .into(),
                CompoundWeight::Via(_) => self
                    .via(GenericIndex::<ViaWeight>::new(compound.petgraph_index()))
                    .shape()
                    .into(),
            },
        }
    }

    /// Checks if a node is not a primitive part of a compound, and if yes, returns its center
    pub fn center_of_compoundless_node(&self, node: NodeIndex) -> Option<Point> {
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
                match primitive.primitive(self.drawing()) {
                    Primitive::FixedDot(dot) => Some(dot.weight().pos()),
                    // Primitive::LooseDot(dot) => Some(dot.weight().pos()),
                    _ => None,
                }
            }
            NodeIndex::Compound(_) => Some(self.node_shape(node).center()),
        }
    }

    /// Finds all bands on `layer` between `left` and `right`
    /// (usually assuming `left` and `right` are neighbors in a Delaunay triangulation)
    /// and returns them ordered from `left` to `right`.
    pub fn bands_between_nodes(
        &self,
        layer: usize,
        left: NodeIndex,
        right: NodeIndex,
    ) -> Vec<BandUid> {
        assert_ne!(left, right);
        let left_pos = self.node_shape(left).center();
        let right_pos = self.node_shape(right).center();
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
        let location_start = location_denom.start();
        let location_denom = location_denom.end() - location_denom.start();

        let mut bands: Vec<_> = self
            .drawing
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
            .filter_map(|(loose, shape)| {
                let band_uid = self.drawing.loose_band_uid(loose);
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
                    .then_some((location, band_uid))
            })
            .collect();
        bands.sort_by(|a, b| f64::total_cmp(&a.0, &b.0));

        // TODO: handle "loops" of bands, or multiple primitives from the band crossing the segment
        // both in the case of "edge" of a primitive/loose, and in case the band actually goes into a segment
        // and then again out of it.

        bands.into_iter().map(|(_, band_uid)| band_uid).collect()
    }

    pub fn rules(&self) -> &R {
        self.drawing.rules()
    }

    pub fn rules_mut(&mut self) -> &mut R {
        self.drawing.rules_mut()
    }

    pub fn poly(&self, index: GenericIndex<PolyWeight>) -> Poly<R> {
        Poly::new(index, self.drawing())
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
