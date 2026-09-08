//! Renderer-independent planet topology and validated, versioned RON levels.
mod level;
mod topology;

pub use level::{
    AiConfig, Faction, FactionConfig, LevelDefinition, TileContent, TileOverride, Victory,
};
pub use topology::{TileId, Topology};

/// Errors reported before a level can become an authoritative world.
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    /// The supplied RON is malformed.
    #[error("invalid RON: {0}")]
    Parse(String),
    /// The level violates its versioned contract.
    #[error("invalid level: {0}")]
    Invalid(String),
}

/// A tile's underlying surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Terrain {
    /// Impassable sea.
    Water,
    /// Habitable land.
    Grass,
}

/// Geometry and initial state for one tile, exposed through read-only accessors.
#[derive(Clone, Debug)]
pub struct Tile {
    id: TileId,
    center: [f64; 3],
    corners: Vec<[f64; 3]>,
    neighbors: Vec<TileId>,
    terrain: Terrain,
    owner: Option<Faction>,
    content: TileContent,
}

impl Tile {
    /// Stable ID within this generator version and frequency.
    pub fn id(&self) -> TileId {
        self.id
    }
    /// Unit vector pointing to the tile's center.
    pub fn center(&self) -> [f64; 3] {
        self.center
    }
    /// Counterclockwise polygon corners on the unit sphere.
    pub fn corners(&self) -> &[[f64; 3]] {
        &self.corners
    }
    /// Adjacent tiles, sorted by stable ID.
    pub fn neighbors(&self) -> &[TileId] {
        &self.neighbors
    }
    /// Initial surface.
    pub fn terrain(&self) -> Terrain {
        self.terrain
    }
    /// Initial faction occupation.
    pub fn owner(&self) -> Option<Faction> {
        self.owner
    }
    /// Initial tile improvement or cover.
    pub fn content(&self) -> TileContent {
        self.content
    }
}

/// An immutable resolved initial board. Rendering cannot change its state.
#[derive(Clone, Debug)]
pub struct World {
    frequency: u8,
    seed: u64,
    tiles: Vec<Tile>,
}

impl World {
    /// Validate a scenario, generate its base board, then apply sparse overrides.
    ///
    /// ```
    /// use claimlands_world::{LevelDefinition, Terrain, World};
    /// let world = World::generate(&LevelDefinition::new(2, 0))?;
    /// assert_eq!(world.tiles().len(), 42);
    /// assert!(world.tiles().iter().all(|tile| tile.terrain() == Terrain::Water));
    /// # Ok::<(), claimlands_world::WorldError>(())
    /// ```
    pub fn generate(level: &LevelDefinition) -> Result<Self, WorldError> {
        level.validate()?;
        let topology = Topology::generate(level.frequency)?;
        let mut tiles: Vec<_> = topology
            .tiles
            .into_iter()
            .enumerate()
            .map(|(i, cell)| {
                let terrain = if level.seed == 0 {
                    Terrain::Water
                } else {
                    terrain_at(cell.center, level.seed)
                };
                Tile {
                    id: TileId::new(i as u32),
                    center: cell.center,
                    corners: cell.corners,
                    neighbors: cell.neighbors,
                    terrain,
                    owner: None,
                    content: TileContent::Empty,
                }
            })
            .collect();
        for (&id, change) in &level.overrides {
            let tile = &mut tiles[id as usize];
            tile.terrain = change.terrain;
            tile.owner = change.owner;
            tile.content = change.content;
        }
        Ok(Self {
            frequency: level.frequency,
            seed: level.seed,
            tiles,
        })
    }
    /// All tiles in stable ID order.
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    /// The subdivision frequency used for topology.
    pub fn frequency(&self) -> u8 {
        self.frequency
    }
    /// The input terrain seed, with zero denoting a water base.
    pub fn seed(&self) -> u64 {
        self.seed
    }
    /// Canonical discrete-board fingerprint, independent of RON formatting and GPU math.
    pub fn fingerprint(&self) -> String {
        let mut hash = blake3::Hasher::new();
        hash.update(b"claimlands-world-v1");
        hash.update(&[self.frequency]);
        hash.update(&self.seed.to_le_bytes());
        for tile in &self.tiles {
            hash.update(&tile.id.value().to_le_bytes());
            hash.update(&[
                tile.terrain as u8,
                tile.owner.map_or(255, |x| x as u8),
                tile.content as u8,
            ]);
            hash.update(&[tile.neighbors.len() as u8]);
            for neighbor in &tile.neighbors {
                hash.update(&neighbor.value().to_le_bytes());
            }
        }
        hash.finalize().to_hex().to_string()
    }
}

// Fixed integer PRNG and libm trigonometry make classification independent of
// the browser's math library. Appearance generators never advance this stream.
fn terrain_at(p: [f64; 3], seed: u64) -> Terrain {
    let mut rng = seed;
    let mut sum = 0.0;
    for octave in 0..5 {
        let mut axis = [0.0; 3];
        for value in &mut axis {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *value = ((rng >> 32) as u32 as f64 / u32::MAX as f64) * 2.0 - 1.0;
        }
        let f = 3.0 + f64::from(octave) * 1.8;
        sum += libm::sin((p[0] * axis[0] + p[1] * axis[1] + p[2] * axis[2]) * f + axis[0] * 8.0)
            / (1.0 + f64::from(octave));
    }
    classify_height(sum)
}

// The sea level belongs to water, so a continent begins strictly above it.
fn classify_height(height: f64) -> Terrain {
    if height > -0.1 {
        Terrain::Grass
    } else {
        Terrain::Water
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sea_level_is_water_and_the_first_height_above_it_is_land() {
        let sea_level = -0.1_f64;
        assert_eq!(classify_height(sea_level.next_down()), Terrain::Water);
        assert_eq!(classify_height(sea_level), Terrain::Water);
        assert_eq!(classify_height(sea_level.next_up()), Terrain::Grass);
    }
}
