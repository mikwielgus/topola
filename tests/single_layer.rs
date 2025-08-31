// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use topola::{
    autorouter::{
        execution::Command,
        invoker::{Invoker, InvokerError},
        AutorouterError,
    },
    layout::via::ViaWeight,
    math::Circle,
};

mod common;

#[test]
fn test_tht_de9_to_tht_de9() {
    let autorouter =
        common::load_design("tests/single_layer/tht_de9_to_tht_de9/tht_de9_to_tht_de9.dsn");
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/tht_de9_to_tht_de9/autoroute_all_in_an_order.cmd",
    );

    let (mut autorouter, history, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");

    invoker = Invoker::new_with_history(autorouter, history);
    common::undo_all_and_assert(&mut invoker);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/tht_de9_to_tht_de9/autoroute_all.cmd",
    );
}

#[test]
fn test_0603_breakout() {
    let mut autorouter = common::load_design("tests/single_layer/0603_breakout/0603_breakout.dsn");
    common::assert_navnode_count(&mut autorouter, "R1-2", "J1-2", 22);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/0603_breakout/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 2);
}

#[test]
fn test_tht_diode_bridge_rectifier() {
    let mut autorouter = common::load_design(
        "tests/single_layer/tht_diode_bridge_rectifier/tht_diode_bridge_rectifier.dsn",
    );
    common::assert_navnode_count(&mut autorouter, "J2-2", "D4-2", 68);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/tht_diode_bridge_rectifier/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 4);
    common::assert_band_length(autorouter.board(), "J2-2", "D4-2", 15906.760439007436, 0.01);

    let mut invoker = Invoker::new(autorouter);
    let result = invoker.execute(Command::PlaceVia(ViaWeight {
        from_layer: 0,
        to_layer: 1,
        circle: Circle {
            pos: [0.0, 0.0].into(),
            r: 200000.0,
        },
        maybe_net: Some(1234),
    }));
    assert!(matches!(
        result,
        Err(InvokerError::Autorouter(AutorouterError::CouldNotPlaceVia(
            ..
        )))
    ));
}

#[test]
fn test_4x_3rd_order_smd_lc_filters() {
    let mut autorouter = common::load_design(
        "tests/single_layer/4x_3rd_order_smd_lc_filters/4x_3rd_order_smd_lc_filters.dsn",
    );
    common::assert_navnode_count(&mut autorouter, "J1-1", "L1-1", 558);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/4x_3rd_order_smd_lc_filters/autoroute_signals.cmd",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}

// FIXME: This test fails indeterministically.
// NOTE: Disabled until determinism is fixed.
//#[test]
#[allow(unused)]
fn test_tht_3pin_xlr_to_tht_3pin_xlr() {
    let mut autorouter = common::load_design(
        "tests/single_layer/tht_3pin_xlr_to_tht_3pin_xlr/tht_3pin_xlr_to_tht_3pin_xlr.dsn",
    );
    //common::assert_navnode_count(&mut autorouter, "R1-2", "J1-2", ?);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/tht_3pin_xlr_to_tht_3pin_xlr/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}

#[test]
fn test_vga_dac_breakout() {
    let mut autorouter =
        common::load_design("tests/single_layer/vga_dac_breakout/vga_dac_breakout.dsn");
    common::assert_navnode_count(&mut autorouter, "J1-2", "R4-1", 272);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/vga_dac_breakout/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}

#[test]
fn test_smd_non_rectangular_buck_converter() {
    let path = "tests/single_layer/smd_non_rectangular_buck_converter/smd_non_rectangular_buck_converter.dsn";
    let autorouter = common::load_design(&path);

    let mut invoker = common::create_invoker_and_assert(autorouter);

    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/smd_non_rectangular_buck_converter/route_all.cmd",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}
