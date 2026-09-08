//! Public contracts for CPU assets; GPU appearance has separate visual baselines.
use claimlands_visuals::PlanetAssets;
use claimlands_world::{LevelDefinition, Terrain, TileContent, TileOverride, World};

#[test]
fn sea_mesh_is_a_closed_outward_facing_sphere() {
    for frequency in [2, 12] {
        let world = World::generate(&LevelDefinition::new(frequency, 0)).unwrap();
        let assets = PlanetAssets::generate(&world);
        let sides: usize = world.tiles().iter().map(|tile| tile.corners().len()).sum();
        assert_eq!(assets.vertices.len(), world.tiles().len() + sides);
        assert_eq!(assets.indices.len(), 3 * sides);
        for vertex in &assets.vertices {
            assert!((length_squared(vertex.position) - 1.0).abs() < 1e-6);
            assert!((length_squared(vertex.normal) - 1.0).abs() < 1e-6);
        }
        for triangle in assets.indices.chunks_exact(3) {
            let [a, b, c]: [[f32; 3]; 3] =
                std::array::from_fn(|i| assets.vertices[triangle[i] as usize].position);
            let ab = std::array::from_fn(|i| b[i] - a[i]);
            let ac = std::array::from_fn(|i| c[i] - a[i]);
            assert!(dot(cross(ab, ac), a) > 0.0);
        }
    }
}

#[test]
fn land_override_adds_lifted_surface_and_exactly_two_wall_triangles_per_edge() {
    let mut level = LevelDefinition::new(2, 0);
    let water_world = World::generate(&level).unwrap();
    let water = PlanetAssets::generate(&water_world);
    // Include both exceptional pentagons and ordinary hexagons.
    for sides in [5, 6] {
        let tile = water_world
            .tiles()
            .iter()
            .find(|tile| tile.corners().len() == sides)
            .unwrap();
        level.overrides.clear();
        level.overrides.insert(
            tile.id().value(),
            TileOverride {
                terrain: Terrain::Grass,
                owner: None,
                content: TileContent::Empty,
            },
        );
        let land = PlanetAssets::generate(&World::generate(&level).unwrap());
        assert_eq!(land.vertices.len(), water.vertices.len() + 4 * sides);
        assert_eq!(land.indices.len(), water.indices.len() + 6 * sides);
        let lifted = land
            .vertices
            .iter()
            .filter(|vertex| length_squared(vertex.position) > 1.01)
            .count();
        assert_eq!(lifted, 1 + sides + 2 * sides);
        assert!(
            land.vertices
                .iter()
                .all(|vertex| (length_squared(vertex.normal) - 1.0).abs() < 1e-6)
        );
    }
}

#[test]
fn atlas_contains_only_terrain_palettes_and_opaque_used_cells() {
    let mut level = LevelDefinition::new(2, 0);
    level.overrides.insert(
        0,
        TileOverride {
            terrain: Terrain::Grass,
            owner: None,
            content: TileContent::Empty,
        },
    );
    let world = World::generate(&level).unwrap();
    let assets = PlanetAssets::generate(&world);
    assert_eq!(assets.texture_size, 960);
    let grass = [[63, 125, 52, 255], [90, 164, 68, 255], [124, 194, 85, 255]];
    let water = [[31, 86, 125, 255], [39, 108, 148, 255], [52, 129, 165, 255]];
    for tile in world.tiles() {
        let x0 = tile.id().index() % 40 * 24;
        let y0 = tile.id().index() / 40 * 24;
        let palette = if tile.terrain() == Terrain::Grass {
            &grass
        } else {
            &water
        };
        let mut shades = std::collections::BTreeSet::new();
        for y in y0..y0 + 24 {
            for x in x0..x0 + 24 {
                let offset = (y * assets.texture_size as usize + x) * 4;
                let pixel: [u8; 4] = assets.pixels[offset..offset + 4].try_into().unwrap();
                assert!(palette.contains(&pixel));
                shades.insert(pixel);
            }
        }
        assert_eq!(
            shades.len(),
            3,
            "each tile must retain procedural texture variation"
        );
    }
    assert_eq!(
        assets
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] == 255)
            .count(),
        world.tiles().len() * 24 * 24
    );
}

#[test]
fn atlas_coordinates_form_an_outward_regular_polygon_inside_each_cell() {
    let world = World::generate(&LevelDefinition::new(2, 7)).unwrap();
    let assets = PlanetAssets::generate(&world);
    for tile in world.tiles() {
        let normal = tile.center().map(|value| value as f32);
        let vertices: Vec<_> = assets
            .vertices
            .iter()
            .filter(|vertex| vertex.normal == normal)
            .collect();
        let origin = [
            (tile.id().index() % 40 * 24) as f32,
            (tile.id().index() / 40 * 24) as f32,
        ];
        let local = |uv: [f32; 2]| [uv[0] * 960. - origin[0], uv[1] * 960. - origin[1]];
        assert_near_2(local(vertices[0].uv), [12., 12.]);
        let ring: Vec<_> = vertices[1..=tile.corners().len()]
            .iter()
            .map(|vertex| {
                let uv = local(vertex.uv);
                [uv[0] - 12., uv[1] - 12.]
            })
            .collect();
        assert_near_2(ring[0], [10.5, 0.]);
        let step_cosine = if ring.len() == 5 { 0.309_017_f32 } else { 0.5 };
        for (i, a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            assert!((a[0] * a[0] + a[1] * a[1] - 110.25).abs() < 0.002);
            assert!((a[0] * b[0] + a[1] * b[1] - 110.25 * step_cosine).abs() < 0.002);
            assert!(
                a[0] * b[1] - a[1] * b[0] > 0.,
                "UV winding must agree with geometry"
            );
        }
        for vertex in &vertices[tile.corners().len() + 1..] {
            assert_near_2(local(vertex.uv), [1., 1.]);
        }
    }
}

