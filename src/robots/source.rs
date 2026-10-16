pub const INTERIORS: [[f32; 3]; 3] = [
    [-169.474388, 4.572000, 163.688268],
    [-155.859988, 4.572000, 163.891468],
    [-142.651988, 4.445000, 163.866068],
];
pub const TUBES: [&[[f32; 3]]; 3] = [
    &[
        [-173.863000, 3.860800, 162.001200],
        [-172.389800, 3.860800, 161.493200],
        [-170.865800, 3.860800, 161.493200],
        [-169.456100, 3.860800, 162.001200],
    ],
    &[
        [-160.274000, 3.860800, 162.001200],
        [-158.788100, 3.860800, 161.493200],
        [-157.264100, 3.860800, 161.480500],
        [-155.841700, 3.860800, 161.988500],
    ],
    &[
        [-147.053300, 3.733800, 161.988500],
        [-145.580100, 3.733800, 161.480500],
        [-144.056100, 3.733800, 161.493200],
        [-142.633700, 3.733800, 162.001200],
    ],
];

pub const LANDINGS: &[[f32; 3]] = &[
    [-41.452800, 2.721610, -16.967200],
    [-64.719200, 5.123688, 10.160000],
    [-22.352000, 2.720340, 9.448800],
    [13.106400, 2.323084, -20.421600],
    [32.105600, 2.519680, 5.994400],
    [47.777400, 3.149600, -20.320000],
    [69.799200, 4.100576, 5.892800],
    [21.539200, 6.964680, 79.451200],
    [9.956800, 6.297930, 50.190400],
    [13.208000, -3.987800, -79.248000],
    [62.179200, -3.644900, -79.654400],
];

pub const TANK_LANDINGS: &[[f32; 3]] = &[
    [1.016000, -5.255006, -101.803200],
    [1.016000, -5.282438, -103.632000],
    [1.016000, -5.281168, -105.257600],
    [0.609600, -5.153406, -107.086400],
    [-3.048000, -5.209286, -107.289600],
    [-6.908800, -5.288788, -106.680000],
    [-8.331200, -5.284978, -104.851200],
    [-9.144000, -5.324094, -102.209600],
];

pub const ALIGNMENT: &[[f32; 3]] = &[
    [0.000000, 17.272000, -0.000000],
    [0.000000, 17.272000, -95.504000],
    [0.000000, 17.272000, 44.297600],
];

use bevy::prelude::*;

pub(super) struct Footprint {
    pub point: Vec3,
    pub lights: Vec<Transform>,
}

/// The client script follows the map's target chains; lights within 800 source
/// units of the next footprint stay on until one second after that foot lands.
pub(super) fn warning_paths(bytes: &[u8]) -> [[Vec<Footprint>; 2]; 3] {
    #[derive(serde::Deserialize)]
    struct Map {
        entities: Vec<Record>,
    }
    #[derive(serde::Deserialize)]
    struct Record {
        #[serde(default)]
        targetname: String,
        #[serde(default)]
        target: String,
        #[serde(default)]
        origin: String,
        #[serde(default)]
        angles: String,
    }
    fn vector(text: &str) -> Vec3 {
        let values: Vec<f32> = text
            .split_whitespace()
            .map(|v| v.parse().expect("Robot marker vector"))
            .collect();
        assert_eq!(values.len(), 3, "Required robot marker vector");
        let v = Vec3::new(values[0], values[2], -values[1]) * 0.0254;
        assert!(v.is_finite());
        v
    }
    let map: Map = serde_json::from_slice(bytes).expect("Original robot map entities");
    std::array::from_fn(|robot| {
        let prefix = ["nml", "trench", "church"][robot];
        let light_name = format!("{prefix}_foot_warn_light");
        let lights: Vec<_> = map
            .entities
            .iter()
            .filter(|e| e.targetname == light_name)
            .map(|e| {
                let mut pose = Transform::from_translation(vector(&e.origin));
                if !e.angles.is_empty() {
                    let angles: Vec<f32> = e
                        .angles
                        .split_whitespace()
                        .map(|v| v.parse::<f32>().unwrap().to_radians())
                        .collect();
                    assert_eq!(angles.len(), 3);
                    pose.rotation = Quat::from_rotation_y(-angles[1])
                        * Quat::from_rotation_z(-angles[0])
                        * Quat::from_rotation_x(angles[2]);
                }
                pose
            })
            .collect();
        assert!(!lights.is_empty(), "Required robot {robot} warning lights");
        std::array::from_fn(|side| {
            let mut name = format!("{prefix}_warn_light_fp_ref_{}", ["left", "right"][side]);
            let mut visited = std::collections::BTreeSet::new();
            let mut path = Vec::new();
            while !name.is_empty() {
                assert!(
                    visited.insert(name.clone()),
                    "Robot footprint target cycle: {name}"
                );
                let marker = map
                    .entities
                    .iter()
                    .find(|e| e.targetname == name)
                    .unwrap_or_else(|| panic!("Required robot footprint marker {name}"));
                let point = vector(&marker.origin);
                path.push(Footprint {
                    point,
                    lights: lights
                        .iter()
                        .copied()
                        .filter(|light| {
                            light.translation.distance_squared(point) < (800_f32 * 0.0254).powi(2)
                        })
                        .collect(),
                });
                name.clone_from(&marker.target);
            }
            path
        })
    })
}
