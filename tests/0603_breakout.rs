use petgraph::{
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, NodeIndexable},
};
use std::sync::{Arc, Mutex};

use topola::{
    autorouter::Autorouter, dsn::design::DsnDesign, graph::GetNodeIndex,
    router::EmptyRouterObserver, triangulation::GetVertexIndex,
};

#[test]
fn test() {
    let design = DsnDesign::load_from_file("tests/data/0603_breakout/0603_breakout.dsn").unwrap();
    let layout_arc_mutex = Arc::new(Mutex::new(design.make_layout()));

    let mut autorouter = Autorouter::new(layout_arc_mutex.clone()).unwrap();
    autorouter.autoroute(0, &mut EmptyRouterObserver);

    let layout = layout_arc_mutex.lock().unwrap();
    let mut unionfind = UnionFind::new(layout.drawing().geometry().graph().node_bound());

    for edge in layout.drawing().geometry().graph().edge_references() {
        unionfind.union(edge.source(), edge.target());
    }

    assert_eq!(
        autorouter
            .ratsnest()
            .graph()
            .edge_indices()
            .collect::<Vec<_>>()
            .len(),
        2
    );

    for ratline in autorouter.ratsnest().graph().edge_references() {
        let from_index = autorouter
            .ratsnest()
            .graph()
            .node_weight(ratline.source())
            .unwrap()
            .vertex_index()
            .node_index();
        let to_index = autorouter
            .ratsnest()
            .graph()
            .node_weight(ratline.target())
            .unwrap()
            .vertex_index()
            .node_index();
        assert!(unionfind.equiv(from_index, to_index));
    }
}
