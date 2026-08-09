//! Proves `grid::FootprintTable::standard()`'s real, cited numbers are
//! actually what the real cached LDraw geometry measures — not typed from
//! memory. `#[ignore]`d because it needs a populated `.ldraw-cache/`
//! (network on a cold cache), the same convention `spex-ldraw`'s own
//! network-dependent tests already use.
//!
//! Run with: `cargo test -p spex-build --test footprint_provenance -- --ignored`
use spex_build::grid::FootprintTable;
use spex_ldraw::{resolve_part_full, LdrawCache};

#[test]
#[ignore = "reads the real local .ldraw-cache/ (or network on a cold cache)"]
fn standard_footprints_match_the_real_cached_ldraw_geometry() {
    let cache = LdrawCache::new(".ldraw-cache");
    let table = FootprintTable::standard();

    for part in ["3005.dat", "3004.dat", "3010.dat", "3710.dat", "2431.dat", "3009.dat", "3022.dat", "3020.dat"] {
        let geometry = resolve_part_full(&cache, part).unwrap_or_else(|e| panic!("resolving real part {part}: {e:?}"));
        let mut min = [f64::MAX; 3];
        let mut max = [f64::MIN; 3];
        for t in &geometry.triangles {
            for v in &t.vertices {
                for i in 0..3 {
                    min[i] = min[i].min(v[i]);
                    max[i] = max[i].max(v[i]);
                }
            }
        }
        let studs_w = ((max[0] - min[0]) / 20.0).round() as u32;
        let studs_d = ((max[2] - min[2]) / 20.0).round() as u32;
        // The real stacking pitch is the part's own max Y (bottom of its
        // underside tube in LDraw's down-positive frame) — NOT the raw
        // bbox extent, which over-counts the stud's protrusion above the
        // Y=0 reference plane. See `grid::FootprintTable`'s doc comment.
        let height_plates = (max[1] / 8.0).round() as u32;

        let expected = table.get(part).unwrap_or_else(|| panic!("no footprint entry for real part {part}"));
        assert_eq!(expected.studs_w, studs_w, "{part}: real measured width");
        assert_eq!(expected.studs_d, studs_d, "{part}: real measured depth");
        assert_eq!(expected.height_plates, height_plates, "{part}: real measured stacking height");
    }
}
