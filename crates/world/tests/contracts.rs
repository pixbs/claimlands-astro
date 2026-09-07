//! The same stable-world contracts run on native hosts and in browsers.
use claimlands_world::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;
#[cfg(target_arch = "wasm32")]
wasm_bindgen_test_configure!(run_in_browser);

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn generator_v1_matches_versioned_compatibility_fixtures() {
    #[derive(serde::Deserialize)]
    struct Fixture {
        frequency: u8,
        seed: u64,
        grass_tiles: usize,
        first_neighbors: Vec<u32>,
        fingerprint: String,
    }
    let fixtures: Vec<Fixture> =
        ron::from_str(include_str!("../../../tests/fixtures/world-v1.ron")).unwrap();
    assert_eq!(fixtures.len(), 3, "fixture set must not silently disappear");
    for fixture in fixtures {
        let world =
            World::generate(&LevelDefinition::new(fixture.frequency, fixture.seed)).unwrap();
        assert_eq!(world.fingerprint(), fixture.fingerprint);
        assert_eq!(
            world
                .tiles()
                .iter()
                .filter(|tile| tile.terrain() == Terrain::Grass)
                .count(),
            fixture.grass_tiles,
        );
        assert_eq!(
            world.tiles()[0]
                .neighbors()
                .iter()
                .map(|id| id.value())
                .collect::<Vec<_>>(),
            fixture.first_neighbors,
        );
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn closed_sphere_contract() {
    for n in 2..=12 {
        let world = World::generate(&LevelDefinition::new(n, 0)).unwrap();
        assert_eq!(world.tiles().len(), 10 * usize::from(n).pow(2) + 2);
        assert_eq!(
            Topology::generate(n).unwrap().tile_count(),
            world.tiles().len()
        );
        assert_eq!(
            world
                .tiles()
                .iter()
                .filter(|t| t.neighbors().len() == 5)
                .count(),
            12
        );
        let mut visited = std::collections::BTreeSet::new();
        let mut queue = vec![world.tiles()[0].id()];
        while let Some(id) = queue.pop() {
            if visited.insert(id) {
                queue.extend(world.tiles()[id.index()].neighbors());
            }
        }
        assert_eq!(visited.len(), world.tiles().len());
        for (i, tile) in world.tiles().iter().enumerate() {
            assert_eq!(tile.id().index(), i);
            assert!(matches!(tile.neighbors().len(), 5 | 6));
            assert_eq!(tile.corners().len(), tile.neighbors().len());
            assert_eq!(tile.terrain(), Terrain::Water);
            assert_eq!(tile.owner(), None);
            assert_eq!(tile.content(), TileContent::Empty);
            assert!(tile.neighbors().windows(2).all(|w| w[0] < w[1]));
            assert!(!tile.neighbors().contains(&tile.id()));
            assert!((tile.center().iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-12);
            for (index, corner) in tile.corners().iter().enumerate() {
                assert!((corner.iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-12);
                let next = tile.corners()[(index + 1) % tile.corners().len()];
                let cross = [
                    corner[1] * next[2] - corner[2] * next[1],
                    corner[2] * next[0] - corner[0] * next[2],
                    corner[0] * next[1] - corner[1] * next[0],
                ];
                assert!(
                    cross
                        .iter()
                        .zip(tile.center())
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                        > 0.0,
                    "polygon winding must face outward for rendering and picking",
                );
            }
            for neighbor in tile.neighbors() {
                assert!(
                    world.tiles()[neighbor.index()]
                        .neighbors()
                        .contains(&tile.id())
                );
                assert_eq!(
                    tile.corners()
                        .iter()
                        .filter(|corner| world.tiles()[neighbor.index()].corners().contains(corner))
                        .count(),
                    2,
                    "neighboring polygons must share exactly one edge",
                );
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn scenarios_are_repeatable_and_format_independent() {
    let source = include_str!("../../../levels/foundation.ron");
    let level = LevelDefinition::from_ron(source).unwrap();
    assert_eq!(
        LevelDefinition::from_ron(&level.to_ron().unwrap()).unwrap(),
        level
    );
    let a = World::generate(&level).unwrap();
    let b = World::generate(&LevelDefinition::from_ron(&format!("// ignored\n{source}")).unwrap())
        .unwrap();
    assert_eq!(a.fingerprint(), b.fingerprint());
    assert_eq!(a.seed(), 63352);
    assert_eq!(a.frequency(), 8);
    assert!(a.tiles().iter().any(|t| t.terrain() == Terrain::Water));
    assert!(a.tiles().iter().any(|t| t.terrain() == Terrain::Grass));
    assert_ne!(
        a.fingerprint(),
        World::generate(&LevelDefinition::new(8, 63353))
            .unwrap()
            .fingerprint()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn validated_editor_overrides_apply_to_water_base() {
    let mut level = LevelDefinition::new(2, 0);
    level.overrides.insert(
        0,
        TileOverride {
            terrain: Terrain::Grass,
            owner: Some(Faction::Red),
            content: TileContent::Capital,
        },
    );
    level.overrides.insert(
        1,
        TileOverride {
            terrain: Terrain::Grass,
            owner: None,
            content: TileContent::Forest,
        },
    );
    let world = World::generate(&level).unwrap();
    assert_eq!(world.tiles()[0].owner(), Some(Faction::Red));
    assert_eq!(world.tiles()[0].content(), TileContent::Capital);
    assert_eq!(world.tiles()[1].content(), TileContent::Forest);
    assert_eq!(
        world
            .tiles()
            .iter()
            .filter(|t| t.terrain() == Terrain::Grass)
            .count(),
        2
    );
    assert_eq!(
        LevelDefinition::from_ron(&level.to_ron().unwrap()).unwrap(),
        level
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn invalid_inputs_fail_without_constructing_a_world() {
    for n in [0, 1, 13, 255] {
        assert!(Topology::generate(n).is_err());
    }
    let valid = LevelDefinition::new(2, 1);
    let reject = |level: &LevelDefinition| {
        assert!(level.validate().is_err());
        assert!(level.to_ron().is_err());
        assert!(World::generate(level).is_err());
        assert!(LevelDefinition::from_ron(&ron::to_string(level).unwrap()).is_err());
    };
    let mut x = valid.clone();
    x.schema_version = 2;
    reject(&x);
    let mut x = valid.clone();
    x.generator_version = 2;
    reject(&x);
    let mut x = valid.clone();
    x.frequency = 1;
    reject(&x);
    let mut x = valid.clone();
    x.factions.clear();
    reject(&x);
    let mut x = valid.clone();
    x.factions = vec![x.factions[0].clone(); 5];
    reject(&x);
    let mut x = valid.clone();
    x.factions[1].faction = Faction::Red;
    reject(&x);
    for a in [
        AiConfig {
            hostility: 101,
            intelligence: 50,
        },
        AiConfig {
            hostility: 50,
            intelligence: 101,
        },
    ] {
        let mut x = valid.clone();
        x.factions[1].ai = Some(a);
        reject(&x);
    }
    for (id, tile) in [
        (
            42,
            TileOverride {
                terrain: Terrain::Grass,
                owner: None,
                content: TileContent::Empty,
            },
        ),
        (
            0,
            TileOverride {
                terrain: Terrain::Water,
                owner: Some(Faction::Red),
                content: TileContent::Empty,
            },
        ),
        (
            0,
            TileOverride {
                terrain: Terrain::Water,
                owner: None,
                content: TileContent::Field,
            },
        ),
        (
            0,
            TileOverride {
                terrain: Terrain::Grass,
                owner: Some(Faction::Green),
                content: TileContent::Town,
            },
        ),
        (
            0,
            TileOverride {
                terrain: Terrain::Grass,
                owner: None,
                content: TileContent::Capital,
            },
        ),
    ] {
        let mut x = valid.clone();
        x.overrides.insert(id, tile);
        reject(&x);
    }
    for text in ["broken", "()", "(unknown:1)", &" ".repeat(1_048_577)] {
        assert!(LevelDefinition::from_ron(text).is_err());
    }
    let unknown = valid.to_ron().unwrap().replacen('(', "(typo:1,", 1);
    assert!(LevelDefinition::from_ron(&unknown).is_err());
    assert!(!format!("{}", WorldError::Parse("test".into())).is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn four_factions_and_boundary_ai_values_are_valid() {
    let mut level = LevelDefinition::new(12, u64::MAX);
    level.factions.push(FactionConfig {
        faction: Faction::Green,
        ai: Some(AiConfig {
            hostility: 0,
            intelligence: 100,
        }),
    });
    level.factions.push(FactionConfig {
        faction: Faction::Yellow,
        ai: Some(AiConfig {
            hostility: 100,
            intelligence: 0,
        }),
    });
    level.validate().unwrap();
    assert_eq!(
        LevelDefinition::from_ron(&level.to_ron().unwrap()).unwrap(),
        level
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn parser_rejects_ambiguous_and_misspelled_editor_data() {
    let mut level = LevelDefinition::new(2, 0);
    level.overrides.insert(
        0,
        TileOverride {
            terrain: Terrain::Grass,
            owner: None,
            content: TileContent::Empty,
        },
    );
    let source = level.to_ron().unwrap();
    let edit = "0:(terrain:Grass,owner:None,content:Empty)";
    let duplicate = source.replace(edit, &format!("{edit},{edit}"));
    assert_ne!(duplicate, source);
    let error = LevelDefinition::from_ron(&duplicate).unwrap_err();
    assert!(error.to_string().contains("duplicate override tile ID"));
    for malformed in [
        source.replace("terrain:Grass", "terrain:Grass,typo:1"),
        source.replace("faction:Red", "faction:Red,typo:1"),
        source.replace("hostility:50", "hostility:50,typo:1"),
        source.replace("overrides:{", "overrides:["),
        format!("{source} ()"),
    ] {
        assert!(LevelDefinition::from_ron(&malformed).is_err());
    }
    let boundary = format!("{}{}", " ".repeat(1_048_576 - source.len()), source);
    assert_eq!(LevelDefinition::from_ron(&boundary).unwrap(), level);
    assert!(LevelDefinition::from_ron(&format!(" {boundary}")).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sparse_overrides_preserve_other_tiles_and_canonical_order() {
    let mut level = LevelDefinition::new(2, 0);
    let mut previous = World::generate(&level).unwrap().fingerprint();
    for content in [
        TileContent::Empty,
        TileContent::Field,
        TileContent::Town,
        TileContent::Capital,
    ] {
        level.overrides.insert(
            41,
            TileOverride {
                terrain: Terrain::Grass,
                owner: Some(Faction::Red),
                content,
            },
        );
        let world = World::generate(&level).unwrap();
        assert_eq!(world.tiles()[41].id().value(), 41);
        assert_eq!(world.tiles()[41].owner(), Some(Faction::Red));
        assert_eq!(world.tiles()[41].content(), content);
        assert!(
            world.tiles()[..41]
                .iter()
                .all(|tile| tile.terrain() == Terrain::Water)
        );
        assert_ne!(world.fingerprint(), previous);
        previous = world.fingerprint();
    }
    level.overrides.insert(
        0,
        TileOverride {
            terrain: Terrain::Grass,
            owner: Some(Faction::Blue),
            content: TileContent::Forest,
        },
    );
    let mut reversed = level.clone();
    reversed.overrides = level
        .overrides
        .iter()
        .rev()
        .map(|(id, tile)| (*id, tile.clone()))
        .collect();
    assert_eq!(level.to_ron().unwrap(), reversed.to_ron().unwrap());
    assert!(!level.to_ron().unwrap().contains('\n'));
    assert_eq!(
        World::generate(&level).unwrap().fingerprint(),
        World::generate(&reversed).unwrap().fingerprint()
    );
}

#[cfg(not(target_arch = "wasm32"))]
mod properties {
    use super::*;
    use proptest::prelude::*;
    proptest! {
        #![proptest_config(ProptestConfig { cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(32), ..ProptestConfig::default() })]
        #[test]
        fn seeded_worlds_survive_round_trip(n in 2u8..=12,seed in any::<u64>()) {
            let level=LevelDefinition::new(n,seed);
            let a=World::generate(&level).unwrap();
            let b=World::generate(&LevelDefinition::from_ron(&level.to_ron().unwrap()).unwrap()).unwrap();
            prop_assert_eq!(a.fingerprint(),b.fingerprint());
        }
    }
}
