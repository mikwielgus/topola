use topola::{
    autorouter::{
        invoker::{Command, InvokerError},
        AutorouterError,
    },
    board::mesadata::AccessMesadata,
    layout::via::ViaWeight,
    math::Circle,
};

mod common;

#[test]
fn test_unrouted_lm317_breakout() {
    let mut invoker = common::load_design_and_assert(
        "tests/multilayer/data/prerouted_lm317_breakout/unrouted_lm317_breakout.dsn",
    );

    let result = invoker.execute(Command::PlaceVia(ViaWeight {
        from_layer: 0,
        to_layer: 1,
        circle: Circle {
            pos: [125000.0, -84000.0].into(),
            r: 1000.0,
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
fn test_signal_integrity_test() {
    let invoker = common::load_design_and_assert(
        "tests/multilayer/data/signal_integrity_test/signal_integrity_test.dsn",
    );

    assert_eq!(
        invoker
            .autorouter()
            .board()
            .layout()
            .drawing()
            .layer_count(),
        4
    );

    for layer in 0..invoker
        .autorouter()
        .board()
        .layout()
        .drawing()
        .layer_count()
    {
        let layername = invoker
            .autorouter()
            .board()
            .mesadata()
            .layer_layername(layer);

        if layer == 0 {
            assert_eq!(layername, Some("F.Cu"));
        } else if layer == 1 {
            assert_eq!(layername, Some("In1.Cu"));
        } else if layer == 2 {
            assert_eq!(layername, Some("In2.Cu"));
        } else if layer == 3 {
            assert_eq!(layername, Some("B.Cu"));
        }
    }
}
