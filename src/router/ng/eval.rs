// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::{algorithm::line_measures::metric_spaces::Euclidean, Distance};
use pie::{algo::pmg_astar::InsertionInfo, NavmeshIndex, RelaxedPath};
use std::collections::BTreeMap;

use crate::{
    drawing::{
        band::BandUid,
        dot::FixedDotIndex,
        graph::MakePrimitiveRef as _,
        head::{BareHead, GetFace as _, Head},
        primitive::MakePrimitiveShape as _,
        rules::AccessRules,
    },
    geometry::{primitive::PrimitiveShape, shape::AccessShape as _, shape::MeasureLength as _},
    graph::{GenericIndex, GetPetgraphIndex as _},
    layout::{poly::PolyWeight, CompoundWeight},
    math::{poly_ext_handover, RotationSense},
    router::{
        draw::Draw,
        ng::{
            pie, Alignment, AstarContext, Common, EtchedPath, EvalException, FloatingRouting,
            PieNavmeshBase, PieNavmeshRef, PolygonRouting, SubContext,
        },
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Etched {
    Core(FixedDotIndex),
    Path(EtchedPath),
}

impl Etched {
    fn resolve(
        &self,
        bands: &BTreeMap<EtchedPath, BandUid>,
    ) -> Result<Option<BandUid>, EvalException> {
        Ok(match self {
            Etched::Path(ep) => Some(ep.resolve_to_uid(bands)?),
            _ => None,
        })
    }
}

impl<'a> TryFrom<&'a RelaxedPath<EtchedPath, ()>> for Etched {
    type Error = ();

    fn try_from(x: &'a RelaxedPath<EtchedPath, ()>) -> Result<Etched, ()> {
        match x {
            RelaxedPath::Weak(()) => Err(()),
            RelaxedPath::Normal(ep) => Ok(Etched::Path(ep.clone())),
        }
    }
}

impl AstarContext {
    fn evaluate_navmesh_intern<R: AccessRules + Clone>(
        navmesh: PieNavmeshRef<'_>,
        ctx: &Self,
        common: &Common<R>,
        ins_info: InsertionInfo<PieNavmeshBase>,
    ) -> Result<(f64, Self), EvalException> {
        let (start_idx, end_idx) = (ins_info.prev_node, ins_info.cur_node);
        if !common.allowed_edges.is_empty() {
            let edge_idx = pie::navmesh::OrderedPair::from((start_idx, end_idx));
            if !common.allowed_edges.contains(&edge_idx) {
                return Err(EvalException::EdgeDisallowed(edge_idx));
            }
        }

        let mut sub = if let Some(x) = ins_info.maybe_new_goal {
            // start processing a new goal
            let face = match start_idx {
                NavmeshIndex::Primal(prim) => prim,
                NavmeshIndex::Dual(_) => panic!("invalid goal initialization"),
            };
            SubContext {
                label: x,
                active_head: BareHead { face }.into(),
                polygon: None,
                floating: None,
            }
        } else {
            ctx.sub.as_ref().expect("no goal initialized").clone()
        };

        let mut layout = ctx.last_layout(common);
        let mut recorder = ctx.recorder.clone();

        let width = *common.widths.get(&sub.label).expect("no width given");

        let edge_meta = ins_info.edge_meta;
        // paths on edge are ordered from edge_meta.lhs, to edge_meta.rhs
        let edge_paths = navmesh.access_edge_paths(ins_info.epi);

        debug_assert_eq!(
            edge_paths.as_ref()[ins_info.intro],
            RelaxedPath::Normal(sub.label.clone())
        );

        let to_pos = navmesh.node_data(&end_idx).unwrap().pos;
        let to_pos = geo::point! { x: to_pos.x, y: to_pos.y };

        match (start_idx, end_idx) {
            (NavmeshIndex::Primal(_), _) => {
                // no alignment to handle, handle like `floating`
                // TODO: keep track of what is left and right to us
                //sub.append_to_center_poly(&layout, to_pos, None);
                //sub.check_center_poly(&layout, common.active_layer)?;
                // TODO: prevent any wrapping around the start
                Ok((
                    ctx.length + Euclidean::distance(sub.head_center(&layout), to_pos),
                    AstarContext {
                        recorder,
                        bands: ctx.bands.clone(),
                        length: ctx.length,
                        sub: Some(sub),
                    },
                ))
            }
            (_, NavmeshIndex::Primal(prim)) => {
                //if let Some(mut floating) = sub.floating.take() {
                // this would be overly strict
                //floating.push(&layout, sub.active_head.face(), to_pos, Some(prim));
                //}
                let mut length = ctx.length;
                if let Some(old_poly) = sub.polygon.take() {
                    if prim != old_poly.apex {
                        let destination = prim.primitive_ref(layout.drawing()).shape().center();
                        let exit = old_poly.entry_point(destination, true)?;
                        let (new_head, length_delta) = old_poly.route_to_exit(
                            &mut layout,
                            &mut recorder,
                            sub.active_head,
                            exit,
                            width,
                        )?;
                        sub.active_head = new_head;
                        length += length_delta;
                    }
                }

                let fin = layout.finish_in_dot(
                    &mut recorder.layout_edit,
                    sub.active_head,
                    prim,
                    width,
                )?;
                length += sub
                    .active_head
                    .maybe_cane()
                    .map(|cane| cane.bend.primitive_ref(layout.drawing()).shape().length())
                    .unwrap_or(0.0);
                length += {
                    match fin.primitive_ref(layout.drawing()).shape() {
                        PrimitiveShape::Dot(_) => unreachable!(),
                        PrimitiveShape::Seg(seg) => seg.length(),
                        PrimitiveShape::Bend(bend) => bend.length(),
                    }
                };
                let mut bands = ctx.bands.clone();
                bands.insert(
                    sub.label.clone(),
                    layout
                        .drawing()
                        .find_loose_band_uid(fin.into())
                        .expect("a completely routed band should've Seg's as ends"),
                );
                Ok((
                    length,
                    AstarContext {
                        recorder,
                        bands,
                        length,
                        sub: Some(sub),
                    },
                ))
            }
            _ => {
                let edge_paths = edge_paths.as_ref();

                let alignment = match (edge_meta.lhs, edge_meta.rhs) {
                    (Some(_), Some(_)) => {
                        let mut alignment = Alignment::Left;
                        for _ in edge_paths
                            .iter()
                            .take(ins_info.intro)
                            .filter(|i| matches!(i, RelaxedPath::Weak(())))
                        {
                            alignment.incr_inplace();
                        }
                        alignment
                    }
                    (Some(_), None) => Alignment::Left,
                    (None, Some(_)) => Alignment::Right,
                    // this should only happen when one end-point is primal, handled above
                    (None, None) => unreachable!(),
                };

                let (lhs, rhs) = (
                    if let Some(lhs_idx) = ins_info.intro.checked_sub(1) {
                        (&edge_paths[lhs_idx]).try_into().ok()
                    } else {
                        edge_meta.lhs.map(Etched::Core)
                    },
                    if ins_info.intro + 1 < edge_paths.len() {
                        (&edge_paths[ins_info.intro + 1]).try_into().ok()
                    } else {
                        edge_meta.rhs.map(Etched::Core)
                    },
                );

                if lhs == Some(Etched::Path(sub.label.clone()))
                    || rhs == Some(Etched::Path(sub.label.clone()))
                {
                    return Err(EvalException::RouteBouncedBack);
                }

                let next_floating = match (edge_meta.lhs, edge_meta.rhs) {
                    (Some(lhs), Some(rhs)) => Some(FloatingRouting::new(
                        &layout,
                        sub.active_head.face(),
                        lhs.primitive_ref(&layout.drawing()).shape().center(),
                        rhs.primitive_ref(&layout.drawing()).shape().center(),
                    )),
                    _ => None,
                };

                if let Some(floating) = &mut sub.floating {
                    if let Some(next_floating) = next_floating {
                        *floating = floating.push(&next_floating, sub.active_head.face())?;
                    } else {
                        sub.floating = None;
                    }
                }

                let (wrap_etched, wrap_core, cw) = match alignment {
                    Alignment::Center => {
                        if sub.floating.is_none() {
                            sub.floating = next_floating;
                        }
                        return Ok((
                            ctx.length + Euclidean::distance(sub.head_center(&layout), to_pos),
                            AstarContext {
                                recorder,
                                bands: ctx.bands.clone(),
                                length: ctx.length,
                                sub: Some(sub),
                            },
                        ));
                    }
                    Alignment::Left => (
                        lhs.unwrap(),
                        edge_meta.lhs.unwrap(),
                        RotationSense::Counterclockwise,
                    ),
                    Alignment::Right => (
                        rhs.unwrap(),
                        edge_meta.rhs.unwrap(),
                        RotationSense::Clockwise,
                    ),
                };

                // we left the floating context above
                sub.floating = None;

                let current_poly = layout
                    .drawing()
                    .compounds(GenericIndex::<()>::new(wrap_core.petgraph_index()))
                    .find_map(|(_, compound)| {
                        if let CompoundWeight::Poly(_) = layout.drawing().compound_weight(compound)
                        {
                            Some(compound)
                        } else {
                            None
                        }
                    })
                    .map(|compound| GenericIndex::<PolyWeight>::new(compound.petgraph_index()));

                let (active_head, length_delta) = match (
                    sub.polygon.take(),
                    current_poly,
                    wrap_etched,
                ) {
                    (Some(existing_poly), Some(current_poly), _)
                        if existing_poly.idx == current_poly =>
                    {
                        // still the same polygon
                        let old_uid = existing_poly.inner;
                        let new_uid = wrap_etched.resolve(&ctx.bands)?;
                        return if old_uid != new_uid {
                            log::warn!(
                                "encountered changing inner band around polygon with apex={:?}",
                                existing_poly.apex
                            );
                            Err(EvalException::InnerPathChangedAroundPolygon {
                                apex: existing_poly.apex,
                                old_uid,
                                new_uid,
                            })
                        } else {
                            sub.polygon = Some(existing_poly);
                            Ok((
                                ctx.length,
                                AstarContext {
                                    recorder,
                                    bands: ctx.bands.clone(),
                                    length: ctx.length,
                                    sub: Some(sub),
                                },
                            ))
                        };
                    }
                    (None, _, Etched::Core(dot)) if sub.is_end_point(dot) => {
                        // `dot` is already the goal (even if it is inside a polygon)
                        // justification: that we manage to wrap directly around the goal means
                        // that there is also a shorter way to the goal
                        return Err(EvalException::UnnecessaryWrapAroundEndpoint);
                    }
                    (Some(old_poly), new_poly, _)
                        if new_poly.is_none()
                            || matches!(wrap_etched, Etched::Core(dot) if sub.is_end_point(dot)) =>
                    {
                        log::debug!(
                            "routing away from polygon with apex={:?}, wrap around {:?} with head {:?}",
                            old_poly.apex,
                            wrap_etched,
                            sub.active_head
                        );

                        let destination = sub.head_center(&layout);
                        let exit = old_poly.entry_point(destination, true)?;
                        let (new_head, mut length_delta) = old_poly.route_to_exit(
                            &mut layout,
                            &mut recorder,
                            sub.active_head,
                            exit,
                            width,
                        )?;
                        sub.active_head = new_head;

                        let next_head = super::cane_around(
                            &mut layout,
                            &mut recorder,
                            &mut length_delta,
                            sub.active_head,
                            wrap_core,
                            wrap_etched.resolve(&ctx.bands)?,
                            cw,
                            width,
                        )?;
                        (Head::Cane(next_head), length_delta)
                    }
                    // handled above
                    (Some(_), None, _) => unreachable!(),
                    (None, None, _) => {
                        let mut length_delta = 0.0;
                        let next_head = super::cane_around(
                            &mut layout,
                            &mut recorder,
                            &mut length_delta,
                            sub.active_head,
                            wrap_core,
                            wrap_etched.resolve(&ctx.bands)?,
                            cw,
                            width,
                        )?;
                        (Head::Cane(next_head), length_delta)
                    }
                    (Some(old_poly), Some(current_poly), _) => {
                        log::debug!(
                            "routing at polygon with apex={:?}, wrap around {:?} with head {:?}",
                            old_poly.apex,
                            wrap_etched,
                            sub.active_head
                        );

                        let mut poly = PolygonRouting::new(&layout, cw, current_poly, wrap_core);
                        let (exit, entry) = match poly_ext_handover(
                            &old_poly.convex_hull,
                            old_poly.cw,
                            &poly.convex_hull,
                            poly.cw,
                        ) {
                            None => {
                                return Err(EvalException::InvalidPolyHandoverData {
                                    source_poly_ext: old_poly.convex_hull.clone(),
                                    source_sense: old_poly.cw,
                                    target_poly_ext: poly.convex_hull.clone(),
                                    target_sense: poly.cw,
                                })
                            }
                            Some(x) => x,
                        };
                        // TODO: also handle bends around polygons...
                        // if the polygon encloses the head already, we can't route.
                        poly.entry_point = Some(entry);
                        poly.inner = wrap_etched.resolve(&ctx.bands)?;

                        log::debug!("exit point = {:?}; wrap around polygon {:?}", exit, poly,);

                        let (new_head, mut length_delta) = old_poly.route_to_exit(
                            &mut layout,
                            &mut recorder,
                            sub.active_head,
                            exit,
                            width,
                        )?;
                        sub.active_head = new_head;

                        let (res, length_delta2) = poly.route_to_entry(
                            &mut layout,
                            &mut recorder,
                            sub.active_head,
                            entry,
                            width,
                        )?;
                        length_delta += length_delta2;
                        sub.polygon = Some(poly);
                        (res, length_delta)
                    }
                    (None, Some(current_poly), _) => {
                        log::debug!(
                            "routing into polygon with apex={:?}, idx={:?}, wrap_around {:?} with head {:?}",
                            wrap_core,
                            current_poly,
                            wrap_etched,
                            sub.active_head
                        );

                        let mut poly = PolygonRouting::new(&layout, cw, current_poly, wrap_core);
                        let source = sub.head_center(&layout);
                        // if the polygon encloses the head already, we can't route.
                        let dot = poly.entry_point(source, false)?;
                        poly.entry_point = Some(dot);
                        poly.inner = wrap_etched.resolve(&ctx.bands)?;
                        log::debug!(
                            "wrap around polygon {:?}: entry point = {:?}, cw? {:?}",
                            poly,
                            dot,
                            cw
                        );

                        let res = poly.route_to_entry(
                            &mut layout,
                            &mut recorder,
                            sub.active_head,
                            dot,
                            width,
                        )?;
                        sub.polygon = Some(poly);
                        res
                    }
                };

                sub.active_head = active_head;
                let length = ctx.length + length_delta;
                Ok((
                    length,
                    AstarContext {
                        recorder,
                        bands: ctx.bands.clone(),
                        length,
                        sub: Some(sub),
                    },
                ))
            }
        }
    }

    pub fn evaluate_navmesh<R: AccessRules + Clone + std::panic::RefUnwindSafe>(
        navmesh: PieNavmeshRef<'_>,
        ctx: &Self,
        common: &Common<R>,
        ins_info: InsertionInfo<PieNavmeshBase>,
    ) -> Result<(f64, Self), EvalException> {
        std::panic::catch_unwind(move || {
            Self::evaluate_navmesh_intern(navmesh, ctx, common, ins_info)
        })
        .map_err(|e| EvalException::Panic(e.into()))
        .and_then(core::convert::identity)
    }
}