#[test]
fn each_coastal_wall_is_a_complete_nondegenerate_quad_between_surface_and_sea() {
    let mut level = LevelDefinition::new(2, 0);
    let ocean = World::generate(&level).unwrap();
    for sides in [5, 6] {
        let tile = ocean
            .tiles()
            .iter()
            .find(|tile| tile.corners().len() == sides)
            .unwrap();
        level.overrides.clear();
        level.overrides.insert(
            tile.id().value(),
            TileOverride {
                terrain: Terrain::Grass,
                owner: None,
                content: TileContent::Empty,
            },
        );
        let assets = PlanetAssets::generate(&World::generate(&level).unwrap());
        let normal = tile.center().map(|value| value as f32);
        let triangles: Vec<[[f32; 3]; 3]> = assets
            .indices
            .chunks_exact(3)
            .filter(|indices| assets.vertices[indices[0] as usize].normal == normal)
            .map(|indices| std::array::from_fn(|i| assets.vertices[indices[i] as usize].position))
            .collect();
        for i in 0..sides {
            let a = tile.corners()[i].map(|value| value as f32);
            let b = tile.corners()[(i + 1) % sides].map(|value| value as f32);
            let quad = [
                a,
                b,
                a.map(|value| value * 1.012),
                b.map(|value| value * 1.012),
            ];
            let walls: Vec<_> = triangles
                .iter()
                .filter(|triangle| {
                    triangle
                        .iter()
                        .all(|vertex| quad.iter().any(|corner| points_near(*vertex, *corner)))
                })
                .collect();
            assert_eq!(
                walls.len(),
                2,
                "every coast edge needs exactly two wall triangles"
            );
            let mut corners_used = [false; 4];
            for triangle in walls {
                let ab = std::array::from_fn(|axis| triangle[1][axis] - triangle[0][axis]);
                let ac = std::array::from_fn(|axis| triangle[2][axis] - triangle[0][axis]);
                assert!(
                    length_squared(cross(ab, ac)) > 1e-8,
                    "wall triangles must have area"
                );
                for vertex in triangle {
                    for (index, corner) in quad.iter().enumerate() {
                        corners_used[index] |= points_near(*vertex, *corner);
                    }
                }
            }
            assert_eq!(
                corners_used, [true; 4],
                "walls must connect all four corners"
            );
        }
    }
}

#[test]
fn procedural_pixel_samples_preserve_the_version_one_texture_pattern() {
    // Fixed samples include both axes, distinct atlas rows, both palettes, and
    // nonzero seeds. These are output fixtures, not a reimplementation of the PRNG.
    let samples = [
        (0, 0),
        (1, 0),
        (0, 1),
        (2, 3),
        (7, 11),
        (12, 12),
        (22, 23),
        (23, 23),
    ];
    let fixtures = [
        (0, 0, false, [0, 0, 1, 0, 0, 2, 2, 1]),
        (0, 40, false, [1, 2, 1, 1, 1, 1, 0, 0]),
        (7, 0, true, [2, 1, 0, 0, 2, 2, 2, 0]),
        (7, 1, true, [2, 0, 1, 1, 1, 2, 2, 2]),
        (7, 40, false, [0, 2, 0, 2, 2, 0, 1, 2]),
        (7, 41, true, [2, 0, 2, 1, 0, 0, 1, 1]),
        (63352, 1, false, [2, 2, 0, 2, 0, 2, 2, 2]),
        (63352, 41, true, [2, 0, 2, 1, 1, 0, 0, 2]),
    ];
    let grass = [[63, 125, 52, 255], [90, 164, 68, 255], [124, 194, 85, 255]];
    let water = [[31, 86, 125, 255], [39, 108, 148, 255], [52, 129, 165, 255]];
    for (seed, tile, land, expected_shades) in fixtures {
        let world = World::generate(&LevelDefinition::new(2, seed)).unwrap();
        let assets = PlanetAssets::generate(&world);
        let palette = if land { grass } else { water };
        for ((x, y), shade) in samples.into_iter().zip(expected_shades) {
            let offset = ((tile / 40 * 24 + y) * 960 + tile % 40 * 24 + x) * 4;
            assert_eq!(
                &assets.pixels[offset..offset + 4],
                &palette[shade],
                "seed {seed}, tile {tile}, pixel ({x},{y})"
            );
        }
    }
}

fn assert_near_2(actual: [f32; 2], expected: [f32; 2]) {
    assert!((actual[0] - expected[0]).abs() < 0.0001);
    assert!((actual[1] - expected[1]).abs() < 0.0001);
}

fn points_near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6)
}

fn length_squared(value: [f32; 3]) -> f32 {
    dot(value, value)
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
