use std::fs::File;

use petgraph::{stable_graph::NodeIndex, unionfind::UnionFind, visit::NodeIndexable};
use topola::{
    autorouter::{
        board::Board,
        history::HistoryError,
        invoker::{Invoker, InvokerError},
        Autorouter,
    },
    drawing::{
        graph::{GetLayer, GetMaybeNet},
        rules::RulesTrait,
    },
    dsn::{design::DsnDesign, rules::DsnRules},
    graph::GetNodeIndex,
};

pub fn load_design_and_assert(filename: &str) -> Invoker<DsnRules> {
    let design = DsnDesign::load_from_file(filename).unwrap();
    let mut invoker = Invoker::new(Autorouter::new(design.make_board()).unwrap());

    assert!(matches!(
        invoker.undo(),
        Err(InvokerError::History(HistoryError::NoPreviousCommand))
    ));
    assert!(matches!(
        invoker.redo(),
        Err(InvokerError::History(HistoryError::NoNextCommand))
    ));

    invoker
}

pub fn replay_and_assert(invoker: &mut Invoker<DsnRules>, filename: &str) {
    let file = File::open(filename).unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let prev_node_count = invoker.autorouter().board().layout().drawing().node_count();

    // Sanity test: check if node count remained the same after some attempts at undo-redo.

    if invoker.redo().is_ok() {
        let _ = invoker.undo();
    }

    if invoker.undo().is_ok() {
        if invoker.undo().is_ok() {
            let _ = invoker.redo();
        }

        let _ = invoker.redo();
    }

    assert_eq!(
        invoker.autorouter().board().layout().drawing().node_count(),
        prev_node_count,
    );
}

pub fn assert_single_layer_groundless_autoroute(
    autorouter: &mut Autorouter<impl RulesTrait>,
    layername: &str,
) {
    let unionfind = unionfind(autorouter);

    for ratline in autorouter.ratsnest().graph().edge_indices() {
        let (source_dot, target_dot) = autorouter.ratline_endpoints(ratline);

        let source_layer = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(source_dot)
            .layer();
        let target_layer = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(target_dot)
            .layer();

        if let (Some(source_layername), Some(target_layername)) = (
            autorouter.board().layername(source_layer),
            autorouter.board().layername(target_layer),
        ) {
            dbg!(source_layername, target_layername);
            assert_eq!(source_layername, target_layername);

            if source_layername != layername {
                continue;
            }
        } else {
            assert!(false);
        }

        let source_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(source_dot)
            .maybe_net();
        let target_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(target_dot)
            .maybe_net();
        dbg!(source_net, target_net);
        assert_eq!(source_net, target_net);

        let net = source_net.unwrap();

        if let Some(netname) = autorouter.board().netname(net) {
            // We don't route ground.
            if netname != "GND" {
                dbg!(source_dot, target_dot);
                assert_eq!(
                    unionfind.find(source_dot.node_index()),
                    unionfind.find(target_dot.node_index())
                );
            }
        }
    }
}

/*pub fn assert_number_of_conncomps(
    autorouter: &mut Autorouter<impl RulesTrait>,
    conncomp_count: usize,
) {
    let unionfind = unionfind(autorouter);
    let mut labels = unionfind.into_labeling();
    labels.sort_unstable();
    labels.dedup();

    assert_eq!(labels.len(), conncomp_count);
}*/

pub fn assert_band_length(
    board: &Board<impl RulesTrait>,
    source: &str,
    target: &str,
    length: f64,
    epsilon: f64,
) {
    let band = board.band_between_pins(source, target).unwrap();
    let band_length = board.layout().band_length(band);
    dbg!(band_length);
    assert!((band_length - length).abs() < epsilon);
}

fn unionfind(autorouter: &mut Autorouter<impl RulesTrait>) -> UnionFind<NodeIndex<usize>> {
    for ratline in autorouter.ratsnest().graph().edge_indices() {
        // Accessing endpoints may create new dots because apex construction is lazy, so we access
        // tem all before starting unionfind, as it requires a constant index bound.
        let _ = autorouter.ratline_endpoints(ratline);
    }

    let mut unionfind = UnionFind::new(
        autorouter
            .board()
            .layout()
            .drawing()
            .geometry()
            .graph()
            .node_bound(),
    );

    for primitive in autorouter.board().layout().drawing().primitive_nodes() {
        for joined in autorouter
            .board()
            .layout()
            .drawing()
            .geometry()
            .joineds(primitive)
        {
            unionfind.union(primitive.node_index(), joined.node_index());
        }
    }

    unionfind
}
