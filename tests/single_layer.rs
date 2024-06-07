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

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 2);
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
    //common::assert_number_of_conncomps(&mut autorouter, 4);
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
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}

#[test]
fn test_3pin_xlr_tht_female_to_tht_female() {
    let design = DsnDesign::load_from_file(
        "tests/single_layer/data/3pin_xlr_tht_female_to_tht_female/3pin_xlr_tht_female_to_tht_female.dsn"
    );
    let board = design.unwrap().make_board();

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    let file =
        File::open("tests/single_layer/data/3pin_xlr_tht_female_to_tht_female/autoroute_all.cmd")
            .unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let (mut autorouter, ..) = invoker.destruct();

    // FIXME: The routing result is pretty bad.
    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}
