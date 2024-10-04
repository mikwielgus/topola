use topola::{
    autorouter::{
        command::Command,
        invoker::{Invoker, InvokerError},
        AutorouterError,
    },
    layout::via::ViaWeight,
    math::Circle,
};

mod common;

#[test]
fn test_0603_breakout() {
    let mut invoker =
        common::load_design_and_assert("tests/single_layer/0603_breakout/0603_breakout.dsn");
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/0603_breakout/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 2);
}

#[test]
fn test_tht_diode_bridge_rectifier() {
    let mut invoker = common::load_design_and_assert(
        "tests/single_layer/tht_diode_bridge_rectifier/tht_diode_bridge_rectifier.dsn",
    );
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/tht_diode_bridge_rectifier/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 4);
    common::assert_band_length(autorouter.board(), "J2-2", "D4-2", 15900.0, 0.01);

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
    let mut invoker = common::load_design_and_assert(
        "tests/single_layer/4x_3rd_order_smd_lc_filters/4x_3rd_order_smd_lc_filters.dsn",
    );
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/4x_3rd_order_smd_lc_filters/autoroute_signals.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
    //common::assert_number_of_conncomps(&mut autorouter, 16);
}

// FIXME: This test fails indeterministically.
// NOTE: Disabled until determinism is fixed.
//#[test]
fn test_tht_3pin_xlr_to_tht_3pin_xlr() {
    let mut invoker = common::load_design_and_assert(
        "tests/single_layer/tht_3pin_xlr_to_tht_3pin_xlr/tht_3pin_xlr_to_tht_3pin_xlr.dsn",
    );
    common::replay_and_assert(
        &mut invoker,
        "tests/single_layer/tht_3pin_xlr_to_tht_3pin_xlr/autoroute_all.cmd",
    );

    let (mut autorouter, ..) = invoker.destruct();

    // FIXME: The routing result is pretty bad.
    common::assert_single_layer_groundless_autoroute(&mut autorouter, "F.Cu");
}
