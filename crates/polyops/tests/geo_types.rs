//! `geo-types` interop tests. Only compiled/run with `--features geo-types`.
#![cfg(feature = "geo-types")]

use geo_types as gt;
use polyops::{intersection, Geometry, ToGeo};

fn square(x0: f64, y0: f64, s: f64) -> gt::Polygon<f64> {
    gt::Polygon::new(
        gt::LineString(vec![
            gt::Coord { x: x0, y: y0 },
            gt::Coord { x: x0 + s, y: y0 },
            gt::Coord {
                x: x0 + s,
                y: y0 + s,
            },
            gt::Coord { x: x0, y: y0 + s },
            gt::Coord { x: x0, y: y0 },
        ]),
        vec![],
    )
}

#[test]
fn geo_polygon_into_geometry_and_back() {
    let p = square(0.0, 0.0, 2.0);
    let g: Geometry = p.clone().into();
    let back = g.to_geo();
    assert_eq!(back.0.len(), 1, "Polygon -> 1-polygon MultiPolygon");
    assert_eq!(back.0[0].exterior(), p.exterior());
}

#[test]
fn intersection_with_geo_inputs() {
    let a: Geometry = square(0.0, 0.0, 2.0).into();
    let b: Geometry = square(1.0, 1.0, 2.0).into();

    let result = intersection(a, b).expect("non-empty intersection");
    let geo = result.to_geo(); // op result (MultiPolygon alias) -> geo_types
    assert_eq!(geo.0.len(), 1, "overlap of two squares is one polygon");

    // the overlap is the unit square [1,2] x [1,2]
    let xs: Vec<f64> = geo.0[0].exterior().0.iter().map(|c| c.x).collect();
    let ys: Vec<f64> = geo.0[0].exterior().0.iter().map(|c| c.y).collect();
    assert!(xs.iter().all(|&x| (1.0..=2.0).contains(&x)));
    assert!(ys.iter().all(|&y| (1.0..=2.0).contains(&y)));
}

#[test]
fn interiors_are_preserved() {
    let outer = gt::LineString(vec![
        gt::Coord { x: 0.0, y: 0.0 },
        gt::Coord { x: 4.0, y: 0.0 },
        gt::Coord { x: 4.0, y: 4.0 },
        gt::Coord { x: 0.0, y: 4.0 },
        gt::Coord { x: 0.0, y: 0.0 },
    ]);
    let hole = gt::LineString(vec![
        gt::Coord { x: 1.0, y: 1.0 },
        gt::Coord { x: 2.0, y: 1.0 },
        gt::Coord { x: 2.0, y: 2.0 },
        gt::Coord { x: 1.0, y: 1.0 },
    ]);
    let g: Geometry = gt::Polygon::new(outer, vec![hole]).into();
    match &g {
        Geometry::Polygon(rings) => assert_eq!(rings.len(), 2, "exterior + 1 hole"),
        _ => panic!("expected Geometry::Polygon"),
    }
    assert_eq!(
        g.to_geo().0[0].interiors().len(),
        1,
        "hole survives round-trip"
    );
}
