use std::fs::File;

use petgraph::{
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use topola::{
    autorouter::{invoker::Invoker, Autorouter},
    drawing::{
        graph::{GetLayer, GetMaybeNet},
        primitive::GetInnerOuter,
    },
    dsn::design::DsnDesign,
    graph::GetNodeIndex,
    layout::NodeIndex,
    triangulation::GetTrianvertexIndex,
};

mod common;

#[test]
fn test_0603_breakout() {
    let design =
        DsnDesign::load_from_file("tests/single_layer/data/0603_breakout/0603_breakout.dsn")
            .unwrap();
    let mut invoker = Invoker::new(Autorouter::new(design.make_board()).unwrap());
    let file = File::open("tests/single_layer/data/0603_breakout/autoroute_all.cmd").unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let mut unionfind = UnionFind::new(
        invoker
            .autorouter()
            .board()
            .layout()
            .drawing()
            .geometry()
            .graph()
            .node_bound(),
    );

    for edge in invoker
        .autorouter()
        .board()
        .layout()
        .drawing()
        .geometry()
        .graph()
        .edge_references()
    {
        unionfind.union(edge.source(), edge.target());
    }

    assert_eq!(
        invoker
            .autorouter()
            .ratsnest()
            .graph()
            .edge_indices()
            .collect::<Vec<_>>()
            .len(),
        2
    );

    for ratline in invoker.autorouter().ratsnest().graph().edge_references() {
        let from_index = invoker
            .autorouter()
            .ratsnest()
            .graph()
            .node_weight(ratline.source())
            .unwrap()
            .trianvertex_index()
            .node_index();
        let to_index = invoker
            .autorouter()
            .ratsnest()
            .graph()
            .node_weight(ratline.target())
            .unwrap()
            .trianvertex_index()
            .node_index();
        assert_eq!(unionfind.find(from_index), unionfind.find(to_index));
    }
}

#[test]
fn test_tht_diode_bridge_rectifier() {
    let design = DsnDesign::load_from_file(
        "tests/single_layer/data/tht_diode_bridge_rectifier/tht_diode_bridge_rectifier.dsn",
    );
    let board = design.unwrap().make_board();

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    let file =
        File::open("tests/single_layer/data/tht_diode_bridge_rectifier/autoroute_all.cmd").unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    common::assert_band_length(autorouter.board(), "J2-2", "D4-2", 15511.0, 0.5);
}

#[test]
fn test_four_3rd_order_smd_lc_filters() {
    let design = DsnDesign::load_from_file(
        "tests/single_layer/data/four_3rd_order_smd_lc_filters/four_3rd_order_smd_lc_filters.dsn",
    );
    let board = design.unwrap().make_board();

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    let file =
        File::open("tests/single_layer/data/four_3rd_order_smd_lc_filters/autoroute_signals.cmd")
            .unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}
