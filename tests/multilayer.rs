use topola::board::mesadata::MesadataTrait;

mod common;

#[test]
fn test_prerouted_lm317_breakout() {
    let mut invoker = common::load_design_and_assert(
        "tests/multilayer/data/prerouted_lm317_breakout/unrouted_lm317_breakout.dsn",
    );
}

#[test]
fn test_signal_integrity_test() {
    let mut invoker = common::load_design_and_assert(
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
