//! Smoke tests — exercise the public surface enough that the type
//! signatures compile and a build regression surfaces in CI even while
//! the algorithm is still stubbed.

use polyops::{Geometry, Polygon};

#[test]
fn public_types_compile() {
    /* Confirm the public types and From impls round-trip cleanly. */
    let polygon: Polygon = vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]];
    let _geom: Geometry = polygon.clone().into();
    let _multi: Geometry = vec![polygon].into();
}
