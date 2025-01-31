// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

#![cfg(feature = "serde")]

use pie::{DualIndex, NavmeshIndex, RelaxedPath};
use planar_incr_embed as pie;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
enum PrimitiveIndex {
    FixedDot(usize),
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
enum LayoutNodeIndex {
    Primitive(PrimitiveIndex),
}

#[derive(Clone)]
struct MyNavmeshBase;
type EtchedPath = pie::navmesh::EdgeIndex<LayoutNodeIndex>;

impl pie::NavmeshBase for MyNavmeshBase {
    type PrimalNodeIndex = LayoutNodeIndex;
    type EtchedPath = EtchedPath;
    type GapComment = ();
    type Scalar = f64;
}

type Navmesh = pie::navmesh::Navmesh<MyNavmeshBase>;

#[test]
fn tht_3pin_xlr_to_tht_3pin_xlr() {
    // the intermediate navmesh

    let navmesh_ser: pie::navmesh::NavmeshSer<MyNavmeshBase> = ron::from_str(
        &std::fs::read_to_string("tests/tht_3pin_xlr_to_tht_3pin_xlr/navmesh_intermed.ron")
            .unwrap(),
    )
    .unwrap();

    let mut navmesh: Navmesh = navmesh_ser.into();

    /*
    let _goals: Vec<EtchedPath> = ron::from_str(
        &std::fs::read_to_string("tests/tht_3pin_xlr_to_tht_3pin_xlr/goals.ron").unwrap(),
    )
    .unwrap();
    */

    // execute the path of the second goal "2 -> 8"

    let p2 = LayoutNodeIndex::Primitive(PrimitiveIndex::FixedDot(2));
    let p8 = LayoutNodeIndex::Primitive(PrimitiveIndex::FixedDot(8));

    let label = RelaxedPath::Normal(EtchedPath::from((p2, p8)));
    let p2 = NavmeshIndex::Primal(p2);
    let d5 = NavmeshIndex::Dual(DualIndex::Inner(5));
    let d6 = NavmeshIndex::Dual(DualIndex::Inner(6));

    let next_pos = navmesh
        .as_ref()
        .planarr_find_other_end(&d6, &p2, 0, false, &d5)
        .unwrap()
        .1;
    let mut tmp = navmesh.as_mut();
    tmp.edge_data_mut(d6, p2).unwrap().with_borrow_mut(|mut j| {
        j.insert(0, label.clone());
    });
    let next_pos = tmp.edge_data_mut(d6, d5).unwrap().with_borrow_mut(|mut j| {
        j.insert(next_pos.insert_pos, label);
        j.len() - next_pos.insert_pos - 1
    });

    insta::assert_json_snapshot!(navmesh
        .as_ref()
        .planarr_find_all_other_ends(&d5, &d6, next_pos, true)
        .unwrap()
        .1
        .collect::<Vec<_>>());
}
