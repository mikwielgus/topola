// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{fs::File, io::BufReader};

use topola::{
    autorouter::{
        conncomps::Conncomps,
        history::{History, HistoryError},
        invoker::{Invoker, InvokerError},
        ratline::RatlineUid,
        Autorouter,
    },
    board::{edit::BoardEdit, AccessMesadata, Board},
    drawing::{
        graph::{GetMaybeNet, MakePrimitiveRef, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::{shape::MeasureLength, GenericNode, GetLayer},
    graph::{GetIndex, MakeRef},
    router::{navmesh::Navmesh, RouterOptions},
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

pub fn load_design(filename: &str) -> Autorouter<SpecctraMesadata> {
    let design_file = File::open(filename).unwrap();
    let design_bufread = BufReader::new(design_file);
    let design = SpecctraDesign::load(design_bufread).unwrap();
    Autorouter::new(design.make_board(&mut BoardEdit::new())).unwrap()
}

pub fn create_invoker_and_assert(
    autorouter: Autorouter<SpecctraMesadata>,
) -> Invoker<SpecctraMesadata> {
    let mut invoker = Invoker::new(autorouter);

    assert!(matches!(
        invoker.undo(),
        Err(InvokerError::History(HistoryError::NoPreviousCommand))
    ));
    assert!(matches!(
        invoker.redo(),
        Err(InvokerError::History(HistoryError::NoNextCommand))
    ));

    invoker
}

pub fn replay_and_assert_and_report(
    invoker: &mut Invoker<SpecctraMesadata>,
    filename: &str,
    variant: &str,
) {
    let file = File::open(filename).unwrap();
    let history: History = serde_json::from_reader(file).unwrap();

    invoker.replay(history);
    report_route_lengths(invoker.autorouter());

    if variant == "with_undo_redo_replay" {
        assert_undo_redo_replay(invoker, filename);
    }
}

pub fn assert_undo_redo_replay(invoker: &mut Invoker<SpecctraMesadata>, filename: &str) {
    let prev_node_count = invoker.autorouter().board().layout().drawing().node_count();

    // Sanity test: check if node count remained the same after some attempts at undo-redo.

    if invoker.redo().is_ok() {
        let _ = invoker.undo();
    }

    if invoker.undo().is_ok() {
        if invoker.undo().is_ok() {
            let _ = invoker.redo();
        }

        let _ = invoker.redo();
    }

    assert_eq!(
        invoker.autorouter().board().layout().drawing().node_count(),
        prev_node_count,
    );

    // Another sanity test: undo all, and then replay again. This protects
    // against undo failing to remove something that does not cause the
    // subsequent redo to fail or something that the redo restores idempotently.

    let file = File::open(filename).unwrap();
    let history: History = serde_json::from_reader(file).unwrap();

    undo_all_and_assert(invoker);
    invoker.replay(history);

    assert_eq!(
        invoker.autorouter().board().layout().drawing().node_count(),
        prev_node_count,
    );
}

pub fn undo_all_and_assert(invoker: &mut Invoker<SpecctraMesadata>) {
    for _ in 0..invoker.history().done().len() {
        invoker.undo().unwrap();
    }

    assert!(matches!(
        invoker.undo(),
        Err(InvokerError::History(HistoryError::NoPreviousCommand))
    ));

    assert_no_loose_nodes(invoker.autorouter());
}

pub fn assert_no_loose_nodes(autorouter: &Autorouter<impl AccessMesadata>) {
    for node in autorouter.board().layout().drawing().primitive_nodes() {
        match node {
            PrimitiveIndex::LooseDot(..)
            | PrimitiveIndex::LoneLooseSeg(..)
            | PrimitiveIndex::SeqLooseSeg(..)
            | PrimitiveIndex::LooseBend(..) => assert!(false),
            _ => (),
        }
    }
}

pub fn assert_layer_0_navnode_count(
    autorouter: &mut Autorouter<SpecctraMesadata>,
    origin_pin: &str,
    destination_pin: &str,
    expected_count: usize,
) {
    let (origin, destination) = autorouter
        .ratsnests()
        .on_principal_layer(0)
        .graph()
        .edge_indices()
        .map(|index| RatlineUid {
            principal_layer: 0,
            index,
        })
        .collect::<Vec<_>>()
        .iter()
        .find_map(|ratline| {
            let (candidate_origin, candidate_destination) =
                ratline.ref_(autorouter).terminating_dots();
            let candidate_origin_pin = autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(candidate_origin.into()))
                .unwrap();
            let candidate_destination_pin = autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(candidate_destination.into()))
                .unwrap();

            ((candidate_origin_pin == origin_pin && candidate_destination_pin == destination_pin)
                || (candidate_origin_pin == destination_pin
                    && candidate_destination_pin == origin_pin))
                .then_some((candidate_origin, candidate_destination))
        })
        .unwrap();

    let navmesh = Navmesh::new(
        autorouter.board().layout(),
        origin,
        destination,
        RouterOptions {
            wrap_around_bands: true,
            squeeze_through_under_bends: false,
            routed_band_width: 100.0,
        },
    )
    .unwrap();
    assert_eq!(navmesh.graph().node_count(), expected_count);
}

pub fn assert_that_all_ratlines_besides_gnd_are_autorouted(
    autorouter: &mut Autorouter<impl AccessMesadata>,
) {
    let conncomps = Conncomps::new(autorouter.board());
    assert!(autorouter.board().layout().drawing().layer_count() >= 1);

    for principal_layer in 0..autorouter.board().layout().drawing().layer_count() {
        for ratline in autorouter
            .ratsnests()
            .on_principal_layer(principal_layer)
            .graph()
            .edge_indices()
            .map(|index| RatlineUid {
                principal_layer,
                index,
            })
        {
            let (origin_dot, destination_dot) = ratline.ref_(autorouter).endpoint_dots();

            let origin_net = autorouter
                .board()
                .layout()
                .drawing()
                .primitive(origin_dot)
                .maybe_net();
            let destination_net = autorouter
                .board()
                .layout()
                .drawing()
                .primitive(destination_dot)
                .maybe_net();
            assert_eq!(origin_net, destination_net);

            let net = origin_net.unwrap();

            if let Some(netname) = autorouter.board().layout().rules().net_netname(net) {
                // We don't route ground.
                let origin = conncomps.unionfind().find(origin_dot.index());
                let destination = conncomps.unionfind().find(destination_dot.index());

                if netname != "GND" {
                    assert_eq!(origin, destination);
                }
            }
        }
    }
}

/*pub fn assert_number_of_conncomps(
    autorouter: &mut Autorouter<impl MesadataTrait>,
    conncomp_count: usize,
) {
    let unionfind = unionfind(autorouter);
    let mut labels = unionfind.into_labeling();
    labels.sort_unstable();
    labels.dedup();

    assert_eq!(labels.len(), conncomp_count);
}*/

pub fn assert_band_length(
    board: &Board<impl AccessMesadata>,
    source_pin: &str,
    target_pin: &str,
    expected_length: f64,
    rel_err: f64,
) {
    let band = board.band_between_pins(source_pin, target_pin).unwrap();
    let band_length = band[false].ref_(board.layout().drawing()).length();
    assert!(
        (band_length - expected_length).abs() < expected_length * rel_err,
        "band_length = {}, expected_length = {}, epsilon = {}",
        band_length,
        expected_length,
        rel_err
    );
}

pub fn report_route_lengths(autorouter: &Autorouter<impl AccessMesadata>) {
    let mut total_length = 0.0;

    for layer in 0..autorouter.board().layout().drawing().layer_count() {
        let mut layer_total_length = 0.0;

        for primitive in autorouter
            .board()
            .layout()
            .drawing()
            .layer_primitive_nodes(layer)
        {
            match primitive {
                PrimitiveIndex::LooseDot(..)
                | PrimitiveIndex::LoneLooseSeg(..)
                | PrimitiveIndex::SeqLooseSeg(..) => {
                    layer_total_length += primitive
                        .primitive_ref(autorouter.board().layout().drawing())
                        .shape()
                        .length();
                }
                _ => (),
            }
        }

        dbg!(layer, layer_total_length);

        total_length += layer_total_length;
    }

    dbg!(total_length);
}
