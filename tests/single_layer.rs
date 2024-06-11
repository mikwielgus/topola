use std::fs::File;

use petgraph::{
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use topola::{
    autorouter::{
        invoker::{Command, Invoker, InvokerError},
        Autorouter, AutorouterError,
    },
    drawing::{
        graph::{GetLayer, GetMaybeNet},
        primitive::GetInnerOuter,
    },
    dsn::design::DsnDesign,
    graph::GetNodeIndex,
    layout::{via::ViaWeight, NodeIndex},
    math::Circle,
    router::EmptyRouterObserver,
    triangulation::GetTrianvertexIndex,
};

mod common;

#[test]
fn test_0603_breakout() {
    let mut invoker =
        common::load_design_and_assert("tests/single_layer/data/0603_breakout/0603_breakout.dsn");
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/data/0603_breakout/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 2);
}

#[test]
fn test_tht_diode_bridge_rectifier() {
    let mut invoker = common::load_design_and_assert(
        "tests/single_layer/data/tht_diode_bridge_rectifier/tht_diode_bridge_rectifier.dsn",
    );
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/data/tht_diode_bridge_rectifier/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 4);
    common::assert_band_length(autorouter.board(), "J2-2", "D4-2", 15500.0, 0.5);

    let mut invoker = Invoker::new(autorouter);
    let result = invoker.execute(
        Command::PlaceVia(ViaWeight {
            from_layer: 0,
            to_layer: 1,
            circle: Circle {
                pos: [0.0, 0.0].into(),
                r: 200000.0,
            },
            maybe_net: Some(1234),
        }),
        &mut EmptyRouterObserver,
    );
    let result = dbg!(result);
    assert!(matches!(
        result,
        Err(InvokerError::Autorouter(AutorouterError::CouldNotPlaceVia(
            ..
        )))
    ));
}

#[test]
fn test_four_3rd_order_smd_lc_filters() {
    let mut invoker = common::load_design_and_assert(
        "tests/single_layer/data/four_3rd_order_smd_lc_filters/four_3rd_order_smd_lc_filters.dsn",
    );
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/data/four_3rd_order_smd_lc_filters/autoroute_signals.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}

// FIXME: This test fails indeterministically.
#[test]
fn test_3pin_xlr_tht_female_to_tht_female() {
    let mut invoker = common::load_design_and_assert(
        "tests/single_layer/data/3pin_xlr_tht_female_to_tht_female/3pin_xlr_tht_female_to_tht_female.dsn",
    );
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/data/3pin_xlr_tht_female_to_tht_female/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    // FIXME: The routing result is pretty bad.
    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}
