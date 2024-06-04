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
    triangulation::GetTrianvertexIndex,
};

#[test]
fn test() {
    let design = DsnDesign::load_from_file(
        "tests/data/single_layer_tht_diode_bridge_rectifier/single_layer_tht_diode_bridge_rectifier.dsn",
    );
    let board = design.unwrap().make_board();

    let mut invoker = Invoker::new(Autorouter::new(board).unwrap());
    let file =
        File::open("tests/data/single_layer_tht_diode_bridge_rectifier/autoroute_all.cmd").unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let (mut autorouter, ..) = invoker.destruct();

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
        dbg!(primitive);
        for joined in autorouter
            .board()
            .layout()
            .drawing()
            .geometry()
            .joineds(primitive)
        {
            dbg!(joined);
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

        let Some(source_layername) = autorouter.board().layername(source_layer) else {
            continue;
        };

        let Some(target_layername) = autorouter.board().layername(target_layer) else {
            continue;
        };

        if source_layername != "F.Cu" || target_layername != "F.Cu" {
            continue;
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

        assert_eq!(source_net, target_net);
        let net = source_net.unwrap();

        if let Some(netname) = autorouter.board().netname(net) {
            dbg!(netname);
            dbg!(source_dot, target_dot);
            assert_eq!(
                unionfind.find(source_dot.node_index()),
                unionfind.find(target_dot.node_index())
            );
        }
    }
}
