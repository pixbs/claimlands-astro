use crate::{Terrain, WorldError};
use std::collections::{BTreeMap, BTreeSet};

/// One of the four supported faction identities.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Faction {
    /// Red faction.
    Red,
    /// Yellow faction.
    Yellow,
    /// Green faction.
    Green,
    /// Blue faction.
    Blue,
}
/// Initial cover or improvement. These variants do not yet implement game rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TileContent {
    /// Unimproved surface.
    Empty,
    /// Trees.
    Forest,
    /// Wheat production.
    Field,
    /// Gold production.
    Town,
    /// Territory treasury.
    Capital,
}
/// Serializable AI difficulty controls; execution belongs to a later milestone.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiConfig {
    /// Aggressiveness, from 0 to 100.
    pub hostility: u8,
    /// Planning level, from 0 to 100.
    pub intelligence: u8,
}
/// A participating faction, controlled locally when AI is absent.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionConfig {
    /// Identity used by tile occupation.
    pub faction: Faction,
    /// Optional automated controller configuration.
    pub ai: Option<AiConfig>,
}
/// Explicit victory modes supported by this schema.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Victory {
    /// Last surviving faction wins. Other victory criteria need design decisions.
    Elimination,
}
/// Complete initial state override for a single stable tile ID.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileOverride {
    /// Underlying terrain.
    pub terrain: Terrain,
    /// Optional faction occupation.
    pub owner: Option<Faction>,
    /// Surface content.
    pub content: TileContent,
}
/// Version-one level interchange format. Rust internals must not become its schema.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelDefinition {
    /// Interchange version, currently 1.
    pub schema_version: u32,
    /// Topology/terrain generator version, currently 1.
    pub generator_version: u32,
    /// Icosahedron subdivision frequency, 2..=12.
    pub frequency: u8,
    /// Base terrain seed. Zero creates all water before editor overrides.
    pub seed: u64,
    /// Tile state edits applied after base generation, sorted by tile ID.
    #[serde(deserialize_with = "unique_overrides")]
    pub overrides: BTreeMap<u32, TileOverride>,
    /// Two to four unique participants.
    pub factions: Vec<FactionConfig>,
    /// Victory mode, retained for the later simulation.
    pub victory: Victory,
}
impl LevelDefinition {
    /// A two-faction scenario with no overrides.
    pub fn new(frequency: u8, seed: u64) -> Self {
        Self {
            schema_version: 1,
            generator_version: 1,
            frequency,
            seed,
            overrides: BTreeMap::new(),
            factions: vec![
                FactionConfig {
                    faction: Faction::Red,
                    ai: None,
                },
                FactionConfig {
                    faction: Faction::Blue,
                    ai: Some(AiConfig {
                        hostility: 50,
                        intelligence: 25,
                    }),
                },
            ],
            victory: Victory::Elimination,
        }
    }
    /// Parse bounded, non-executable RON and validate its complete contract.
    pub fn from_ron(input: &str) -> Result<Self, WorldError> {
        if input.len() > 1_048_576 {
            return Err(WorldError::Invalid("level exceeds 1 MiB".into()));
        }
        let level: Self = ron::Options::default()
            .with_recursion_limit(64)
            .from_str(input)
            .map_err(|e| WorldError::Parse(e.to_string()))?;
        level.validate()?;
        Ok(level)
    }
    /// Serialize to a compact, shareable RON string after validation.
    ///
    /// ```
    /// use claimlands_world::LevelDefinition;
    /// let level = LevelDefinition::new(4, 73);
    /// let encoded = level.to_ron()?;
    /// assert!(!encoded.contains('\n'));
    /// assert_eq!(LevelDefinition::from_ron(&encoded)?, level);
    /// # Ok::<(), claimlands_world::WorldError>(())
    /// ```
    pub fn to_ron(&self) -> Result<String, WorldError> {
        self.validate()?;
        ron::to_string(self).map_err(|e| WorldError::Parse(e.to_string()))
    }
    /// Validate versions, identifiers, ownership, and bounded difficulty settings.
    pub fn validate(&self) -> Result<(), WorldError> {
        let invalid = |s: &str| Err(WorldError::Invalid(s.into()));
        if self.schema_version != 1 || self.generator_version != 1 {
            return invalid("unsupported schema or generator version");
        }
        if !(2..=12).contains(&self.frequency) {
            return invalid("frequency must be between 2 and 12");
        }
        if !(2..=4).contains(&self.factions.len()) {
            return invalid("two to four factions are required");
        }
        let mut factions = BTreeSet::new();
        for config in &self.factions {
            if !factions.insert(config.faction) {
                return invalid("factions must be unique");
            }
            if config
                .ai
                .as_ref()
                .is_some_and(|a| a.hostility > 100 || a.intelligence > 100)
            {
                return invalid("AI properties must be 0..=100");
            }
        }
        let count = 10 * u32::from(self.frequency).pow(2) + 2;
        for (&id, tile) in &self.overrides {
            if id >= count {
                return invalid("override tile ID outside topology");
            }
            if tile.terrain == Terrain::Water
                && (tile.owner.is_some() || tile.content != TileContent::Empty)
            {
                return invalid("water cannot have ownership or cover");
            }
            if tile.owner.is_some_and(|owner| !factions.contains(&owner)) {
                return invalid("tile owner is not a participating faction");
            }
            if tile.content == TileContent::Capital && tile.owner.is_none() {
                return invalid("capital requires an owner");
            }
        }
        Ok(())
    }
}

// A duplicate tile edit is ambiguous editor input, not a last-write-wins rule.
fn unique_overrides<'de, D>(deserializer: D) -> Result<BTreeMap<u32, TileOverride>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Overrides;
    impl<'de> serde::de::Visitor<'de> for Overrides {
        type Value = BTreeMap<u32, TileOverride>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("tile overrides with unique tile IDs")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut overrides = BTreeMap::new();
            while let Some((id, value)) = map.next_entry()? {
                if overrides.insert(id, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate override tile ID"));
                }
            }
            Ok(overrides)
        }
    }
    deserializer.deserialize_map(Overrides)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_override_type_explains_the_required_map_shape() {
        let input = serde::de::value::BoolDeserializer::<serde::de::value::Error>::new(true);
        let error = unique_overrides(input).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("tile overrides with unique tile IDs")
        );
    }
}
