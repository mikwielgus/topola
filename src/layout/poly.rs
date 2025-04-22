// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Module for handling Polygon properties

use enum_dispatch::enum_dispatch;

use geo::{algorithm::ConvexHull, IsConvex, LineString, Point, Polygon};

use crate::{
    drawing::{
        dot::{FixedDotIndex, FixedDotWeight, GeneralDotWeight},
        graph::{GetMaybeNet, PrimitiveIndex},
        primitive::GetLimbs,
        rules::AccessRules,
        seg::SegIndex,
        Drawing,
    },
    geometry::{compound::ManageCompounds, shape::AccessShape, GetLayer, GetSetPos},
    graph::{GenericIndex, GetPetgraphIndex, MakeRef},
    layout::{CompoundEntryKind, CompoundWeight, Layout, LayoutEdit},
    math::Circle,
};

#[enum_dispatch]
pub trait MakePolygon {
    fn shape(&self) -> Polygon;
}

#[derive(Debug)]
pub struct PolyRef<'a, R> {
    pub index: GenericIndex<PolyWeight>,
    drawing: &'a Drawing<CompoundWeight, CompoundEntryKind, R>,
}

impl<'a, R: 'a> MakeRef<'a, Layout<R>> for GenericIndex<PolyWeight> {
    type Output = PolyRef<'a, R>;
    fn ref_(&self, layout: &'a Layout<R>) -> PolyRef<'a, R> {
        PolyRef::new(*self, layout.drawing())
    }
}

pub(super) fn add_poly_with_nodes_intern<R: AccessRules>(
    layout: &mut Layout<R>,
    recorder: &mut LayoutEdit,
    poly: GenericIndex<PolyWeight>,
    nodes: &[PrimitiveIndex],
    layer: usize,
    maybe_net: Option<usize>,
) -> FixedDotIndex {
    let poly_compound = poly.into();

    for i in nodes {
        layout.drawing.add_to_compound(
            recorder,
            GenericIndex::<()>::new(i.petgraph_index()),
            CompoundEntryKind::Normal,
            poly_compound,
        );
    }

    let shape = poly.ref_(layout).shape();
    let apex = layout.add_fixed_dot_infringably(
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

    if !shape.exterior().is_convex() {
        // create a temporary RTree to speed up lookups from O(n²) to O(n log n)
        #[derive(Clone, Copy)]
        struct Rto {
            pt: Point,
            idx: PrimitiveIndex,
        }
        impl rstar::RTreeObject for Rto {
            type Envelope = rstar::AABB<[f64; 2]>;
            fn envelope(&self) -> rstar::AABB<[f64; 2]> {
                rstar::AABB::from_point([self.pt.x(), self.pt.y()])
            }
        }
        impl rstar::PointDistance for Rto {
            fn distance_2(&self, point: &[f64; 2]) -> f64 {
                let d_x = self.pt.0.x - point[0];
                let d_y = self.pt.0.y - point[1];
                d_x * d_x + d_y * d_y
            }
        }

        let mut temp_rtree = rstar::RTree::<Rto>::new();

        for &idx in nodes {
            let PrimitiveIndex::FixedDot(dot) = idx else {
                continue;
            };

            let pt = layout.drawing.geometry().dot_weight(dot.into()).pos();
            temp_rtree.insert(Rto { pt, idx });
        }

        // compute the convex hull
        let convex_hull_exterior = shape.exterior().convex_hull().into_inner().0;
        for pt in convex_hull_exterior {
            // note that the convex hull should be a strict subset of the points
            // on the exterior of `shape`
            assert!(temp_rtree.pop_nearest_neighbor(&[pt.x, pt.y]).is_some());
        }

        // mark all elements which aren't in the convex hull
        for Rto { idx, .. } in temp_rtree {
            layout.drawing.add_to_compound(
                recorder,
                GenericIndex::<()>::new(idx.petgraph_index()),
                CompoundEntryKind::NotInConvexHull,
                poly_compound,
            );
        }
    }

    // maybe this should be a different edge kind
    layout.drawing.add_to_compound(
        recorder,
        apex,
        CompoundEntryKind::NotInConvexHull,
        poly_compound,
    );

    assert!(is_apex(&layout.drawing, apex));
    apex
}

fn is_apex<R>(drawing: &Drawing<CompoundWeight, CompoundEntryKind, R>, dot: FixedDotIndex) -> bool {
    !drawing
        .primitive(dot)
        .segs()
        .iter()
        .any(|seg| matches!(seg, SegIndex::Fixed(..)))
        && drawing.primitive(dot).bends().is_empty()
}

impl<'a, R> PolyRef<'a, R> {
    pub fn new(
        index: GenericIndex<PolyWeight>,
        drawing: &'a Drawing<CompoundWeight, CompoundEntryKind, R>,
    ) -> Self {
        Self { index, drawing }
    }

    pub fn apex(&self) -> FixedDotIndex {
        self.drawing
            .geometry()
            .compound_members(self.index.into())
            .find_map(|(kind, primitive_node)| {
                if kind == CompoundEntryKind::NotInConvexHull {
                    if let PrimitiveIndex::FixedDot(dot) = primitive_node {
                        if is_apex(self.drawing, dot) {
                            return Some(dot);
                        }
                    }
                }

                None
            })
            .unwrap()
    }
}

impl<R> GetLayer for PolyRef<'_, R> {
    fn layer(&self) -> usize {
        if let CompoundWeight::Poly(weight) = self.drawing.compound_weight(self.index.into()) {
            weight.layer()
        } else {
            unreachable!();
        }
    }
}

impl<R> GetMaybeNet for PolyRef<'_, R> {
    fn maybe_net(&self) -> Option<usize> {
        self.drawing.compound_weight(self.index.into()).maybe_net()
    }
}

impl<R> MakePolygon for PolyRef<'_, R> {
    fn shape(&self) -> Polygon {
        Polygon::new(
            LineString::from(
                self.drawing
                    .geometry()
                    .compound_members(self.index.into())
                    .filter_map(|(_kind, primitive_node)| {
                        let PrimitiveIndex::FixedDot(dot) = primitive_node else {
                            return None;
                        };

                        if is_apex(self.drawing, dot) {
                            None
                        } else {
                            Some(self.drawing.geometry().dot_weight(dot.into()).pos())
                        }
                    })
                    .collect::<Vec<Point>>(),
            ),
            vec![],
        )
    }
}

#[enum_dispatch(GetLayer, GetMaybeNet)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PolyWeight {
    Solid(SolidPolyWeight),
    Pour(PourPolyWeight),
}

impl From<GenericIndex<PolyWeight>> for GenericIndex<CompoundWeight> {
    fn from(poly: GenericIndex<PolyWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(poly.petgraph_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidPolyWeight {
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for SolidPolyWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for SolidPolyWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<SolidPolyWeight>> for GenericIndex<CompoundWeight> {
    fn from(poly: GenericIndex<SolidPolyWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(poly.petgraph_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PourPolyWeight {
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for PourPolyWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for PourPolyWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl From<GenericIndex<PourPolyWeight>> for GenericIndex<CompoundWeight> {
    fn from(poly: GenericIndex<PourPolyWeight>) -> Self {
        GenericIndex::<CompoundWeight>::new(poly.petgraph_index())
    }
}
