use std::{fs::File, io::BufReader};

use petgraph::{stable_graph::NodeIndex, unionfind::UnionFind, visit::NodeIndexable};
use topola::{
    autorouter::{
        history::HistoryError,
        invoker::{Invoker, InvokerError},
        Autorouter,
    },
    board::{mesadata::AccessMesadata, Board},
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet},
    },
    geometry::{shape::MeasureLength, GenericNode},
    graph::{GetPetgraphIndex, MakeRef},
    layout::LayoutEdit,
    router::{navmesh::Navmesh, RouterOptions},
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

pub fn load_design(filename: &str) -> Autorouter<SpecctraMesadata> {
    let design_file = File::open(filename).unwrap();
    let design_bufread = BufReader::new(design_file);
    let design = SpecctraDesign::load(design_bufread).unwrap();
    Autorouter::new(design.make_board(&mut LayoutEdit::new())).unwrap()
}

pub fn create_invoker_and_assert(
    autorouter: Autorouter<SpecctraMesadata>,
) -> Invoker<SpecctraMesadata> {
    let mut invoker = Invoker::new(autorouter);

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

pub fn replay_and_assert(invoker: &mut Invoker<SpecctraMesadata>, filename: &str) {
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

pub fn assert_navvertex_count(
    autorouter: &mut Autorouter<SpecctraMesadata>,
    origin_pin: &str,
    destination_pin: &str,
    expected_count: usize,
) {
    let (origin, destination) = autorouter
        .ratsnest()
        .graph()
        .edge_indices()
        .collect::<Vec<_>>()
        .iter()
        .find_map(|ratline| {
            let (candidate_origin, candidate_destination) = autorouter.ratline_endpoints(*ratline);
            let candidate_origin_pin = autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(candidate_origin.into()))
                .unwrap();
            let candidate_destination_pin = autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(candidate_destination.into()))
                .unwrap();

            ((candidate_origin_pin == origin_pin && candidate_destination_pin == destination_pin)
                || (candidate_origin_pin == destination_pin
                    && candidate_destination_pin == origin_pin))
                .then_some((candidate_origin, candidate_destination))
        })
        .unwrap();

    let navmesh = Navmesh::new(
        autorouter.board().layout(),
        origin,
        destination,
        RouterOptions {
            wrap_around_bands: true,
            squeeze_through_under_bands: false,
            routed_band_width: 100.0,
        },
    )
    .unwrap();
    assert_eq!(navmesh.graph().node_count(), expected_count);
}

pub fn assert_single_layer_groundless_autoroute(
    autorouter: &mut Autorouter<impl AccessMesadata>,
    layername: &str,
) {
    let unionfind = unionfind(autorouter);

    for ratline in autorouter.ratsnest().graph().edge_indices() {
        let (origin_dot, destination_dot) = autorouter.ratline_endpoints(ratline);

        let origin_layer = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(origin_dot)
            .layer();
        let destination_layer = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(destination_dot)
            .layer();

        if let (Some(origin_layername), Some(destination_layername)) = (
            autorouter
                .board()
                .layout()
                .rules()
                .layer_layername(origin_layer),
            autorouter
                .board()
                .layout()
                .rules()
                .layer_layername(destination_layer),
        ) {
            assert_eq!(origin_layername, destination_layername);

            if origin_layername != layername {
                continue;
            }
        } else {
            assert!(false);
        }

        let origin_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(origin_dot)
            .maybe_net();
        let destination_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(destination_dot)
            .maybe_net();
        assert_eq!(origin_net, destination_net);

        let net = origin_net.unwrap();

        if let Some(netname) = autorouter.board().layout().rules().net_netname(net) {
            // We don't route ground.
            let mut org = unionfind.find(origin_dot.petgraph_index());
            let mut desc = unionfind.find(destination_dot.petgraph_index());

            if netname != "GND" {
                assert_eq!(org, desc);
            }
        }
    }
}

/*pub fn assert_number_of_conncomps(
    autorouter: &mut Autorouter<impl MesadataTrait>,
    conncomp_count: usize,
) {
    let unionfind = unionfind(autorouter);
    let mut labels = unionfind.into_labeling();
    labels.sort_unstable();
    labels.dedup();

    assert_eq!(labels.len(), conncomp_count);
}*/

pub fn assert_band_length(
    board: &Board<impl AccessMesadata>,
    source_pin: &str,
    target_pin: &str,
    expected_length: f64,
    rel_err: f64,
) {
    let band = board.band_between_pins(source_pin, target_pin).unwrap();
    let band_length = band.0.ref_(board.layout().drawing()).length();
    assert!(
        (band_length - expected_length).abs() < expected_length * rel_err,
        "band_length = {}, expected_length = {}, epsilon = {}",
        band_length,
        expected_length,
        rel_err
    );
}

fn unionfind(autorouter: &mut Autorouter<impl AccessMesadata>) -> UnionFind<NodeIndex<usize>> {
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
            unionfind.union(primitive.petgraph_index(), joined.petgraph_index());
        }
    }

    unionfind
}
