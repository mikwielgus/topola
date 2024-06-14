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
}
