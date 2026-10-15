//! Original ImpactFx selection and networked explosion orientation.
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Row {
    nonflesh: [Option<String>; 32],
    flesh: [Option<String>; 4],
}

#[derive(Deserialize)]
pub(super) struct ImpactEffects {
    impact_rows: Vec<Row>,
    impact_directions: Vec<[f32; 3]>,
}

impl ImpactEffects {
    pub(super) fn validate(&self) {
        assert_eq!(
            self.impact_rows.len(),
            21,
            "Original ImpactFx requires all 21 rows"
        );
        assert_eq!(
            self.impact_directions.len(),
            162,
            "Original direction table requires all 162 vectors"
        );
    }
}

impl super::Catalog {
    ///00535700 maps WeaponDef.impactType to the expanded internal ImpactFx row.
    /// A null authored entry deliberately emits nothing.
    pub(crate) fn impact(&self, impact_type: u32, surface: u8, flags: u32) -> Option<&str> {
        let impact_type = if flags & 8 != 0 { 1 } else { impact_type };
        let exit = usize::from(flags & 4 != 0);
        let row = match impact_type {
            1 => {
                if exit != 0 {
                    1
                } else {
                    usize::from(flags & 8 != 0) * 2
                }
            }
            2 => 3 + exit,
            3 => 7 + exit,
            4 => 9 + exit,
            5 => 5 + exit,
            6 => 11,
            7 | 8 => 12,
            9 => 13,
            10 => 14,
            11 => 15,
            12 => 16,
            13 => 17,
            14 => 19,
            15 => 20,
            _ => return None,
        };
        let row = &self.impacts.impact_rows[row];
        let effect = if surface == 7 {
            &row.flesh[((flags & 1) * 2 | u32::from(flags & 2 != 0)) as usize]
        } else {
            row.nonflesh.get(usize::from(surface)).unwrap_or_else(|| {
                panic!("Original ImpactFx surface {surface} exceeds its 32 surface entries")
            })
        };
        effect.as_deref().map(|name| name.trim_start_matches(','))
    }

    ///0060a8f0 ->007490e0 quantizes the server normal;004c1f40 ->00749140
    /// decodes it and0074e540 constructs the perpendicular source axes.
    pub(crate) fn explosion_pose(&self, center: Vec3, normal: Vec3) -> Transform {
        let native_normal = Vec3::new(normal.x, -normal.z, normal.y);
        let mut selected = self.impacts.impact_directions[0];
        let mut best = 0.;
        for direction in &self.impacts.impact_directions {
            let dot = Vec3::from_array(*direction).dot(native_normal);
            if dot > best {
                best = dot;
                selected = *direction;
            }
        }
        let normal = Vec3::from_array(selected);
        let squared = normal * normal;
        let mut least = usize::from(squared.y < squared.x);
        if squared.z < squared[least] {
            least = 2;
        }
        let mut perpendicular = -normal * normal[least];
        perpendicular[least] += 1.;
        let perpendicular = perpendicular.normalize();
        let tangent = perpendicular.cross(normal);
        let convert = |v: Vec3| Vec3::new(v.x, v.z, -v.y);
        // Local source x/y/z first pass through the runtime's native() basis.
        let rotation = Mat3::from_cols(convert(normal), convert(perpendicular), -convert(tangent));
        Transform::from_translation(center).with_rotation(Quat::from_mat3(&rotation))
    }

    pub(crate) fn emit_explosion(
        &self,
        commands: &mut Commands,
        impact_type: u32,
        surface: u8,
        explicit_effect: Option<&str>,
        center: Vec3,
        normal: Vec3,
    ) {
        let pose = self.explosion_pose(center, normal);
        if let Some(effect) = self.impact(impact_type, surface, 0) {
            super::emit(commands, effect, pose);
        }
        if let Some(effect) = explicit_effect {
            super::emit(commands, effect, pose);
        }
    }
}
