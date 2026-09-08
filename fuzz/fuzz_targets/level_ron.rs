#![no_main]
use claimlands_world::{LevelDefinition, World};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    if let Ok(level) = LevelDefinition::from_ron(input) {
        let world = World::generate(&level).expect("a validated level generates");
        let encoded = level.to_ron().expect("validated levels serialize");
        let restored = LevelDefinition::from_ron(&encoded).expect("our own encoding parses");
        assert_eq!(world.fingerprint(), World::generate(&restored).unwrap().fingerprint());
    }
});
