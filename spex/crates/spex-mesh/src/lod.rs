//! M59 — coarser levels of detail, generated from the real reference chain.
//!
//! A site four hundred metres "away" in the Atlas does not need its studs.
//! But "which triangles are the studs" is a question that must not be
//! answered by looking at the triangles.
//!
//! Review 01's finding **B5** is the whole design here. Rev 1 said LOD1 would
//! gate stud and tube removal "on the reference path, never on a heuristic" —
//! and then discovered that `resolve_into` threw the reference chain away, so
//! there was no path to gate on. The fix landed early, in M51: every triangle
//! and every edge carries a `source` index, and `PartGeometry::sources` holds
//! the real reference **chain** rather than the leaf file name.
//!
//! That distinction is what makes this work at all. `p/4-4cyli.dat` is a
//! quarter-cylinder primitive, and the same file is used for a stud, for an
//! underside tube, and for a hole through a technic beam. Its name settles
//! nothing. Its chain settles everything:
//!
//! ```text
//! parts/3001.dat > parts/s/3001s01.dat > p/stud.dat  > p/4-4cyli.dat   <- a stud
//! parts/3001.dat > parts/s/3001s01.dat > p/stud4.dat > p/4-4cyli.dat   <- a tube
//! parts/3001.dat > parts/s/3001s01.dat > p/box5.dat                    <- the wall
//! ```

use spex_ldraw::{Edge, PartGeometry};

/// LDraw's stud and tube primitives all begin with one of these, and nothing
/// else does.
///
/// `stud.dat` is the ordinary top stud; `stud2` is the open one, `stud3` and
/// `stud4` are the *underside* rings and tubes — which is why removing
/// "studs" also removes the tubes, and why the milestone names both. `stug*`
/// are the stud **groups** (`stug-1x4.dat` and friends), which exist only to
/// place several studs at once; anything reached through one is a stud by
/// construction.
const STUD_PREFIXES: [&str; 2] = ["stud", "stug"];

/// Does this reference chain pass through a stud or tube primitive?
///
/// Matched on the **file stem of each path segment**, never on the chain as a
/// substring. A part legitimately called `studless-beam.dat`… would in fact
/// match `stud`, which is why the check is anchored to LDraw's `p/` primitive
/// directory as well: these are primitives, and primitives live there.
pub fn chain_is_stud(chain: &str) -> bool {
    chain.split(" > ").any(|segment| {
        let Some(file) = segment.rsplit('/').next() else { return false };
        // Primitives only. A *part* whose name happens to start with "stud"
        // is a part, and removing it would delete real geometry.
        segment.starts_with("p/") && STUD_PREFIXES.iter().any(|p| file.starts_with(p))
    })
}

/// Which source indices of this part are studs or tubes.
pub fn stud_sources(geo: &PartGeometry) -> Vec<bool> {
    geo.sources.iter().map(|s| chain_is_stud(s)).collect()
}

/// **LOD1** — the same part with its studs and underside tubes gone.
///
/// Everything else is untouched: same vertices, same winding, same colours,
/// same `sources` table (so a triangle's `source` index still means what it
/// meant). Only whole triangles and whole edges are dropped, which is what
/// keeps the result a valid mesh rather than a torn one.
pub fn lod1(geo: &PartGeometry) -> PartGeometry {
    let is_stud = stud_sources(geo);
    let keep = |source: u16| !is_stud.get(source as usize).copied().unwrap_or(false);
    PartGeometry {
        triangles: geo.triangles.iter().filter(|t| keep(t.source)).cloned().collect(),
        edges: geo.edges.iter().filter(|e| keep(e.source)).cloned().collect(),
        sources: geo.sources.clone(),
        description: geo.description.clone(),
        license: geo.license.clone(),
        author: geo.author.clone(),
        uncertified: geo.uncertified.clone(),
    }
}

