use bevy::{prelude::*, render::render_resource::ShaderType};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Reflect, ShaderType)]
pub(super) struct NativeWeaponParameters {
    pub response: Vec4,
    pub detail: Vec4,
    pub ember: Vec4,
    pub motion: Vec4,
    pub flicker: Vec4,
    pub layer_scale_offset: [Vec4; 3],
    pub layer_scroll_rotation: [Vec4; 3],
    pub layer_origin_depth: [Vec4; 3],
    pub layer_alpha: Vec4,
    pub staff: Vec4,
    pub staff_emission: Vec4,
    pub camo: Vec4,
    pub camo_extra: Vec4,
    pub camo_flags: UVec4,
    pub weapon_detail: Vec4,
    pub kind: UVec4,
}

impl Default for NativeWeaponParameters {
    fn default() -> Self {
        Self {
            response: Vec4::ONE,
            detail: Vec4::new(1., 1., 0., 1.),
            ember: Vec4::new(1., 1., 1., 0.),
            motion: Vec4::ZERO,
            flicker: Vec4::ONE,
            layer_scale_offset: [Vec4::new(1., 1., 0., 0.); 3],
            layer_scroll_rotation: [Vec4::ZERO; 3],
            layer_origin_depth: [Vec4::ZERO; 3],
            layer_alpha: Vec4::ONE,
            staff: Vec4::new(1., 1., 1., 2.),
            staff_emission: Vec4::ONE,
            camo: Vec4::new(1., 1., 0., 1.),
            camo_extra: Vec4::ZERO,
            camo_flags: UVec4::ZERO,
            weapon_detail: Vec4::ONE,
            kind: UVec4::ZERO,
        }
    }
}

