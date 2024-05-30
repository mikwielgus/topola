use petgraph::{
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use topola::{
    autorouter::{invoker::Invoker, Autorouter},
    dsn::design::DsnDesign,
    graph::GetNodeIndex,
    router::EmptyRouterObserver,
    triangulation::GetVertexIndex,
};

#[test]
fn test() {
    let design = DsnDesign::load_from_file("tests/data/0603_breakout/0603_breakout.dsn").unwrap();
    let mut invoker = Invoker::new(Autorouter::new(design.make_layout()).unwrap());
    let file = File::open("tests/data/0603_breakout/autoroute_all.cmd").unwrap();
    invoker.replay(serde_json::from_reader(file).unwrap());

    let mut unionfind = UnionFind::new(
        invoker
            .autorouter()
            .layout()
            .drawing()
            .geometry()
            .graph()
            .node_bound(),
    );

    for edge in invoker
        .autorouter()
        .layout()
        .drawing()
        .geometry()
        .graph()
        .edge_references()
    {
        unionfind.union(edge.source(), edge.target());
    }

    assert_eq!(
        invoker
            .autorouter()
            .ratsnest()
            .graph()
            .edge_indices()
            .collect::<Vec<_>>()
            .len(),
        2
    );

    for ratline in invoker.autorouter().ratsnest().graph().edge_references() {
        let from_index = invoker
            .autorouter()
            .ratsnest()
            .graph()
            .node_weight(ratline.source())
            .unwrap()
            .vertex_index()
            .node_index();
        let to_index = invoker
            .autorouter()
            .ratsnest()
            .graph()
            .node_weight(ratline.target())
            .unwrap()
            .vertex_index()
            .node_index();
        assert!(unionfind.equiv(from_index, to_index));
    }
}
