use crate::WorldError;
use std::collections::{BTreeMap, BTreeSet};

/// A stable, dense identifier within one versioned topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileId(u32);
impl TileId {
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }
    /// Index into the world's tile slice.
    pub fn index(self) -> usize {
        self.0 as usize
    }
    /// Persistable integer representation.
    pub fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    pub center: [f64; 3],
    pub corners: Vec<[f64; 3]>,
    pub neighbors: Vec<TileId>,
}

/// The dual of a subdivided icosahedron. Identity is entirely combinatorial.
#[derive(Clone, Debug)]
pub struct Topology {
    pub(crate) tiles: Vec<Cell>,
}

impl Topology {
    /// Build a frequency 2..=12 closed spherical graph.
    pub fn generate(frequency: u8) -> Result<Self, WorldError> {
        if !(2..=12).contains(&frequency) {
            return Err(WorldError::Invalid(
                "frequency must be between 2 and 12".into(),
            ));
        }
        let phi = (1.0 + libm::sqrt(5.0)) / 2.0;
        let base = [
            [-1., phi, 0.],
            [1., phi, 0.],
            [-1., -phi, 0.],
            [1., -phi, 0.],
            [0., -1., phi],
            [0., 1., phi],
            [0., -1., -phi],
            [0., 1., -phi],
            [phi, 0., -1.],
            [phi, 0., 1.],
            [-phi, 0., -1.],
            [-phi, 0., 1.],
        ]
        .map(normalize);
        const FACES: [[usize; 3]; 20] = [
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        let n = frequency;
        let mut identities = BTreeMap::<[u8; 12], usize>::new();
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        for face in FACES {
            let mut grid = BTreeMap::new();
            for i in 0..=n {
                for j in 0..=n - i {
                    let weights = [n - i - j, i, j];
                    let mut key = [0; 12];
                    let mut p = [0.; 3];
                    for k in 0..3 {
                        key[face[k]] = weights[k];
                        for (axis, value) in p.iter_mut().enumerate() {
                            // The next step normalizes p; a common 1/n scale cancels.
                            *value += base[face[k]][axis] * f64::from(weights[k]);
                        }
                    }
                    let id = *identities.entry(key).or_insert_with(|| {
                        vertices.push(normalize(p));
                        vertices.len() - 1
                    });
                    grid.insert((i, j), id);
                }
            }
            for i in 0..n {
                for j in 0..n - i {
                    faces.push([grid[&(i, j)], grid[&(i + 1, j)], grid[&(i, j + 1)]]);
                    if i + j + 1 < n {
                        faces.push([grid[&(i + 1, j)], grid[&(i + 1, j + 1)], grid[&(i, j + 1)]]);
                    }
                }
            }
        }
        let mut corners = vec![Vec::new(); vertices.len()];
        let mut neighbors = vec![BTreeSet::new(); vertices.len()];
        for [a, b, c] in faces {
            let center = normalize([
                vertices[a][0] + vertices[b][0] + vertices[c][0],
                vertices[a][1] + vertices[b][1] + vertices[c][1],
                vertices[a][2] + vertices[b][2] + vertices[c][2],
            ]);
            for (v, x, y) in [(a, b, c), (b, c, a), (c, a, b)] {
                corners[v].push(center);
                neighbors[v].insert(TileId::new(x as u32));
                neighbors[v].insert(TileId::new(y as u32));
            }
        }
        let tiles = vertices
            .into_iter()
            .enumerate()
            .map(|(i, center)| {
                let anchor = if center[1].abs() < 0.9 {
                    [0., 1., 0.]
                } else {
                    [1., 0., 0.]
                };
                let right = normalize(cross(anchor, center));
                let up = cross(center, right);
                corners[i].sort_by(|a, b| {
                    libm::atan2(dot(*a, up), dot(*a, right))
                        .total_cmp(&libm::atan2(dot(*b, up), dot(*b, right)))
                });
                Cell {
                    center,
                    corners: std::mem::take(&mut corners[i]),
                    neighbors: neighbors[i].iter().copied().collect(),
                }
            })
            .collect();
        Ok(Self { tiles })
    }
    /// Number of cells in the dual graph.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normalize(p: [f64; 3]) -> [f64; 3] {
    let n = libm::sqrt(dot(p, p));
    p.map(|x| x / n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_product_is_perpendicular_and_changes_sign_when_reversed() {
        let a = [1., 2., 3.];
        let b = [4., 5., 6.];
        let normal = cross(a, b);
        assert_eq!(normal, [-3., 6., -3.]);
        assert_eq!(dot(normal, a), 0.);
        assert_eq!(dot(normal, b), 0.);
        assert_eq!(cross(b, a), normal.map(|value| -value));
    }
}