impl NativeWeaponParameters {
    pub(super) fn read(surface: &Value) -> Self {
        let native = &surface["native_weapon"];
        let kind = match native["kind"].as_str() {
            Some("lit" | "cloth" | "skin") => 1,
            Some("transparent") => 2,
            Some("unlit") => 3,
            Some("ember") => 4,
            Some("phong") => 5,
            Some("badge") => 6,
            Some("staff") => 7,
            Some("lens") => 8,
            Some("reticle") => 9,
            Some("camo") => 10,
            _ => return Self::default(),
        };
        let constants = &native["constants"];
        let value = |names: &[&str], index: usize, default: f32| {
            names
                .iter()
                .find_map(|name| constants[*name][index].as_f64())
                .map_or(default, |v| v as f32)
        };
        let channel = |role| {
            surface["channels"]
                .as_array()
                .is_some_and(|channels| channels.iter().any(|c| c["role"] == role))
        };
        let mut flags = 0;
        for (i, role) in [
            "specular_gloss",
            "normal_detail",
            "occlusion",
            "detail",
            "gloss",
            "emissive",
            "heat",
            "reveal",
        ]
        .iter()
        .enumerate()
        {
            flags |= (channel(*role) as u32) << i;
        }
        flags |= (channel("specular") as u32) << 0;
        flags |= (channel("specular_detail") as u32) << 8;
        flags |= (native["family"]
            .as_str()
            .is_some_and(|s| s.contains("char_skin") || s.contains("char_cloth"))
            as u32)
            << 9;
        flags |= match native["family"].as_str() {
            Some("mc_sw4_3d_viewmodel_unlit_8216051u") => 1024,
            Some("mc_sw4_3d_viewmodel_unlit_60466j88") => 2048,
            Some("mc_sw4_3d_viewmodel_unlit_j299w86e") => 4096,
            _ => 0,
        };
        flags |= (native["family"]
            .as_str()
            .is_some_and(|s| s.contains("viewmodel_unlit")) as u32)
            << 13;
        flags |= ((native["family"]
            .as_str()
            .is_some_and(|s| s.contains("n0x0"))
            && !channel("specular_gloss")
            && !channel("specular")) as u32)
            << 14;
        let layer = |i: usize, suffix: &str, j, default| {
            value(&[&format!("Layer{}{suffix}", i + 1)], j, default)
        };
        let camo = |i, default| {
            native["shader_consts"][i]
                .as_f64()
                .map_or(default, |v| v as f32)
        };
        Self {
            layer_scale_offset: std::array::from_fn(|i| {
                Vec4::new(
                    layer(i, "Scale", 0, 1.),
                    layer(i, "Scale", 1, 1.),
                    layer(i, "Offset", 0, 0.),
                    layer(i, "Offset", 1, 0.),
                )
            }),
            layer_scroll_rotation: std::array::from_fn(|i| {
                Vec4::new(
                    layer(i, "Scroll", 0, 0.),
                    layer(i, "Scroll", 1, 0.),
                    layer(i, "Rotation", 0, 0.),
                    layer(i, "Rotation", 1, 0.),
                )
            }),
            layer_origin_depth: std::array::from_fn(|i| {
                Vec4::new(
                    layer(i, "Origin", 0, 0.),
                    layer(i, "Origin", 1, 0.),
                    layer(i, "Depth", 0, 0.),
                    0.,
                )
            }),
            layer_alpha: Vec4::new(
                layer(0, "Alpha", 0, 1.),
                layer(1, "Alpha", 0, 1.),
                layer(2, "Alpha", 0, 1.),
                value(&["Gloss_Amount"], 0, 0.5),
            ),
            staff: Vec4::new(
                value(&["WarpScale"], 0, 1.),
                value(&["WarpScale"], 1, 1.),
                value(&["NormalHeight"], 0, 1.),
                value(&["Charge_Level"], 0, 2.),
            ),
            staff_emission: Vec4::new(
                value(&["Emissive_Amount_Off"], 0, 1.),
                value(&["Emissive_Amount_On"], 0, 1.),
                0.,
                0.,
            ),
            camo: if kind == 9 {
                Vec4::new(
                    value(&["Reticle_Color"], 0, 1.),
                    value(&["Reticle_Color"], 1, 1.),
                    value(&["Reticle_Color"], 2, 1.),
                    value(&["Color_Map_Scale"], 0, 1.),
                )
            } else {
                Vec4::new(camo(0, 1.), camo(1, 1.), camo(2, 0.), camo(3, 1.))
            },
            camo_extra: if kind == 9 {
                Vec4::new(
                    value(&["Glow_Color"], 0, 1.),
                    value(&["Glow_Color"], 1, 1.),
                    value(&["Glow_Color"], 2, 1.),
                    0.,
                )
            } else if kind == 10 {
                Vec4::new(camo(4, 0.), camo(5, 0.), camo(6, 0.), camo(7, 0.))
            } else {
                Vec4::new(0., 0., 1., 0.)
            },
            camo_flags: UVec4::new(
                native["replace_flags"]["color"].as_bool().unwrap_or(false) as u32,
                native["replace_flags"]["normal"].as_bool().unwrap_or(false) as u32,
                native["replace_flags"]["specular"]
                    .as_bool()
                    .unwrap_or(false) as u32,
                native["family"]
                    .as_str()
                    .is_some_and(|s| s.contains("3layer")) as u32,
            ),
            weapon_detail: if kind == 9 {
                Vec4::new(
                    value(&["Noise_Scale"], 0, 1.),
                    value(&["Glow_Falloff"], 0, 1.),
                    value(&["Color_Map_Noise"], 0, 0.),
                    value(&["Background"], 0, 0.),
                )
            } else {
                Vec4::new(
                    value(&["Specular_Decay_Threshold"], 0, 1.),
                    value(&["occlusionAmount"], 1, 1.),
                    value(&["occlusionAmount"], 2, 1.),
                    0.,
                )
            },
            response: Vec4::new(
                value(&["Specular_Amount", "SpecularAmount"], 0, 1.),
                value(&["Reflection_Amount", "ReflectionAmount"], 0, 1.),
                value(&["Gloss_Push"], 0, 1.),
                value(
                    &[
                        "Emissive_Amount",
                        "Emissiver_Amount",
                        "hdrAmount",
                        "scaleRGB",
                    ],
                    0,
                    1.,
                ),
            ),
            detail: Vec4::new(
                value(&["colorDetailScale", "Detail_Normal_Tile"], 0, 1.),
                value(
                    &["colorDetailScale", "Detail_Normal_Tile"],
                    1,
                    value(&["Detail_Normal_Tile"], 0, 1.),
                ),
                value(&["colorDetailScale"], 2, 1.),
                value(&["occlusionAmount"], 0, 1.),
            ),
            ember: Vec4::new(
                value(&["Ember_Scale"], 0, 1.),
                value(&["Heat_Scale"], 0, 1.),
                value(&["Diffuse_Normal_Height_Facing"], 0, 1.),
                value(&["Seed_Value"], 0, 0.),
            ),
            motion: Vec4::new(
                value(&["Ember_Direction"], 0, 0.),
                value(&["Ember_Direction"], 1, 0.),
                value(&["Heat_Direction"], 0, 0.),
                value(&["Heat_Direction"], 1, 0.),
            ),
            flicker: Vec4::new(
                value(&["Flicker_Min"], 0, 1.),
                value(&["Flicker_Max"], 0, 1.),
                value(&["Emissive_Push"], 0, 1.),
                value(&["Specular_Push"], 0, 1.),
            ),
            kind: UVec4::new(kind, flags, channel("normal") as u32, channel("rim") as u32),
        }
    }
}

#[derive(Component)]
pub(super) struct StaffMaterial {
    pub owner: Option<Entity>,
}

pub(super) fn charge(
    entities: Query<(
        &StaffMaterial,
        &MeshMaterial3d<super::opaque::OpaqueMaterial>,
    )>,
    inventories: Query<&crate::weapons::WeaponInventory>,
    mut materials: ResMut<Assets<super::opaque::OpaqueMaterial>>,
) {
    for (staff, handle) in &entities {
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        let level = staff
            .owner
            .and_then(|owner| inventories.get(owner).ok())
            .map_or(0, |i| i.charge_level);
        if material.extension.parameters.native.staff_emission.z != level as f32 {
            materials
                .get_mut(&handle.0)
                .unwrap()
                .extension
                .parameters
                .native
                .staff_emission
                .z = level as f32;
        }
    }
}
