//! Original CPU-generated game geometry and pixel textures. No GPU ownership.
use claimlands_world::{Terrain, World};

/// Interleaved GPU-independent vertex representation.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// Position in unit-planet coordinates.
    pub position: [f32; 3],
    /// Unit surface normal.
    pub normal: [f32; 3],
    /// Per-tile procedural atlas coordinate.
    pub uv: [f32; 2],
}
/// Complete CPU assets for one immutable world revision.
#[derive(Debug)]
pub struct PlanetAssets {
    /// Triangle vertices.
    pub vertices: Vec<Vertex>,
    /// Indices into vertices.
    pub indices: Vec<u32>,
    /// RGBA8 unpremultiplied texture pixels.
    pub pixels: Vec<u8>,
    /// Square atlas width/height.
    pub texture_size: u32,
}

impl PlanetAssets {
    /// Rebuild bounded assets for a resolved world. Textures are always procedural.
    ///
    /// ```
    /// use claimlands_visuals::PlanetAssets;
    /// use claimlands_world::{LevelDefinition, World};
    /// let world = World::generate(&LevelDefinition::new(2, 0))?;
    /// let assets = PlanetAssets::generate(&world);
    /// assert_eq!(assets.indices.len() % 3, 0);
    /// assert!(!assets.pixels.is_empty());
    /// # Ok::<(), claimlands_world::WorldError>(())
    /// ```
    pub fn generate(world: &World) -> Self {
        const CELL: u32 = 24;
        const COLUMNS: u32 = 40;
        let size = CELL * COLUMNS;
        let mut assets = Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            pixels: vec![0; (size * size * 4) as usize],
            texture_size: size,
        };
        for tile in world.tiles() {
            let id = tile.id().value();
            let x0 = (id % COLUMNS) * CELL;
            let y0 = (id / COLUMNS) * CELL;
            let center = tile.center().map(|v| v as f32);
            let land = tile.terrain() == Terrain::Grass;
            let radius = if land { 1.012 } else { 1.0 };
            let base = assets.vertices.len() as u32;
            let atlas_uv =
                |x: f32, y: f32| [(x0 as f32 + x) / size as f32, (y0 as f32 + y) / size as f32];
            assets.vertices.push(Vertex {
                position: center.map(|x| x * radius),
                normal: center,
                uv: atlas_uv(12., 12.),
            });
            for (i, corner) in tile.corners().iter().enumerate() {
                let angle = i as f32 * std::f32::consts::TAU / tile.corners().len() as f32;
                assets.vertices.push(Vertex {
                    position: corner.map(|x| x as f32 * radius),
                    normal: center,
                    uv: atlas_uv(12. + 10.5 * angle.cos(), 12. + 10.5 * angle.sin()),
                });
            }
            let sides = tile.corners().len() as u32;
            for i in 0..sides {
                assets
                    .indices
                    .extend([base, base + 1 + i, base + 1 + (i + 1) % sides]);
                // Side walls connect lifted land to sea without opening cracks.
                if land {
                    let a = tile.corners()[i as usize].map(|x| x as f32);
                    let b = tile.corners()[((i + 1) % sides) as usize].map(|x| x as f32);
                    let start = assets.vertices.len() as u32;
                    for position in [a.map(|x| x * radius), a, b, b.map(|x| x * radius)] {
                        assets.vertices.push(Vertex {
                            position,
                            normal: center,
                            uv: atlas_uv(1., 1.),
                        });
                    }
                    assets.indices.extend([
                        start,
                        start + 1,
                        start + 2,
                        start,
                        start + 2,
                        start + 3,
                    ]);
                }
            }
            for y in 0..CELL {
                for x in 0..CELL {
                    let mut h = id
                        .wrapping_mul(747796405)
                        .wrapping_add(x * 289 + y * 977)
                        .wrapping_add(world.seed() as u32);
                    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
                    h ^= h >> 13;
                    let shade = (h % 3) as usize;
                    let palette = if land {
                        [[63, 125, 52], [90, 164, 68], [124, 194, 85]]
                    } else {
                        [[31, 86, 125], [39, 108, 148], [52, 129, 165]]
                    };
                    let offset = (((y0 + y) * size + x0 + x) * 4) as usize;
                    assets.pixels[offset..offset + 3].copy_from_slice(&palette[shade]);
                    assets.pixels[offset + 3] = 255;
                }
            }
        }
        assets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claimlands_world::LevelDefinition;
    #[test]
    fn assets_are_bounded_valid_and_repeatable() {
        for (frequency, seed) in [(2, 0), (8, 63352), (12, 10)] {
            let world = World::generate(&LevelDefinition::new(frequency, seed)).unwrap();
            let assets = PlanetAssets::generate(&world);
            assert_eq!(
                assets.pixels.len(),
                (assets.texture_size * assets.texture_size * 4) as usize
            );
            assert!(
                assets
                    .indices
                    .iter()
                    .all(|&i| (i as usize) < assets.vertices.len())
            );
            assert!(assets.vertices.iter().all(|v| {
                v.position
                    .iter()
                    .chain(v.normal.iter())
                    .chain(v.uv.iter())
                    .all(|x| x.is_finite())
            }));
            assert!(
                assets
                    .vertices
                    .iter()
                    .all(|v| v.uv.iter().all(|x| (0.0..=1.0).contains(x)))
            );
            assert!(assets.indices.len() >= world.tiles().len() * 15);
            assert_eq!(assets.pixels, PlanetAssets::generate(&world).pixels);
            assert!(assets.pixels.chunks_exact(4).any(|p| p[3] == 255));
        }
    }
    #[test]
    fn changing_seed_changes_procedural_pixels() {
        let a = PlanetAssets::generate(&World::generate(&LevelDefinition::new(2, 7)).unwrap());
        let b = PlanetAssets::generate(&World::generate(&LevelDefinition::new(2, 8)).unwrap());
        assert_ne!(a.pixels, b.pixels);
    }
}
