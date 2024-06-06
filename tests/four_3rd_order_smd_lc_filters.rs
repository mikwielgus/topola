use std::fs::File;

use petgraph::{
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use topola::{
    autorouter::{invoker::Invoker, Autorouter},
    drawing::graph::GetMaybeNet,
    dsn::design::DsnDesign,
    graph::GetNodeIndex,
    triangulation::GetTrianvertexIndex,
};

mod common;

#[test]
fn test() {
    let design = DsnDesign::load_from_file(
        "tests/data/four_3rd_order_smd_lc_filters/four_3rd_order_smd_lc_filters.dsn",
    );
    let board = design.unwrap().make_board();

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    let file =
        File::open("tests/data/four_3rd_order_smd_lc_filters/autoroute_signals.cmd").unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}
