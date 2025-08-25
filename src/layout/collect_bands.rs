// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::iter;

use geo::{Coord, Line, Point};
use planar_incr_embed::RelaxedPath;
use rstar::AABB;

use crate::{
    drawing::{
        band::BandUid,
        dot::DotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        loose::LooseIndex,
        primitive::MakePrimitiveShape,
        rules::AccessRules,
    },
    geometry::{
        compound::ManageCompounds,
        primitive::{AccessPrimitiveShape, PrimitiveShape, SegShape},
        shape::AccessShape,
        GenericNode,
    },
    graph::GetPetgraphIndex,
    layout::{Layout, NodeIndex},
    math::{intersect_linestring_and_ray, LineInGeneralForm, LineIntersection},
};

impl<R: AccessRules> Layout<R> {
    fn bands_between_positions(
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
        let mut orig_hline = LineInGeneralForm::from(ltr_line);
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
                let band_uid = self.drawing.find_loose_band_uid(loose).ok()?;
                let loose_hline = orig_hline.orthogonal_through(&match shape {
                    PrimitiveShape::Seg(seg) => {
                        let seg_hline = LineInGeneralForm::from(seg.middle_line());
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
                let location = (loose_hline.c - location_start) / location_denom;
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
            .bands_between_positions(layer, left_pos, right_pos)
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
        let left_pos = intersect_linestring_and_ray(
            self.drawing.boundary().exterior(),
            &Line {
                start: right_pos.0,
                end: right_pos.0 + left,
            },
        )?;

        let mut bands: Vec<_> = self
            .bands_between_positions(layer, left_pos, right_pos)
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
}
