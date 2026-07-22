use serde::{Deserialize, Serialize};

/// A single point: absolute position (f64, original coordinate space) plus RGB color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub position: [f64; 3],
    pub color: [u8; 3],
}

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    pub fn empty() -> Self {
        Aabb {
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        }
    }

    pub fn from_points(positions: impl IntoIterator<Item = [f64; 3]>) -> Self {
        let mut bounds = Aabb::empty();
        for p in positions {
            bounds.expand(&p);
        }
        bounds
    }

    pub fn expand(&mut self, p: &[f64; 3]) {
        for i in 0..3 {
            if p[i] < self.min[i] {
                self.min[i] = p[i];
            }
            if p[i] > self.max[i] {
                self.max[i] = p[i];
            }
        }
    }

    pub fn center(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }

    /// Widest side length, used as the node's "spacing" proxy for LOD error metrics.
    pub fn diagonal(&self) -> f64 {
        let d = [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ];
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    }

    /// Which of the 8 octants a position falls in, relative to this box's center.
    /// Bit 0 = x half, bit 1 = y half, bit 2 = z half (matches `octant_bounds`).
    pub fn octant_index(&self, p: &[f64; 3]) -> u8 {
        let c = self.center();
        let mut idx = 0u8;
        if p[0] >= c[0] {
            idx |= 1;
        }
        if p[1] >= c[1] {
            idx |= 2;
        }
        if p[2] >= c[2] {
            idx |= 4;
        }
        idx
    }

    /// Bounding box of a given octant (0..8), splitting this box at its center.
    pub fn octant_bounds(&self, octant: u8) -> Aabb {
        let c = self.center();
        let mut min = self.min;
        let mut max = self.max;
        for axis in 0..3 {
            let bit = 1 << axis;
            if octant & bit != 0 {
                min[axis] = c[axis];
            } else {
                max[axis] = c[axis];
            }
        }
        Aabb { min, max }
    }
}

/// Node identifiers are Potree-style octal path strings: "r" is the root,
/// "r0".."r7" are its children, etc. The path IS the tree structure.
pub const ROOT_ID: &str = "r";

pub fn child_id(parent: &str, octant: u8) -> String {
    debug_assert!(octant < 8);
    format!("{parent}{octant}")
}

pub fn node_depth(id: &str) -> usize {
    id.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octant_roundtrip_covers_all_points() {
        let bounds = Aabb {
            min: [0.0, 0.0, 0.0],
            max: [2.0, 2.0, 2.0],
        };
        let p = [1.5, 0.5, 1.9];
        let idx = bounds.octant_index(&p);
        let child = bounds.octant_bounds(idx);
        assert!(p[0] >= child.min[0] && p[0] <= child.max[0]);
        assert!(p[1] >= child.min[1] && p[1] <= child.max[1]);
        assert!(p[2] >= child.min[2] && p[2] <= child.max[2]);
    }

    #[test]
    fn child_id_format() {
        assert_eq!(child_id(ROOT_ID, 3), "r3");
        assert_eq!(child_id("r3", 0), "r30");
        assert_eq!(node_depth("r30"), 2);
        assert_eq!(node_depth(ROOT_ID), 0);
    }
}
