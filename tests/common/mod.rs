use petgraph::{unionfind::UnionFind, visit::NodeIndexable};
use topola::{
    autorouter::{board::Board, Autorouter},
    drawing::{
        graph::{GetLayer, GetMaybeNet},
        rules::RulesTrait,
    },
    graph::GetNodeIndex,
};

pub fn assert_single_layer_groundless_autoroute(
    autorouter: &mut Autorouter<impl RulesTrait>,
    layername: &str,
) {
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