/// **LOD2** — the part's own bounding box. 12 triangles, 12 hard edges.
///
/// The milestone says "oriented bounding box", and for a real LDraw part that
/// is the axis-aligned one: parts are authored in their own frame, aligned to
/// the stud grid, and the instance's orientation matrix carries the rotation.
/// Computing a covariance-fitted OBB here would produce the same box with
/// more code and a worse guarantee.
///
/// Colour comes out as `None` — LDraw code 16, "inherit" — so a box takes the
/// colour of whatever brick it stands in for, which is the entire point.
pub fn lod2(geo: &PartGeometry) -> PartGeometry {
    let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for t in &geo.triangles {
        for v in t.vertices {
            for k in 0..3 {
                min[k] = min[k].min(v[k]);
                max[k] = max[k].max(v[k]);
            }
        }
    }
    if !min[0].is_finite() {
        return PartGeometry { sources: geo.sources.clone(), ..Default::default() };
    }

    // Corner numbering: bit 0 = x, bit 1 = y, bit 2 = z.
    let corner = |i: usize| {
        [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ]
    };

    // Each face as two triangles, wound so the normal points outward in
    // LDraw's own frame — the bundle writer reverses all of them at the
    // mirror, exactly as it does for real geometry, so a box behaves like
    // every other part under backface culling.
    const FACES: [[usize; 4]; 6] = [
        [0, 2, 3, 1], // -z
        [4, 5, 7, 6], // +z
        [0, 1, 5, 4], // -y
        [2, 6, 7, 3], // +y
        [0, 4, 6, 2], // -x
        [1, 3, 7, 5], // +x
    ];
    let mut triangles = Vec::with_capacity(12);
    for f in FACES {
        for [a, b, c] in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]] {
            triangles.push(spex_ldraw::FullTriangle {
                vertices: [corner(a), corner(b), corner(c)],
                color_code: None,
                source: 0,
            });
        }
    }

    // The twelve real edges of the box, so a distant brick still gets an
    // outline rather than becoming a smooth blob at exactly the distance
    // where its silhouette is all anyone can see.
    const BOX_EDGES: [[usize; 2]; 12] = [
        [0, 1], [1, 3], [3, 2], [2, 0],
        [4, 5], [5, 7], [7, 6], [6, 4],
        [0, 4], [1, 5], [2, 6], [3, 7],
    ];
    let edges = BOX_EDGES
        .iter()
        .map(|&[a, b]| Edge {
            vertices: [corner(a), corner(b)],
            color_code: None,
            kind: spex_ldraw::EdgeKind::Hard,
            source: 0,
        })
        .collect();

    PartGeometry {
        triangles,
        edges,
        sources: geo.sources.clone(),
        description: geo.description.clone(),
        license: geo.license.clone(),
        author: geo.author.clone(),
        uncertified: geo.uncertified.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spex_ldraw::{EdgeKind, FullTriangle};

    fn tri(source: u16) -> FullTriangle {
        FullTriangle { vertices: [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], color_code: None, source }
    }

    /// Real chains, copied from a real `3001.dat` resolve.
    const REAL_CHAINS: [&str; 8] = [
        "parts/3001.dat",
        "parts/3001.dat > parts/s/3001s01.dat",
        "parts/3001.dat > parts/s/3001s01.dat > p/stud4.dat",
        "parts/3001.dat > parts/s/3001s01.dat > p/stud4.dat > p/4-4cyli.dat",
        "parts/3001.dat > parts/s/3001s01.dat > p/box5.dat",
        "parts/3001.dat > parts/s/3001s01.dat > p/box3u2p.dat",
        "parts/3001.dat > parts/s/3001s01.dat > p/stud.dat",
        "parts/3001.dat > parts/s/3001s01.dat > p/stud.dat > p/4-4disc.dat",
    ];

    #[test]
    fn the_same_primitive_is_a_stud_or_not_depending_on_how_it_was_reached() {
        // This is the whole of B5 in one assertion. Both chains end in the
        // identical file; only the path tells them apart.
        assert!(chain_is_stud("parts/3001.dat > p/stud.dat > p/4-4cyli.dat"));
        assert!(!chain_is_stud("parts/3001.dat > p/box5.dat > p/4-4cyli.dat"));
    }

    #[test]
    fn real_chains_classify_correctly() {
        let got: Vec<bool> = REAL_CHAINS.iter().map(|c| chain_is_stud(c)).collect();
        assert_eq!(
            got,
            vec![false, false, true, true, false, false, true, true],
            "stud4 is the underside tube and stud is the top stud; box5/box3u2p are the walls"
        );
    }

    #[test]
    fn a_part_named_like_a_primitive_is_not_a_primitive() {
        // Anchored to `p/`, so a hypothetical part file cannot be deleted by
        // its name alone.
        assert!(!chain_is_stud("parts/studless-beam.dat"));
        assert!(!chain_is_stud("parts/3001.dat > parts/s/studs01.dat"));
    }

    #[test]
    fn lod1_drops_exactly_the_stud_sourced_geometry() {
        let geo = PartGeometry {
            triangles: vec![tri(0), tri(1), tri(2), tri(0)],
            edges: vec![Edge { vertices: [[0.0; 3], [1.0; 3]], color_code: None, kind: EdgeKind::Hard, source: 2 }],
            sources: vec![
                "parts/x.dat".into(),
                "parts/x.dat > p/box5.dat".into(),
                "parts/x.dat > p/stud.dat".into(),
            ],
            ..Default::default()
        };
        let l1 = lod1(&geo);
        assert_eq!(l1.triangles.len(), 3, "only the source-2 triangle goes");
        assert!(l1.edges.is_empty(), "its edges go with it");
        assert_eq!(l1.sources, geo.sources, "the sources table is untouched, so indices still mean what they meant");
    }

    #[test]
    fn lod2_is_twelve_triangles_and_twelve_edges_around_the_real_extent() {
        let geo = PartGeometry {
            triangles: vec![FullTriangle {
                vertices: [[-2.0, 0.0, -1.0], [4.0, 3.0, 5.0], [0.0, -1.0, 0.0]],
                color_code: Some(4),
                source: 0,
            }],
            sources: vec!["parts/x.dat".into()],
            ..Default::default()
        };
        let l2 = lod2(&geo);
        assert_eq!(l2.triangles.len(), 12);
        assert_eq!(l2.edges.len(), 12);
        let (mut mn, mut mx) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for t in &l2.triangles {
            for v in t.vertices {
                for k in 0..3 {
                    mn[k] = mn[k].min(v[k]);
                    mx[k] = mx[k].max(v[k]);
                }
            }
        }
        assert_eq!(mn, [-2.0, -1.0, -1.0]);
        assert_eq!(mx, [4.0, 3.0, 5.0]);
        assert!(
            l2.triangles.iter().all(|t| t.color_code.is_none()),
            "a box inherits the colour of the brick it stands in for"
        );
    }

    #[test]
    #[ignore = "resolves the real 3001.dat; network on a cold cache"]
    fn real_2x4_brick_lod1_reduction() {
        // M59 AC3, against the real part rather than a fixture.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(".ldraw-cache");
        let cache = spex_ldraw::LdrawCache::new(&root);
        let geo = spex_ldraw::resolve_part_full(&cache, "3001.dat").unwrap();
        let l1 = lod1(&geo);
        let l2 = lod2(&geo);
        let drop = 1.0 - (l1.triangles.len() as f64 / geo.triangles.len() as f64);
        println!(
            "3001.dat  LOD0 {} tris / {} edges | LOD1 {} tris / {} edges ({:.1}% fewer triangles) | LOD2 {} tris / {} edges",
            geo.triangles.len(), geo.edges.len(),
            l1.triangles.len(), l1.edges.len(), drop * 100.0,
            l2.triangles.len(), l2.edges.len(),
        );
        assert!(drop >= 0.55, "AC3 wants >= 55% fewer triangles, got {:.1}%", drop * 100.0);
        assert_eq!(l2.triangles.len(), 12);
    }

    #[test]
    fn lod2_of_nothing_is_nothing_rather_than_a_panic() {
        let l2 = lod2(&PartGeometry { sources: vec!["parts/x.dat".into()], ..Default::default() });
        assert!(l2.triangles.is_empty());
        assert!(l2.edges.is_empty());
    }
}
