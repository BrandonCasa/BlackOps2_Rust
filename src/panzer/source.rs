use bevy::prelude::*;
pub(super) struct Spawn {
    pub zone: crate::origins::ZoneId,
    pub pose: Transform,
}
pub(super) fn spawns(bytes: &[u8], game: &crate::origins::GameDefinition) -> Vec<Spawn> {
    #[derive(serde::Deserialize)]
    struct Map {
        entities: Vec<Record>,
    }
    #[derive(serde::Deserialize)]
    struct Record {
        #[serde(default)]
        script_noteworthy: String,
        #[serde(default)]
        targetname: String,
        #[serde(default)]
        origin: String,
        #[serde(default)]
        angles: String,
    }
    let map: Map = serde_json::from_slice(bytes).expect("Panzer spawn map metadata");
    let vectors = |text: &str| {
        let values: Vec<f32> = text
            .split_whitespace()
            .map(|v| v.parse().expect("Source Panzer spawn vector"))
            .collect();
        let v: [f32; 3] = values
            .try_into()
            .expect("Source Panzer spawn vector length");
        assert!(v.iter().all(|v| v.is_finite()));
        v
    };
    let spawns: Vec<_> = map
        .entities
        .iter()
        .filter(|e| e.script_noteworthy == "mechz_location")
        .map(|e| {
            let [x, y, z] = vectors(&e.origin);
            let yaw = if e.angles.is_empty() {
                0.
            } else {
                vectors(&e.angles)[1].to_radians()
            };
            let source_zone = e
                .targetname
                .trim_end_matches("_spawners")
                .trim_end_matches("_spawner");
            let zone = crate::origins::ZoneId(format!(
                "zone.{}",
                source_zone
                    .strip_prefix("zone_")
                    .expect("Panzer spawn source zone")
            ));
            assert!(
                game.content.zones.iter().any(|z| z.id == zone),
                "Panzer spawn unknown zone {}",
                zone.0
            );
            Spawn {
                zone,
                pose: Transform::from_translation(Vec3::new(x, z, -y) * 0.0254)
                    .with_rotation(Quat::from_rotation_y(yaw)),
            }
        })
        .collect();
    assert!(!spawns.is_empty(), "Required Panzer source spawn markers");
    spawns
}
