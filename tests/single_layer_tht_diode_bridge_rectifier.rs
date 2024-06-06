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
fn test() {
    let design = DsnDesign::load_from_file(
        "tests/data/single_layer_tht_diode_bridge_rectifier/single_layer_tht_diode_bridge_rectifier.dsn",
    );
    let board = design.unwrap().make_board();

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    let file =
        File::open("tests/data/single_layer_tht_diode_bridge_rectifier/autoroute_all.cmd").unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    common::assert_band_length(autorouter.board(), "J2-2", "D4-2", 15511.0, 0.5);
}
