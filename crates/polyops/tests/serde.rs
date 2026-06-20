//! serde round-trip tests for the public types.
//! Only compiled/run with `--features serde`.
#![cfg(feature = "serde")]

use polyops::{Geometry, Operation};

#[test]
fn geometry_polygon_round_trips() {
    let g = Geometry::Polygon(vec![vec![
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [0.0, 2.0],
        [0.0, 0.0],
    ]]);
    let json = serde_json::to_string(&g).expect("serialize");
    let back: Geometry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(g, back);
}

#[test]
fn geometry_multipolygon_round_trips() {
    let g = Geometry::MultiPolygon(vec![
        vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]],
        vec![vec![[5.0, 5.0], [6.0, 5.0], [6.0, 6.0], [5.0, 5.0]]],
    ]);
    let json = serde_json::to_string(&g).expect("serialize");
    let back: Geometry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(g, back);
}

#[test]
fn operation_round_trips() {
    for op in [
        Operation::Intersection,
        Operation::Union,
        Operation::Difference,
        Operation::Xor,
    ] {
        let json = serde_json::to_string(&op).expect("serialize");
        let back: Operation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(op, back);
    }
}
