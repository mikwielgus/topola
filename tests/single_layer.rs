// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use rstest::rstest;
use rstest_reuse::{self, *};
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

#[template]
#[rstest]
#[case::plain("")]
#[case::with_undo_redo_replay("with_undo_redo_replay")]
fn test_master(#[case] variant: &str) {}

#[apply(test_master)]
fn autoroute_4x4_1206_led_matrix_breakout(variant: &str) {
    let autorouter = common::load_design(
        "tests/single_layer/4x4_1206_led_matrix_breakout/4x4_1206_led_matrix_breakout.dsn",
    );
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/4x4_1206_led_matrix_breakout/autoroute_all.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();
    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
}

#[apply(test_master)]
fn autoroute_4x4_1206_led_matrix_breakout_in_predefined_order(variant: &str) {
    let autorouter = common::load_design(
        "tests/single_layer/4x4_1206_led_matrix_breakout/4x4_1206_led_matrix_breakout.dsn",
    );
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/4x4_1206_led_matrix_breakout/autoroute_all_in_predefined_order.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();
    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
}

#[apply(test_master)]
fn autoroute_tht_de9_to_tht_de9(variant: &str) {
    let autorouter =
        common::load_design("tests/single_layer/tht_de9_to_tht_de9/tht_de9_to_tht_de9.dsn");
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::undo_all_and_assert(&mut invoker);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/tht_de9_to_tht_de9/autoroute_all.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();
    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
}

#[apply(test_master)]
fn autoroute_tht_de9_to_tht_de9_in_predefined_order(variant: &str) {
    let autorouter =
        common::load_design("tests/single_layer/tht_de9_to_tht_de9/tht_de9_to_tht_de9.dsn");
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/tht_de9_to_tht_de9/autoroute_all_in_predefined_order.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();
    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
}

#[apply(test_master)]
fn autoroute_0603_breakout(variant: &str) {
    let mut autorouter = common::load_design("tests/single_layer/0603_breakout/0603_breakout.dsn");
    common::assert_navnode_count(&mut autorouter, "R1-2", "J1-2", 22);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/0603_breakout/autoroute_all.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
    //common::assert_number_of_conncomps(&mut autorouter, 2);
}

#[apply(test_master)]
fn autoroute_tht_diode_bridge_rectifier(variant: &str) {
    let mut autorouter = common::load_design(
        "tests/single_layer/tht_diode_bridge_rectifier/tht_diode_bridge_rectifier.dsn",
    );
    common::assert_navnode_count(&mut autorouter, "J2-2", "D4-2", 68);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/tht_diode_bridge_rectifier/autoroute_all.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
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

#[apply(test_master)]
fn autoroute_4x_3rd_order_smd_lc_filters(variant: &str) {
    let mut autorouter = common::load_design(
        "tests/single_layer/4x_3rd_order_smd_lc_filters/4x_3rd_order_smd_lc_filters.dsn",
    );
    common::assert_navnode_count(&mut autorouter, "J1-1", "L1-1", 558);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/4x_3rd_order_smd_lc_filters/autoroute_signals.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}

// FIXME: This test fails indeterministically.
// NOTE: Disabled until determinism is fixed.
//#[test]
/*#[allow(unused)]
#[case("")]
#[case("with_undo_redo_replay")]
fn test_tht_3pin_xlr_to_tht_3pin_xlr(#[case] variant: &str) {
    let mut autorouter = common::load_design(
        "tests/single_layer/tht_3pin_xlr_to_tht_3pin_xlr/tht_3pin_xlr_to_tht_3pin_xlr.dsn",
    );
    //common::assert_navnode_count(&mut autorouter, "R1-2", "J1-2", ?);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/tht_3pin_xlr_to_tht_3pin_xlr/autoroute_all.cmd",
        "undo_redo_replay",
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(&mut autorouter, "F.Cu");
}*/

#[apply(test_master)]
fn autoroute_vga_dac_breakout(variant: &str) {
    let mut autorouter =
        common::load_design("tests/single_layer/vga_dac_breakout/vga_dac_breakout.dsn");
    common::assert_navnode_count(&mut autorouter, "J1-2", "R4-1", 272);
    let mut invoker = common::create_invoker_and_assert(autorouter);
    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/vga_dac_breakout/autoroute_all.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
}

#[apply(test_master)]
fn autoroute_smd_non_rectangular_buck_converter(variant: &str) {
    let path = "tests/single_layer/smd_non_rectangular_buck_converter/smd_non_rectangular_buck_converter.dsn";
    let autorouter = common::load_design(&path);

    let mut invoker = common::create_invoker_and_assert(autorouter);

    common::replay_and_assert_and_report(
        &mut invoker,
        "tests/single_layer/smd_non_rectangular_buck_converter/route_all.cmd",
        variant,
    );

    let (mut autorouter, ..) = invoker.dissolve();

    common::assert_that_all_single_layer_groundless_ratlines_are_autorouted(
        &mut autorouter,
        "F.Cu",
    );
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}
