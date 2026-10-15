use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

macro_rules! identifiers {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
    )+};
}
identifiers!(ContentId, ZoneId, WeaponId, PerkId, QuestId, InteractionId);

#[derive(
    bevy::prelude::Component,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
)]
#[serde(transparent)]
pub struct PlacementId(pub String);

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageCategory {
    Bullet,
    Melee,
    Explosive,
    Fire,
    Ice,
    Lightning,
    Wind,
    Environmental,
    Crush,
}

#[derive(Debug)]
pub enum ContentError {
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    Json {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    UnsupportedVersion(u32),
    InvalidPath(String),
    InvalidDefinition(String),
    DuplicateId(String),
    MissingReference {
        owner: String,
        target: String,
    },
}
impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(
                f,
                "Cannot read {}: {source}. Restore the required asset files.",
                path.display()
            ),
            Self::Json { path, source } => write!(
                f,
                "Invalid {}: {source}. Regenerate/import game.json.",
                path.display()
            ),
            Self::UnsupportedVersion(v) => write!(
                f,
                "Unsupported content version {v}; expected 2. Restore a supported game.json definition."
            ),
            Self::InvalidPath(p) => write!(f, "Unsafe runtime asset path: {p}"),
            Self::InvalidDefinition(reason) => f.write_str(reason),
            Self::DuplicateId(id) => write!(f, "Duplicate or malformed stable content ID: {id}"),
            Self::MissingReference { owner, target } => {
                write!(f, "Content {owner} refers to missing ID {target}")
            }
        }
    }
}
impl std::error::Error for ContentError {}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDefinition {
    pub id: WeaponId,
    pub name: String,
    #[serde(default)]
    pub native_view_model_index: Option<u8>,
    #[serde(default)]
    pub hud: Option<WeaponHud>,
    pub damage: u32,
    pub category: DamageCategory,
    pub magazine: u32,
    pub reserve: u32,
    pub fire_interval: f32,
    pub reload_seconds: f32,
    pub upgraded: Option<WeaponId>,
    pub presentation: Option<ContentId>,
    #[serde(default)]
    pub ads_presentation: Option<ContentId>,
    #[serde(default)]
    pub dual_weapon: Option<WeaponId>,
    #[serde(default)]
    pub pickup: Option<ContentId>,
    #[serde(default)]
    pub ammo_group: Option<ContentId>,
    #[serde(default)]
    pub localization_key: Option<String>,
    #[serde(default)]
    pub animation_actions: BTreeMap<WeaponAnimation, ContentId>,
    #[serde(default)]
    pub view_offset: [f32; 3],
    #[serde(default)]
    pub mechanics: WeaponMechanics,
    #[serde(default)]
    pub attachments: BTreeMap<String, AttachmentDefinition>,
    #[serde(default)]
    pub throwable: Option<crate::equipment::ThrowableDefinition>,
    #[serde(default)]
    pub mine: Option<crate::mines::MineDefinition>,
    #[serde(default)]
    pub lure: Option<crate::lures::LureDefinition>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponHud {
    pub center: Option<ContentId>,
    pub side: Option<ContentId>,
    pub center_size: f32,
    pub side_size: f32,
    pub side_position: f32,
    pub minimum_offset: f32,
    pub keep_when_ads: bool,
    pub low_ammo_fraction: f32,
}

#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AttachmentDefinition {
    pub native_selectors: Option<[u8; 3]>,
    pub alternate: Option<WeaponId>,
    pub ads_fov: Vec<f32>,
    pub ads_sight: Option<AdsSight>,
    pub ads_presentation: Option<ContentId>,
    pub presentation: Option<ContentId>,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub hide_tags: Vec<String>,
    pub ads_zoom_fraction: Option<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponAnimation {
    Idle,
    IdleEmpty,
    LastShot,
    Rechamber,
    AdsFire,
    AdsLastShot,
    AdsRechamber,
    Prime,
    Fire,
    Reload,
    ReloadEmpty,
    ReloadStart,
    ReloadEnd,
    Equip,
    Unequip,
    Melee,
    AdsEnter,
    AdsLeave,
    SprintEnter,
    SprintLoop,
    SprintLeave,
    ProneEnter,
    ProneLoop,
    ProneBack,
    ProneRight,
    ProneLeft,
    ProneLeave,
    ProneEnterEmpty,
    ProneLoopEmpty,
    ProneBackEmpty,
    ProneRightEmpty,
    ProneLeftEmpty,
    ProneLeaveEmpty,
    DiveEnter,
    DiveLoop,
    DiveLeave,
    DiveEnterEmpty,
    DiveLoopEmpty,
    DiveLeaveEmpty,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WeaponDelivery {
    #[default]
    Hitscan,
    Projectile,
    Melee,
    Equipment,
}
#[derive(Clone, Copy, Debug, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum FirePattern {
    #[default]
    Semi,
    Automatic,
    Burst {
        shots: u32,
    },
    Charge,
}
#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WeaponMechanics {
    pub delivery: WeaponDelivery,
    pub fire: FirePattern,
    pub starting_reserve: u32,
    pub minimum_damage: u32,
    pub damage_range: [f32; 2],
    pub range: f32,
    pub melee_extent: [f32; 2],
    pub hit_multipliers: [f32; 21],
    pub pellets: u32,
    pub penetration: Option<[f32; 32]>,
    pub recoil_pitch: [f32; 2],
    pub recoil_yaw: [f32; 2],
    pub recoil_recovery: f32,
    pub ads_recoil_recovery: Option<f32>,
    pub alternate: Option<WeaponId>,
    pub player_damage: [u32; 2],
    pub projectile_trail: Option<String>,
    pub projectile_loop: Option<String>,
    pub projectile_effect: Option<String>,
    pub weapon_class: Option<String>,
    pub spread: [f32; 4],
    pub ads_fov: f32,
    pub ads_seconds: [f32; 2],
    pub sprint_seconds: [f32; 3],
    pub crawl_seconds: [f32; 7],
    pub dive_seconds: [f32; 3],
    pub ads_zoom_fraction: [f32; 2],
    pub ads_pose: Vec<AdsPoseSample>,
    pub ads_sight: Option<AdsSight>,
    pub ads_enabled: bool,
    pub native_ads_pitch: Option<f32>,
    pub ads_translation: [f32; 3],
    pub ads_rotation: [f32; 4],
    pub burst_delay: f32,
    pub movement: [f32; 2],
    pub empty_reload: f32,
    pub reload_transfer_seconds: Option<[f32; 2]>,
    pub shell_reload: bool,
    pub shell_seconds: f32,
    pub reload_start_seconds: f32,
    pub reload_start_insert: f32,
    pub reload_end_seconds: f32,
    pub equip_seconds: f32,
    pub unequip_seconds: f32,
    pub dual_wield: bool,
    pub projectile_speed: f32,
    pub projectile_up_speed: f32,
    pub projectile_bounce: bool,
    pub projectile_lifetime: f32,
    pub explosion_radius: f32,
    pub explosion_damage: [u32; 2],
    pub melee_seconds: [f32; 2],
    pub fuse_seconds: f32,
}
impl Default for WeaponMechanics {
    fn default() -> Self {
        Self {
            delivery: WeaponDelivery::Hitscan,
            fire: FirePattern::Semi,
            starting_reserve: 32,
            minimum_damage: 20,
            damage_range: [20., 60.],
            range: 8192. * 0.0254,
            melee_extent: [10. * 0.0254; 2],
            hit_multipliers: [
                1., 3.5, 3.5, 1., 1.25, 1., 1., 0.85, 0.85, 0.85, 0.85, 0.85, 0.85, 0.6, 0.6, 0.6,
                0.6, 0.6, 0.6, 1., 1.,
            ],
            pellets: 1,
            penetration: None,
            recoil_pitch: [0.2, 0.4],
            recoil_yaw: [-0.1, 0.1],
            recoil_recovery: 8.,
            ads_recoil_recovery: None,
            alternate: None,
            player_damage: [0, 0],
            projectile_trail: None,
            projectile_loop: None,
            projectile_effect: None,
            weapon_class: None,
            spread: [3., 2., 1., 0.],
            ads_fov: 50.,
            ads_seconds: [0.2, 0.2],
            sprint_seconds: [0.; 3],
            crawl_seconds: [0.; 7],
            dive_seconds: [0.; 3],
            ads_zoom_fraction: [1., 1.],
            ads_pose: Vec::new(),
            ads_sight: None,
            ads_enabled: true,
            native_ads_pitch: None,
            ads_translation: [0.; 3],
            ads_rotation: [0., 0., 0., 1.],
            burst_delay: 0.,
            movement: [1., 0.5],
            empty_reload: 1.8,
            reload_transfer_seconds: None,
            shell_reload: false,
            shell_seconds: 0.5,
            reload_start_seconds: 0.,
            reload_start_insert: 0.,
            reload_end_seconds: 0.,
            equip_seconds: 0.5,
            unequip_seconds: 0.3,
            dual_wield: false,
            projectile_speed: 30.,
            projectile_up_speed: 0.,
            projectile_bounce: false,
            projectile_lifetime: 10.,
            explosion_radius: 0.,
            explosion_damage: [0, 0],
            melee_seconds: [0.25, 0.7],
            fuse_seconds: 4.,
        }
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdsPoseSample {
    pub fraction: f32,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdsSight {
    pub joint: String,
    pub point: [f32; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub id: ContentId,
    pub health: u32,
    pub damage: u32,
    pub presentation: Option<ContentId>,
    #[serde(default)]
    pub parts: Vec<ActorPart>,
    #[serde(default)]
    pub animation_actions: BTreeMap<EnemyAnimation, ContentId>,
    #[serde(default)]
    pub locomotion_states: BTreeMap<EnemyLocomotion, Vec<ContentId>>,
    #[serde(default)]
    pub combat_states: BTreeMap<EnemyCombat, Vec<EnemyCombatClip>>,
    #[serde(default)]
    pub traversals: BTreeMap<String, Vec<EnemyTraversalDefinition>>,
    #[serde(default = "default_attack_seconds")]
    pub attack_seconds: f32,
    #[serde(default)]
    pub attack_hits: Vec<f32>,
    #[serde(default = "default_death_seconds")]
    pub death_seconds: f32,
    #[serde(default = "default_bounds_margin")]
    pub bounds_margin: f32,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub archetype: String,
    #[serde(default)]
    pub character_group: String,
    #[serde(default)]
    pub character_index: Option<u8>,
    #[serde(default)]
    pub hit_regions: Vec<HitVolumeDefinition>,
    #[serde(default)]
    pub physics: Option<EnemyPhysics>,
    #[serde(default)]
    pub gibs: Option<EnemyGibs>,
    #[serde(default)]
    pub ragdoll: Option<EnemyRagdoll>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyRagdoll {
    pub bones: Vec<RagdollBone>,
    pub joints: Vec<RagdollJoint>,
    pub baselerp: Vec<RagdollBaseLerp>,
    pub self_pairs: Vec<[usize; 2]>,
}
impl EnemyRagdoll {
    fn valid(&self) -> bool {
        !self.bones.is_empty()
            && self.bones.len() <= 14
            && self.bones.iter().enumerate().all(|(index, bone)| {
                !bone.joint.is_empty()
                    && !bone.end_joint.is_empty()
                    && bone.radius.is_finite()
                    && bone.radius > 0.
                    && bone.half_length.is_finite()
                    && bone.half_length >= 0.
                    && bone.base.iter().all(|value| value.is_finite())
                    && bone.base[3].abs() < 0.0001
                    && bone.base[7].abs() < 0.0001
                    && bone.base[11].abs() < 0.0001
                    && (bone.base[15] - 1.).abs() < 0.0001
                    && bone.mass.is_finite()
                    && bone.mass > 0.
                    && bone.friction.is_finite()
                    && bone.friction >= 0.
                    && bone.parent.is_none_or(|parent| parent < index)
            })
            && self.joints.len() <= 28
            && self.joints.iter().all(|joint| {
                joint.bone < self.bones.len()
                    && self.bones[joint.bone].parent.is_some()
                    && !joint.limits.is_empty()
                    && joint.limits.len() <= 3
                    && joint.limits.iter().all(|limit| {
                        limit.axis.iter().all(|value| value.is_finite())
                            && (limit.axis.iter().map(|value| value * value).sum::<f32>() - 1.)
                                .abs()
                                < 0.0001
                            && limit.friction.is_finite()
                            && limit.friction >= 0.
                            && limit.minimum.is_finite()
                            && limit.maximum.is_finite()
                            && limit.minimum <= limit.maximum
                            && limit.minimum >= -std::f32::consts::PI
                            && limit.maximum <= std::f32::consts::PI
                    })
            })
            && self.baselerp.iter().all(|bone| {
                !bone.joint.is_empty()
                    && bone.bone < self.bones.len()
                    && bone.seconds.is_finite()
                    && (0.1..=6.).contains(&bone.seconds)
                    && bone.base.iter().all(|value| value.is_finite())
                    && bone.base[3].abs() < 0.0001
                    && bone.base[7].abs() < 0.0001
                    && bone.base[11].abs() < 0.0001
                    && (bone.base[15] - 1.).abs() < 0.0001
            })
            && self.self_pairs.iter().all(|pair| {
                pair[0] < self.bones.len() && pair[1] < self.bones.len() && pair[0] != pair[1]
            })
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RagdollBone {
    pub joint: String,
    pub end_joint: String,
    pub radius: f32,
    pub half_length: f32,
    pub base: [f32; 16],
    pub mass: f32,
    pub friction: f32,
    pub parent: Option<usize>,
    pub mirror: bool,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RagdollJointKind {
    Hinge,
    Swivel,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RagdollJoint {
    pub bone: usize,
    pub kind: RagdollJointKind,
    pub limits: Vec<RagdollLimit>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RagdollLimit {
    pub axis: [f32; 3],
    pub friction: f32,
    pub minimum: f32,
    pub maximum: f32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RagdollBaseLerp {
    pub joint: String,
    pub bone: usize,
    pub seconds: f32,
    pub base: [f32; 16],
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyGibs {
    pub models: BTreeMap<crate::combat_effects::BodyGib, EnemyGibModels>,
    pub beheaded: ContentId,
    pub pieces: [EnemyGibPiece; 4],
    pub hat: Option<EnemyGibPiece>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyGibPiece {
    pub presentation: ContentId,
    pub joint: String,
    pub physics: NativeModelPhysics,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeModelPhysics {
    pub mass: f32,
    pub bounce: f32,
    pub friction: f32,
    pub bullet_force_scale: f32,
    pub explosive_force_scale: f32,
    pub gravity_scale: f32,
    pub center: [f32; 3],
    pub shapes: Vec<Vec<[f32; 3]>>,
}
impl NativeModelPhysics {
    fn valid(&self) -> bool {
        self.mass.is_finite()
            && self.mass > 0.
            && self.bounce.is_finite()
            && (0. ..=1.).contains(&self.bounce)
            && self.friction.is_finite()
            && self.friction >= 0.
            && self.bullet_force_scale.is_finite()
            && self.bullet_force_scale >= 0.
            && self.explosive_force_scale.is_finite()
            && self.explosive_force_scale >= 0.
            && self.gravity_scale.is_finite()
            && self.center.iter().all(|value| value.is_finite())
            && !self.shapes.is_empty()
            && self.shapes.iter().all(|points| {
                points.len() >= 4
                    && points.iter().flatten().all(|value| value.is_finite())
                    && avian3d::prelude::Collider::convex_hull(
                        &points
                            .iter()
                            .copied()
                            .map(bevy::prelude::Vec3::from_array)
                            .collect::<Vec<_>>(),
                    )
                    .is_some()
            })
    }
}
impl EnemyGibs {
    pub fn presentations(&self) -> impl Iterator<Item = &ContentId> {
        self.models
            .values()
            .flat_map(|models| [&models.body, &models.legs])
            .chain(std::iter::once(&self.beheaded))
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyGibModels {
    pub body: ContentId,
    pub legs: ContentId,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyPhysics {
    pub radius: f32,
    pub height: f32,
    pub crawler_height: f32,
}
impl EnemyPhysics {
    pub fn height(self, crawler: bool) -> f32 {
        if crawler {
            self.crawler_height
        } else {
            self.height
        }
    }
    pub fn center(self, crawler: bool) -> bevy::prelude::Vec3 {
        bevy::prelude::Vec3::Y * (self.height(crawler) * 0.5)
    }
    fn valid(self) -> bool {
        [self.radius, self.height, self.crawler_height]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.)
            && self.crawler_height < self.height
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitVolumeDefinition {
    pub id: String,
    pub region: crate::weapons::HitRegion,
    pub presentation: ContentId,
    pub joint: String,
    pub bounds: [[f32; 3]; 2],
    pub sphere: [f32; 4],
}
fn default_attack_seconds() -> f32 {
    1.8
}
fn default_death_seconds() -> f32 {
    3.
}
fn default_bounds_margin() -> f32 {
    2.5
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorPart {
    pub presentation: ContentId,
    pub socket: String,
    pub root_joint: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyTraversalDefinition {
    pub animation: ContentId,
    pub seconds: f32,
    pub motion: Vec<[f32; 4]>,
}
impl EnemyTraversalDefinition {
    pub fn valid(&self) -> bool {
        self.seconds.is_finite()
            && (0.1..=30.).contains(&self.seconds)
            && (2..=4096).contains(&self.motion.len())
            && self.motion.iter().all(|p| {
                p.iter().all(|v| v.is_finite()) && p[0] >= 0. && p[0] <= self.seconds + 0.001
            })
            && self.motion.windows(2).all(|p| p[0][0] < p[1][0])
            && self.motion.first().is_some_and(|p| p[0] == 0.)
            && self
                .motion
                .last()
                .is_some_and(|p| (p[0] - self.seconds).abs() < 0.001)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyAnimation {
    Idle,
    Walk,
    Run,
    Sprint,
    Attack,
    Death,
    Traverse,
    Landing,
    Tear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyCombat {
    WalkMelee,
    RunMelee,
    WalkMeleeCrawl,
    RunMeleeCrawl,
    StumpyMelee,
    GeneratorMelee,
    Death,
    DeathCrawl,
    FireDeath,
    ShieldMelee,
    ShieldMeleeCrawl,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyCombatClip {
    pub animation: ContentId,
    pub seconds: f32,
    #[serde(default)]
    pub duration: Option<f32>,
    pub hits: Vec<f32>,
    #[serde(default)]
    pub stops: Vec<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyLocomotion {
    Burned,
    Walk,
    WalkCrawl,
    Run,
    RunCrawl,
    Sprint,
    SprintCrawl,
    SuperSprint,
    SuperSprintCrawl,
    Stumpy,
}
impl EnemyLocomotion {
    pub fn for_action(action: EnemyAnimation) -> Option<Self> {
        match action {
            EnemyAnimation::Walk => Some(Self::Walk),
            EnemyAnimation::Run => Some(Self::Run),
            EnemyAnimation::Sprint => Some(Self::Sprint),
            _ => None,
        }
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkDefinition {
    pub id: PerkId,
    #[serde(default)]
    pub generator: Option<ContentId>,
    pub name: String,
    pub price: u32,
    pub icon: Option<ContentId>,
    #[serde(default)]
    pub presentation: Option<ContentId>,
    #[serde(default)]
    pub off_presentation: Option<ContentId>,
    #[serde(default)]
    pub collision: Vec<crate::environment::VolumeDefinition>,
    #[serde(default)]
    pub path_bounds_radius: Option<f32>,
    #[serde(default)]
    pub bump_trigger: Option<String>,
    #[serde(default)]
    pub drink: Option<PerkDrinkDefinition>,
    #[serde(default)]
    pub maximum_health: Option<u32>,
    #[serde(default)]
    pub reload_duration_scale: Option<f32>,
    #[serde(default)]
    pub sprint: Option<PerkSprintDefinition>,
    #[serde(default)]
    pub third_weapon: bool,
    #[serde(default)]
    pub firing: Option<PerkFiringDefinition>,
    #[serde(default)]
    pub cherry: Option<crate::perk_effects::CherryDefinition>,
    #[serde(default)]
    pub explosive_protection: bool,
    #[serde(default)]
    pub fall_protection: bool,
    #[serde(default)]
    pub dive: Option<crate::perk_effects::DiveDefinition>,
}
impl PerkDefinition {
    pub fn has_supported_effect(&self) -> bool {
        self.maximum_health.is_some()
            || self.reload_duration_scale.is_some()
            || self.sprint.is_some()
            || self.third_weapon
            || self.firing.is_some()
            || self.cherry.is_some()
            || self.explosive_protection
            || self.fall_protection
            || self.dive.is_some()
    }
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkFiringDefinition {
    pub interval_scale: f32,
    pub bullet_copies: u32,
    pub hip_spread_scale: f32,
}
impl Default for PerkFiringDefinition {
    fn default() -> Self {
        Self {
            interval_scale: 1.,
            bullet_copies: 1,
            hip_spread_scale: 1.,
        }
    }
}
impl PerkFiringDefinition {
    pub fn copies(&self, weapon: &WeaponDefinition) -> u32 {
        if matches!(weapon.category, DamageCategory::Bullet)
            && weapon.mechanics.delivery == WeaponDelivery::Hitscan
        {
            self.bullet_copies
        } else {
            1
        }
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkSprintDefinition {
    pub duration_scale: f32,
    pub speed_scale: f32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkDrinkDefinition {
    pub bottle: WeaponId,
    pub seconds: f32,
    pub lower_seconds: f32,
    pub activation_delay: f32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerupDefinition {
    pub id: ContentId,
    pub duration: f32,
    pub presentation: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneDefinition {
    pub id: ZoneId,
    pub initially_open: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub id: PlacementId,
    pub definition: ContentId,
    pub position: [f32; 3],
    pub yaw: f32,
    pub zone: ZoneId,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionPlacement {
    pub id: InteractionId,
    pub placement: PlacementId,
    pub action: InteractionAction,
    pub reach: f32,
    pub hold_seconds: f32,
    pub hidden_prompt: bool,
    #[serde(default)]
    pub source_trigger: Option<String>,
    #[serde(default)]
    pub generator: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionAction {
    Barrier {
        barrier: ContentId,
    },
    Door {
        connection: ContentId,
        price: u32,
    },
    Weapon {
        weapon: WeaponId,
        price: u32,
        #[serde(default)]
        ammo_price: Option<u32>,
    },
    Perk {
        perk: PerkId,
    },
    Generator {
        generator: ContentId,
    },
    Quest {
        quest: QuestId,
        event: String,
    },
    Pickup {
        item: ContentId,
    },
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneConnection {
    pub id: ContentId,
    pub from: ZoneId,
    pub to: ZoneId,
    pub initially_open: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationNode {
    pub id: ContentId,
    pub position: [f32; 3],
    pub zone: ZoneId,
    #[serde(default)]
    pub native: Option<NativePathNode>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePathNode {
    pub r#type: u8,
    pub spawn_flags: i32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationLink {
    pub id: ContentId,
    pub from: ContentId,
    pub to: ContentId,
    pub traversal: Traversal,
    #[serde(default)]
    pub cost: Option<f32>,
    pub connection: Option<ContentId>,
    #[serde(default)]
    pub blockers: Vec<ContentId>,
    #[serde(default)]
    pub source_animation: Option<String>,
    #[serde(default)]
    pub negotiation: Option<NativeNegotiation>,
}
#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NegotiationNode {
    pub index: u16,
    pub r#type: u8,
    pub spawn_flags: i32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub forward: [f32; 3],
    pub radius: f32,
    pub minimum_use_distance_squared: f32,
    pub overlap: [i16; 2],
    pub targetname: String,
    pub link_name: String,
    pub noteworthy: String,
    pub target: String,
}
impl NegotiationNode {
    fn valid(&self, kind: u8, identity: &ContentId, nodes: &[NavigationNode]) -> bool {
        self.index < u16::MAX
            && self.r#type == kind
            && identity.0 == format!("node.native.{}", self.index)
            && self.position.iter().all(|x| x.is_finite())
            && self.yaw.is_finite()
            && self.forward.iter().all(|x| x.is_finite())
            && self.forward[1] == 0.
            && (self.forward[0] - self.yaw.cos()).abs() < 0.0001
            && (self.forward[2] + self.yaw.sin()).abs() < 0.0001
            && self.radius.is_finite()
            && self.radius >= 0.
            && self.minimum_use_distance_squared.is_finite()
            && self.minimum_use_distance_squared >= 0.
            && [
                &self.targetname,
                &self.link_name,
                &self.noteworthy,
                &self.target,
            ]
            .iter()
            .all(|s| s.len() < 128)
            && nodes.iter().find(|n| n.id == *identity).is_some_and(|n| {
                n.native.is_none_or(|native| {
                    native.r#type == self.r#type && native.spawn_flags == self.spawn_flags
                }) && n.position
                    .iter()
                    .zip(self.position)
                    .all(|(a, b)| (a - b).abs() < 0.0001)
            })
    }
}
#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeNegotiation {
    pub cost: f32,
    pub flags: u8,
    pub disconnect_count: u8,
    pub start: NegotiationNode,
    pub end: NegotiationNode,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Traversal {
    Walk,
    Climb,
    Jump,
    Rise,
    Portal,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorDefinition {
    pub id: ContentId,
    pub placement: PlacementId,
    pub zones: Vec<ZoneId>,
    pub price: u32,
    pub capture_seconds: f32,
    pub radius: f32,
    pub number: u8,
    pub decay_seconds: f32,
    pub shutdown_seconds: f32,
    pub recovery_seconds: f32,
    pub presentation: ContentId,
    pub model_position: [f32; 3],
    pub emergence: Vec<GeneratorEmergence>,
    pub capture_enemy: ContentId,
    pub attack_points: Vec<GeneratorAttackPoint>,
    pub clip_seconds: [f32; 8],
    pub capture_sound: ContentId,
    pub active_sound: ContentId,
    #[serde(default)]
    pub initial_reward: Option<GeneratorReward>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorReward {
    pub position: [f32; 3],
    pub yaw: f32,
    pub presentation: ContentId,
    pub powerup: ContentId,
    pub source_trigger: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorEmergence {
    pub position: [f32; 3],
    pub yaw: f32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorAttackPoint {
    pub position: [f32; 3],
    pub face: [f32; 3],
    pub inaccessible: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDefinition {
    pub id: ContentId,
    pub points: Vec<[f32; 3]>,
    pub speed: f32,
    pub cooldown: f32,
    pub presentation: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherDefinition {
    pub id: ContentId,
    pub duration: [f32; 2],
    pub snow: bool,
    pub sound: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigDefinition {
    pub id: ContentId,
    pub placement: PlacementId,
    pub snow_only: bool,
    pub loot: Vec<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftingRecipe {
    pub id: ContentId,
    pub parts: Vec<ContentId>,
    pub result: ContentId,
    pub bench: PlacementId,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaffPuzzle {
    pub id: ContentId,
    pub weapon: WeaponId,
    pub interactions: Vec<InteractionId>,
    pub solution: Vec<String>,
    pub souls: u32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTransition {
    pub id: ContentId,
    pub quest: QuestId,
    pub from: String,
    pub to: String,
    pub event: String,
    pub prerequisites: Vec<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDefinition {
    pub id: QuestId,
    pub initial_stage: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationDefinition {
    pub id: ContentId,
    pub scene: Option<String>,
    pub animations: Vec<ContentId>,
    pub sounds: Vec<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundDefinition {
    pub id: ContentId,
    pub path: String,
    pub volume: f32,
    pub pitch: [f32; 2],
    pub distance: f32,
    #[serde(default)]
    pub distance_min: f32,
    pub looping: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationDefinition {
    pub id: ContentId,
    pub path: String,
    pub skeleton: ContentId,
    pub seconds: f32,
    pub notifications: Vec<AnimationNotification>,
    #[serde(default)]
    pub looping: bool,
    #[serde(default)]
    pub root_motion: Option<AnimationRootMotion>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationRootMotion {
    pub seconds: f32,
    pub translation: Vec<[f32; 4]>,
    pub rotation: Vec<[f32; 5]>,
}
impl AnimationRootMotion {
    pub fn valid(&self) -> bool {
        fn track<const N: usize>(values: &[[f32; N]], seconds: f32) -> bool {
            values.is_empty()
                || ((2..=65536).contains(&values.len())
                    && values.iter().all(|sample| {
                        sample.iter().all(|v| v.is_finite())
                            && sample[0] >= 0.
                            && sample[0] <= seconds + 0.001
                    })
                    && values.windows(2).all(|pair| pair[0][0] < pair[1][0])
                    && values.first().is_some_and(|sample| sample[0] == 0.)
                    && values
                        .last()
                        .is_some_and(|sample| (sample[0] - seconds).abs() < 0.001))
        }
        self.seconds.is_finite()
            && self.seconds > 0.
            && track(&self.translation, self.seconds)
            && track(&self.rotation, self.seconds)
            && self
                .rotation
                .iter()
                .all(|sample| sample[1..].iter().map(|v| v * v).sum::<f32>() > 0.001)
    }
    pub fn translation_at(&self, seconds: f32) -> [f32; 3] {
        let Some(first) = self.translation.first() else {
            return [0.; 3];
        };
        if seconds <= 0. {
            return [first[1], first[2], first[3]];
        }
        for pair in self.translation.windows(2) {
            if seconds <= pair[1][0] {
                let fraction = ((seconds - pair[0][0]) / (pair[1][0] - pair[0][0])).clamp(0., 1.);
                return std::array::from_fn(|axis| {
                    pair[0][axis + 1] + (pair[1][axis + 1] - pair[0][axis + 1]) * fraction
                });
            }
        }
        let last = self.translation.last().unwrap();
        [last[1], last[2], last[3]]
    }
    pub fn loop_displacement(&self, start: f32, end: f32) -> [f32; 3] {
        let start_cycles = (start / self.seconds).floor();
        let end_cycles = (end / self.seconds).floor();
        let first = self.translation_at(0.);
        let last = self.translation_at(self.seconds);
        let a = self.translation_at(start.rem_euclid(self.seconds));
        let b = self.translation_at(end.rem_euclid(self.seconds));
        std::array::from_fn(|axis| {
            b[axis] - a[axis] + (end_cycles - start_cycles) * (last[axis] - first[axis])
        })
    }
    pub fn rotation_at(&self, seconds: f32) -> [f32; 4] {
        let Some(first) = self.rotation.first() else {
            return [0., 0., 0., 1.];
        };
        let mut value = [first[1], first[2], first[3], first[4]];
        for pair in self.rotation.windows(2) {
            let fraction = ((seconds - pair[0][0]) / (pair[1][0] - pair[0][0])).clamp(0., 1.);
            let sign = if (1..5)
                .map(|axis| pair[0][axis] * pair[1][axis])
                .sum::<f32>()
                < 0.
            {
                -1.
            } else {
                1.
            };
            value = std::array::from_fn(|axis| {
                pair[0][axis + 1] + (pair[1][axis + 1] * sign - pair[0][axis + 1]) * fraction
            });
            if seconds <= pair[1][0] {
                break;
            }
        }
        let length = value.iter().map(|v| v * v).sum::<f32>().sqrt();
        value.map(|v| v / length)
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationNotification {
    pub seconds: f32,
    pub event: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiAsset {
    pub id: ContentId,
    pub path: String,
    #[serde(default)]
    pub font: Option<BitmapFont>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BitmapFont {
    pub height: f32,
    pub atlas: [f32; 2],
    pub glyphs: Vec<BitmapGlyph>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BitmapGlyph {
    pub codepoint: u32,
    pub offset: [f32; 2],
    pub advance: f32,
    pub size: [f32; 2],
    pub rect: [f32; 4],
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizedString {
    pub id: ContentId,
    pub key: String,
    pub text: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CinematicSequence {
    pub id: ContentId,
    #[serde(default)]
    pub frame_directory: Option<String>,
    #[serde(default)]
    pub frames: Vec<String>,
    pub frames_per_second: f32,
    pub audio: Option<ContentId>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentCatalog {
    #[serde(default)]
    pub barriers: Vec<crate::barriers::BarrierDefinition>,
    pub weapons: Vec<WeaponDefinition>,
    pub enemies: Vec<EnemyDefinition>,
    pub perks: Vec<PerkDefinition>,
    #[serde(default)]
    pub powerups: Vec<PowerupDefinition>,
    pub zones: Vec<ZoneDefinition>,
    pub definitions: Vec<ContentId>,
    pub placements: Vec<Placement>,
    pub interactions: Vec<InteractionPlacement>,
    pub zone_connections: Vec<ZoneConnection>,
    pub navigation_nodes: Vec<NavigationNode>,
    pub navigation_links: Vec<NavigationLink>,
    #[serde(default)]
    pub generators: Vec<GeneratorDefinition>,
    #[serde(default)]
    pub robot_routes: Vec<RouteDefinition>,
    #[serde(default)]
    pub tank_routes: Vec<RouteDefinition>,
    #[serde(default)]
    pub weather: Vec<WeatherDefinition>,
    #[serde(default)]
    pub digging: Vec<DigDefinition>,
    #[serde(default)]
    pub recipes: Vec<CraftingRecipe>,
    #[serde(default)]
    pub staff_puzzles: Vec<StaffPuzzle>,
    #[serde(default)]
    pub quests: Vec<QuestDefinition>,
    #[serde(default)]
    pub quest_transitions: Vec<QuestTransition>,
    pub presentations: Vec<PresentationDefinition>,
    pub sounds: Vec<SoundDefinition>,
    pub animations: Vec<AnimationDefinition>,
    pub ui_assets: Vec<UiAsset>,
    #[serde(default)]
    pub cinematics: Vec<CinematicSequence>,
    #[serde(default)]
    pub localized_strings: Vec<LocalizedString>,
}

pub struct ContentIndex<'a>(BTreeMap<&'a str, &'static str>);
impl<'a> ContentIndex<'a> {
    fn insert(&mut self, id: &'a str, kind: &'static str) -> Result<(), ContentError> {
        if id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || self.0.insert(id, kind).is_some()
        {
            return Err(ContentError::DuplicateId(id.into()));
        }
        Ok(())
    }
    pub fn resolve(&self, owner: &str, target: &str, kind: &str) -> Result<(), ContentError> {
        if self
            .0
            .get(target)
            .is_some_and(|found| kind == "any" || *found == kind)
        {
            Ok(())
        } else {
            Err(ContentError::MissingReference {
                owner: owner.into(),
                target: target.into(),
            })
        }
    }
}
fn valid(condition: bool, label: &str) -> Result<(), ContentError> {
    if condition {
        Ok(())
    } else {
        Err(ContentError::InvalidDefinition(label.into()))
    }
}
fn positive(v: f32) -> bool {
    v.is_finite() && v > 0.
}
fn nonnegative(v: f32) -> bool {
    v.is_finite() && v >= 0.
}

#[cfg(test)]
mod weapon_tuning_tests {
    use super::*;

    #[test]
    fn native_empty_root_delta_is_identity_and_nonempty_tracks_remain_checked() {
        let mut motion = AnimationRootMotion {
            seconds: 1.,
            translation: Vec::new(),
            rotation: Vec::new(),
        };
        assert!(motion.valid());
        for seconds in [-1., 0., 0.5, 1., 2.] {
            assert_eq!(motion.translation_at(seconds), [0.; 3]);
            assert_eq!(motion.rotation_at(seconds), [0., 0., 0., 1.]);
        }
        assert_eq!(motion.loop_displacement(0., 3.), [0.; 3]);
        for seconds in [0., -1., f32::NAN, f32::INFINITY] {
            motion.seconds = seconds;
            assert!(!motion.valid());
        }
        motion.seconds = 1.;
        motion.translation = vec![[0., 0., 0., 0.], [0.5, 1., 0., 0.], [1., 2., 0., 0.]];
        assert!(motion.valid());
        motion.translation.swap(0, 1);
        assert!(!motion.valid());
        motion.translation.swap(0, 1);
        motion.translation[1][1] = f32::NAN;
        assert!(!motion.valid());
        motion.translation[1][1] = 1.;
        motion.translation[2][0] = 0.75;
        assert!(!motion.valid());
        motion.translation.clear();
        motion.rotation = vec![[0.; 5], [1., 0., 0., 0., 0.]];
        assert!(!motion.valid());
    }

    #[test]
    fn native_zero_and_descending_recoil_tuning_keep_active_value_checks() {
        let mut content: ContentCatalog = serde_json::from_value(serde_json::json!({
            "weapons":[{"id":"weapon.fixture","name":"fixture","damage":1,"category":"melee",
                "magazine":1,"reserve":0,"fire_interval":0.,"reload_seconds":0.,"upgraded":null,"presentation":null,
                "mechanics":{"delivery":"melee","starting_reserve":0,"pellets":0,"ads_enabled":false,
                    "ads_fov":0.,"ads_seconds":[0.,0.],"movement":[0.,0.],"recoil_recovery":0.,
                    "empty_reload":0.,"shell_seconds":0.,"equip_seconds":0.,"unequip_seconds":0.,"projectile_lifetime":0.}}],
            "enemies":[],"perks":[],"zones":[],"definitions":[],"placements":[],"interactions":[],
            "zone_connections":[],"navigation_nodes":[],"navigation_links":[],"presentations":[],
            "sounds":[],"animations":[],"ui_assets":[]
        })).unwrap();
        content.validate().unwrap();
        content.weapons[0].mechanics.recoil_yaw = [35., -35.];
        content.weapons[0].mechanics.recoil_pitch = [0.4, 0.2];
        content.validate().unwrap();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            content.weapons[0].mechanics.recoil_yaw[0] = invalid;
            assert!(content.validate().is_err());
            content.weapons[0].mechanics.recoil_yaw[0] = 35.;
            content.weapons[0].mechanics.recoil_pitch[1] = invalid;
            assert!(content.validate().is_err());
            content.weapons[0].mechanics.recoil_pitch[1] = 0.2;
        }
        content.weapons[0].mechanics.delivery = WeaponDelivery::Hitscan;
        assert!(content.validate().is_err());
        content.weapons[0].fire_interval = 0.2;
        content.weapons[0].reload_seconds = 1.;
        content.weapons[0].mechanics.pellets = 1;
        content.weapons[0].mechanics.movement[0] = 1.;
        content.weapons[0].mechanics.empty_reload = 1.;
        content.validate().unwrap();
        content.weapons[0].mechanics.delivery = WeaponDelivery::Projectile;
        assert!(content.validate().is_err());
        content.weapons[0].mechanics.projectile_lifetime = 2.;
        content.validate().unwrap();
        content.weapons[0].mechanics.recoil_recovery = f32::NAN;
        assert!(content.validate().is_err());
    }
}

pub fn validate_asset_path(path: &str) -> Result<(), ContentError> {
    let base = path.split('#').next().unwrap_or("");
    valid(
        !base.is_empty()
            && !base.contains([':', '\\', '%'])
            && base.split('/').all(|p| {
                !p.is_empty() && p != "." && p != ".." && !p.chars().any(char::is_control)
            }),
        "Unsafe relative runtime asset path",
    )
}
impl ContentCatalog {
    pub fn validate_first_playable(&self) -> Result<(), ContentError> {
        for name in ["weapon.c96_zm", "weapon.knife_zm", "weapon.frag_grenade_zm"] {
            let weapon = self
                .weapons
                .iter()
                .find(|weapon| weapon.id.0 == name)
                .ok_or_else(|| ContentError::MissingReference {
                    owner: "first_playable_solo".into(),
                    target: name.into(),
                })?;
            valid(
                weapon.presentation.is_some()
                    && (name == "weapon.knife_zm" || !weapon.animation_actions.is_empty()),
                "First playable weapon lacks its source presentation or animations",
            )?;
        }
        valid(
            !self.enemies.is_empty()
                && !self.barriers.is_empty()
                && self.barriers.iter().all(|barrier| barrier.enter.is_some())
                && self
                    .zones
                    .iter()
                    .any(|zone| zone.id.0 == "zone.start" && zone.initially_open)
                && !self.navigation_nodes.is_empty()
                && self.navigation_nodes.iter().all(|node| node.native.is_some())
                && !self.navigation_links.is_empty()
                && self.navigation_links.iter().all(|link| link.cost.is_some())
                && !self.presentations.is_empty()
                && !self.animations.is_empty()
                && !self.ui_assets.is_empty()
                && !self.localized_strings.is_empty(),
            "First playable solo lacks required enemy, barrier, zone, navigation, animation, or HUD data",
        )?;
        Ok(())
    }

    pub fn index(&self) -> Result<ContentIndex<'_>, ContentError> {
        let mut index = ContentIndex(BTreeMap::new());
        macro_rules! add {
            ($field:ident, $kind:literal) => {
                for v in &self.$field {
                    index.insert(&v.id.0, $kind)?;
                }
            };
        }
        for id in &self.definitions {
            index.insert(&id.0, "definition")?;
        }
        add!(weapons, "weapon");
        add!(barriers, "barrier");
        add!(enemies, "enemy");
        add!(perks, "perk");
        add!(powerups, "powerup");
        add!(zones, "zone");
        add!(placements, "placement");
        add!(interactions, "interaction");
        add!(zone_connections, "connection");
        add!(navigation_nodes, "node");
        add!(navigation_links, "link");
        add!(generators, "generator");
        add!(robot_routes, "route");
        add!(tank_routes, "route");
        add!(weather, "weather");
        add!(digging, "dig");
        add!(recipes, "recipe");
        add!(staff_puzzles, "puzzle");
        add!(quests, "quest");
        add!(quest_transitions, "transition");
        add!(presentations, "presentation");
        add!(sounds, "sound");
        add!(animations, "animation");
        add!(ui_assets, "ui");
        add!(cinematics, "cinematic");
        add!(localized_strings, "string");
        Ok(index)
    }
    pub fn validate(&self) -> Result<(), ContentError> {
        let index = self.index()?;
        let r = |owner: &str, target: &str, kind: &str| index.resolve(owner, target, kind);
        let optional =
            |owner: &str, target: &Option<ContentId>, kind: &str| -> Result<(), ContentError> {
                if let Some(id) = target {
                    r(owner, &id.0, kind)?;
                }
                Ok(())
            };
        for v in &self.weapons {
            valid(
                v.native_view_model_index.is_none_or(|index| index < 16)
                    && v.attachments.values().all(|attachment| {
                        attachment.native_selectors.is_none_or(|selectors| {
                            selectors.into_iter().all(|selector| selector < 64)
                        })
                    }),
                "Invalid native weapon material selectors",
            )?;
            let m = &v.mechanics;
            valid(
                m.reload_transfer_seconds.is_none_or(|times| times.into_iter().all(nonnegative)),
                "Invalid native reload transfer timing",
            )?;
            let uses_ammo = matches!(
                m.delivery,
                WeaponDelivery::Hitscan | WeaponDelivery::Projectile
            );
            if v.id.0 == "weapon.frag_grenade_zm" {
                valid(v.throwable.is_some(), "Missing starting grenade tuning")?;
            }
            if let Some(throwable) = &v.throwable {
                valid(throwable.valid(), "Invalid throwable tuning")?;
            }
            valid(
                v.view_offset.iter().all(|v| v.is_finite() && v.abs() < 2.),
                "Invalid weapon view offset",
            )?;
            if v.throwable.is_some() || v.mine.is_some() {
                valid(
                    [
                        WeaponAnimation::Prime,
                        WeaponAnimation::Idle,
                        WeaponAnimation::Fire,
                    ]
                    .iter()
                    .all(|action| v.animation_actions.contains_key(action)),
                    "Missing equipment deployment animation",
                )?;
            }
            if let Some(mine) = &v.mine {
                valid(mine.valid(), "Invalid mine tuning")?;
                valid(
                    self.presentations
                        .iter()
                        .any(|p| p.id == mine.presentation && p.scene.is_some()),
                    "Missing mine presentation",
                )?;
            }
            if let Some(lure) = &v.lure {
                valid(lure.valid(), "Invalid lure tuning")?;
                valid(
                    v.throwable
                        .as_ref()
                        .is_some_and(|t| t.slot == crate::equipment::ThrowableSlot::Tactical),
                    "Lure requires tactical throwable",
                )?;
                valid(
                    self.animations
                        .iter()
                        .any(|a| a.id == lure.animation && v.pickup.as_ref() == Some(&a.skeleton)),
                    "Missing lure animation",
                )?;
            }
            valid(
                m.ads_translation
                    .iter()
                    .chain(m.ads_rotation.iter())
                    .all(|v| v.is_finite())
                    && (m.ads_rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
                    && nonnegative(m.burst_delay),
                "Invalid ADS pose or burst delay",
            )?;
            valid(
                (!uses_ammo || (v.damage > 0 && v.magazine > 0))
                    && v.hud.as_ref().is_none_or(|hud| {
                        [hud.center_size, hud.side_size, hud.minimum_offset]
                            .iter()
                            .all(|value| value.is_finite() && *value >= 0. && *value <= 512.)
                            && hud.side_position.is_finite()
                            && hud.side_position.abs() <= 8.
                            && hud.low_ammo_fraction.is_finite()
                            && (0. ..=1.).contains(&hud.low_ammo_fraction)
                            && [&hud.center, &hud.side]
                                .into_iter()
                                .flatten()
                                .all(|id| self.ui_assets.iter().any(|asset| asset.id == *id))
                    })
                    && nonnegative(v.fire_interval)
                    && (!uses_ammo || positive(v.fire_interval))
                    && nonnegative(v.reload_seconds)
                    && (!uses_ammo || positive(v.reload_seconds))
                    && m.starting_reserve <= v.reserve
                    && (!uses_ammo || m.pellets > 0)
                    && m.pellets <= 64
                    && m.hit_multipliers.iter().all(|v| nonnegative(*v))
                    && m.spread.iter().all(|v| nonnegative(*v))
                    && m.penetration
                        .as_ref()
                        .is_none_or(|depths| depths.iter().all(|v| nonnegative(*v)))
                    && m.damage_range.iter().all(|v| nonnegative(*v))
                    && m.damage_range[0] <= m.damage_range[1]
                    && positive(m.range)
                    && m.melee_extent.iter().all(|value| nonnegative(*value))
                    && m.recoil_pitch
                        .iter()
                        .chain(m.recoil_yaw.iter())
                        .all(|v| v.is_finite())
                    && nonnegative(m.recoil_recovery)
                    && m.ads_recoil_recovery.is_none_or(nonnegative)
                    && m.ads_fov.is_finite()
                    && (0. ..=179.).contains(&m.ads_fov)
                    && (!m.ads_enabled || m.ads_fov >= 1.)
                    && m.ads_zoom_fraction
                        .iter()
                        .all(|v| v.is_finite() && (0. ..=1.).contains(v))
                    && (m.ads_pose.is_empty()
                        || (m.ads_pose.len() >= 2
                            && m.ads_pose.len() <= 256
                            && m.ads_pose.first().is_some_and(|p| p.fraction == 0.)
                            && m.ads_pose.last().is_some_and(|p| p.fraction == 1.)
                            && m.ads_pose.windows(2).all(|p| p[0].fraction < p[1].fraction)))
                    && m.ads_pose.iter().all(|p| {
                        p.fraction.is_finite()
                            && p.translation
                                .iter()
                                .chain(p.rotation.iter())
                                .all(|v| v.is_finite())
                            && (p.rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
                    })
                    && m.native_ads_pitch.is_none_or(|pitch| pitch.is_finite())
                    && m.ads_sight.as_ref().is_none_or(|s| {
                        matches!(s.joint.as_str(), "j_gun" | "tag_sights")
                            && s.point
                                .iter()
                                .chain(s.rotation.iter())
                                .all(|v| v.is_finite())
                            && (s.rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
                            && s.rotation[3].abs() >= (1_f32.to_radians()).cos()
                    })
                    && m.ads_seconds.iter().all(|v| nonnegative(*v))
                    && (!m.ads_enabled || m.ads_seconds.iter().all(|v| positive(*v)))
                    && m.movement.iter().all(|v| nonnegative(*v))
                    && (!uses_ammo || positive(m.movement[0]))
                    && (!m.ads_enabled || positive(m.movement[1]))
                    && [
                        m.empty_reload,
                        m.shell_seconds,
                        m.equip_seconds,
                        m.unequip_seconds,
                        m.projectile_lifetime,
                    ]
                    .into_iter()
                    .all(nonnegative)
                    && (!uses_ammo || positive(m.empty_reload))
                    && (!m.shell_reload || positive(m.shell_seconds))
                    && (m.delivery != WeaponDelivery::Projectile
                        || positive(m.projectile_lifetime))
                    && [
                        m.projectile_speed,
                        m.explosion_radius,
                        m.fuse_seconds,
                        m.reload_start_seconds,
                        m.reload_start_insert,
                        m.reload_end_seconds,
                    ]
                    .into_iter()
                    .all(nonnegative)
                    && m.melee_seconds.iter().all(|v| nonnegative(*v))
                    && m.sprint_seconds.iter().all(|v| nonnegative(*v))
                    && m.crawl_seconds.iter().all(|v| nonnegative(*v))
                    && m.dive_seconds.iter().all(|v| nonnegative(*v)),
                "Invalid weapon tuning",
            )?;
            if let FirePattern::Burst { shots } = m.fire {
                valid((1..=8).contains(&shots), "Invalid burst count")?;
            }
            if let Some(id) = &v.upgraded {
                r(&v.id.0, &id.0, "weapon")?;
            }
            if let Some(id) = &v.dual_weapon {
                r(&v.id.0, &id.0, "weapon")?;
                let partner = self.weapons.iter().find(|w| w.id == *id).unwrap();
                valid(
                    m.dual_wield
                        && partner.magazine == v.magazine
                        && partner.ammo_group == v.ammo_group,
                    "Invalid dual weapon pairing",
                )?;
            }
            optional(&v.id.0, &v.presentation, "presentation")?;
            optional(&v.id.0, &v.ads_presentation, "presentation")?;
            optional(&v.id.0, &v.pickup, "presentation")?;
            optional(&v.id.0, &v.ammo_group, "definition")?;
            for id in v.animation_actions.values() {
                r(&v.id.0, &id.0, "animation")?;
            }
        }
        for v in &self.enemies {
            valid(
                v.health > 0
                    && v.damage > 0
                    && positive(v.attack_seconds)
                    && positive(v.death_seconds)
                    && positive(v.bounds_margin)
                    && v.physics.is_none_or(EnemyPhysics::valid)
                    && v.ragdoll.as_ref().is_none_or(EnemyRagdoll::valid)
                    && v.attack_hits
                        .iter()
                        .all(|t| nonnegative(*t) && *t <= v.attack_seconds)
                    && v.attack_hits.windows(2).all(|w| w[0] < w[1]),
                "Invalid enemy tuning",
            )?;
            optional(&v.id.0, &v.presentation, "presentation")?;
            let mut hit_ids = std::collections::BTreeSet::new();
            valid(
                v.character_index.is_none_or(|index| {
                    index < 6 && !v.archetype.is_empty() && !v.character_group.is_empty()
                }),
                "Invalid enemy source character selection",
            )?;
            for hit in &v.hit_regions {
                valid(
                    !hit.id.is_empty()
                        && hit
                            .id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                        && hit_ids.insert(&hit.id)
                        && !hit.joint.is_empty()
                        && hit.bounds.iter().flatten().all(|v| v.is_finite())
                        && (0..3).all(|axis| hit.bounds[0][axis] < hit.bounds[1][axis])
                        && hit.sphere.iter().all(|v| v.is_finite())
                        && hit.sphere[3] >= 4. * 0.0254 * 0.0254
                        && (v.presentation.as_ref() == Some(&hit.presentation)
                            || v.parts.iter().any(|p| p.presentation == hit.presentation)
                            || v.gibs.as_ref().is_some_and(|gibs| {
                                gibs.presentations().any(|id| *id == hit.presentation)
                            })),
                    "Invalid enemy hit region",
                )?;
                index.resolve(&v.id.0, &hit.presentation.0, "presentation")?;
            }
            if let Some(gibs) = &v.gibs {
                for id in gibs.presentations() {
                    r(&v.id.0, &id.0, "presentation")?;
                }
                valid(!gibs.models.is_empty(), "Empty native gib replacements")?;
                for piece in gibs.pieces.iter().chain(gibs.hat.iter()) {
                    r(&v.id.0, &piece.presentation.0, "presentation")?;
                    valid(
                        !piece.joint.is_empty() && piece.physics.valid(),
                        "Invalid native gib physics",
                    )?;
                }
            }
            for part in &v.parts {
                r(&v.id.0, &part.presentation.0, "presentation")?;
                valid(
                    !part.socket.is_empty()
                        && part.root_joint.as_ref().is_none_or(|n| !n.is_empty()),
                    "Empty actor attachment joint",
                )?;
            }
            for clip in v.animation_actions.values() {
                r(&v.id.0, &clip.0, "animation")?;
            }
            for clips in v.locomotion_states.values() {
                valid(
                    !clips.is_empty() && clips.len() <= 32,
                    "Invalid locomotion substates",
                )?;
                for clip in clips {
                    r(&v.id.0, &clip.0, "animation")?;
                    valid(
                        self.animations
                            .iter()
                            .any(|a| a.id == *clip && v.presentation.as_ref() == Some(&a.skeleton)),
                        "Locomotion animation belongs to another rig",
                    )?;
                }
            }
            for clips in v.combat_states.values() {
                valid(
                    !clips.is_empty() && clips.len() <= 32,
                    "Invalid combat substates",
                )?;
                for clip in clips {
                    r(&v.id.0, &clip.animation.0, "animation")?;
                    valid(
                        positive(clip.seconds)
                            && clip.duration.is_none_or(|duration| {
                                positive(duration) && duration >= clip.seconds
                            })
                            && clip
                                .hits
                                .iter()
                                .all(|t| nonnegative(*t) && *t <= clip.seconds)
                            && clip.hits.windows(2).all(|p| p[0] < p[1])
                            && clip
                                .stops
                                .iter()
                                .all(|time| nonnegative(*time) && *time <= clip.seconds)
                            && clip.stops.windows(2).all(|pair| pair[0] < pair[1])
                            && self.animations.iter().any(|a| {
                                a.id == clip.animation
                                    && v.presentation.as_ref() == Some(&a.skeleton)
                            }),
                        "Invalid combat clip or timing",
                    )?;
                }
            }
            for (alias, traversals) in &v.traversals {
                valid(
                    !alias.is_empty()
                        && alias.len() < 128
                        && !traversals.is_empty()
                        && traversals.len() <= 64,
                    "Invalid enemy traversal profile",
                )?;
                for traversal in traversals {
                    valid(traversal.valid(), "Invalid enemy traversal motion")?;
                    r(&v.id.0, &traversal.animation.0, "animation")?;
                }
            }
        }
        for v in &self.perks {
            optional(&v.id.0, &v.off_presentation, "presentation")?;
            valid(
                v.path_bounds_radius.is_none_or(positive),
                "Invalid perk source path bounds",
            )?;
            valid(
                v.bump_trigger
                    .as_ref()
                    .is_none_or(|id| !id.is_empty() && id.len() < 128),
                "Invalid perk bump trigger",
            )?;
            valid(
                v.collision.iter().all(|shape| {
                    !shape.id.is_empty()
                        && shape.position.iter().all(|value| value.is_finite())
                        && (4..=128).contains(&shape.vertices.len())
                        && shape
                            .vertices
                            .iter()
                            .flatten()
                            .all(|value| value.is_finite())
                        && avian3d::prelude::Collider::convex_hull(
                            &shape
                                .vertices
                                .iter()
                                .copied()
                                .map(bevy::prelude::Vec3::from_array)
                                .collect::<Vec<_>>(),
                        )
                        .is_some()
                }),
                "Invalid perk source collision",
            )?;
            if let Some(generator) = &v.generator {
                r(&v.id.0, &generator.0, "generator")?;
            }
            valid(
                v.dive.as_ref().is_none_or(|d| d.valid()),
                "Invalid dive modifiers",
            )?;
            valid(
                v.cherry.as_ref().is_none_or(|c| c.valid()),
                "Invalid Electric Cherry modifiers",
            )?;
            valid(
                v.firing.as_ref().is_none_or(|p| {
                    [p.interval_scale, p.hip_spread_scale]
                        .into_iter()
                        .all(|v| v.is_finite() && v > 0. && v <= 1.)
                        && (1..=2).contains(&p.bullet_copies)
                }),
                "Invalid perk firing modifiers",
            )?;
            valid(
                v.sprint.as_ref().is_none_or(|p| {
                    [p.duration_scale, p.speed_scale]
                        .into_iter()
                        .all(|v| v.is_finite() && (1. ..=4.).contains(&v))
                }),
                "Invalid perk sprint modifiers",
            )?;
            valid(
                v.reload_duration_scale
                    .is_none_or(|v| v.is_finite() && v > 0. && v <= 1.),
                "Invalid perk reload scale",
            )?;
            valid(
                v.maximum_health
                    .is_none_or(|h| h > 0 && h <= i32::MAX as u32),
                "Invalid perk maximum health",
            )?;
            optional(&v.id.0, &v.icon, "ui")?;
            optional(&v.id.0, &v.presentation, "presentation")?;
            if let Some(drink) = &v.drink {
                r(&v.id.0, &drink.bottle.0, "weapon")?;
                valid(
                    positive(drink.seconds)
                        && positive(drink.lower_seconds)
                        && nonnegative(drink.activation_delay)
                        && drink.activation_delay <= 0.5,
                    "Invalid perk drinking timing",
                )?;
            }
        }
        for v in &self.powerups {
            valid(nonnegative(v.duration), "Invalid powerup duration")?;
            optional(&v.id.0, &v.presentation, "presentation")?;
        }
        for v in &self.placements {
            r(&v.id.0, &v.definition.0, "any")?;
            r(&v.id.0, &v.zone.0, "zone")?;
            valid(
                v.position.iter().all(|x| x.is_finite()) && v.yaw.is_finite(),
                "Invalid placement transform",
            )?;
        }
        for v in &self.interactions {
            valid(
                v.source_trigger
                    .as_ref()
                    .is_none_or(|id| !id.is_empty() && id.len() < 128),
                "Invalid source interaction trigger",
            )?;
            if let Some(generator) = &v.generator {
                r(&v.id.0, &generator.0, "generator")?;
            }
            r(&v.id.0, &v.placement.0, "placement")?;
            valid(
                positive(v.reach) && nonnegative(v.hold_seconds),
                "Invalid interaction timing/reach",
            )?;
            match &v.action {
                InteractionAction::Barrier { barrier } => r(&v.id.0, &barrier.0, "barrier")?,
                InteractionAction::Door { connection, .. } => {
                    r(&v.id.0, &connection.0, "connection")?
                }
                InteractionAction::Weapon { weapon, .. } => r(&v.id.0, &weapon.0, "weapon")?,
                InteractionAction::Perk { perk } => {
                    r(&v.id.0, &perk.0, "perk")?;
                    valid(
                        self.perks.iter().any(|definition| {
                            definition.id == *perk
                                && (definition.generator.is_some() || perk.0 == "perk.mule_kick")
                        }),
                        "Perk machine lacks source-defined power availability",
                    )?;
                }
                InteractionAction::Generator { generator } => {
                    r(&v.id.0, &generator.0, "generator")?
                }
                InteractionAction::Quest { quest, .. } => r(&v.id.0, &quest.0, "quest")?,
                InteractionAction::Pickup { item } => r(&v.id.0, &item.0, "any")?,
            }
        }
        for v in &self.zone_connections {
            r(&v.id.0, &v.from.0, "zone")?;
            r(&v.id.0, &v.to.0, "zone")?;
        }
        for v in &self.barriers {
            r(&v.id.0, &v.placement.0, "placement")?;
            r(&v.id.0, &v.entry.0, "node")?;
            r(&v.id.0, &v.exit.0, "node")?;
            valid(
                v.audio
                    .repair
                    .iter()
                    .flatten()
                    .all(|alias| !alias.is_empty())
                    && v.audio.repair_delay.is_none_or(nonnegative),
                "Invalid native barrier repair audio",
            )?;
            valid(v.repair_quake.as_ref().is_none_or(|quake| quake.valid()), "Invalid native barrier repair earthquake")?;
            valid(v.valid_actions(), "Invalid native barrier actor action")?;
            valid(v.boards.len() == 6, "Invalid barrier board count")?;
            valid(
                v.repair_origin.iter().all(|value| value.is_finite())
                    && v.repair_yaw.is_finite()
                    && positive(v.attack_spacing)
                    && v.attack_yaw.is_finite()
                    && positive(v.repair_radius)
                    && positive(v.repair_height)
                    && positive(v.repair_interval)
                    && v.repair_points > 0,
                "Invalid source barrier repair trigger",
            )?;
            for b in &v.boards {
                r(&v.id.0, &b.presentation.0, "presentation")?;
                r(&v.id.0, &b.repair.0, "animation")?;
                r(&v.id.0, &b.pull.0, "animation")?;
                valid(
                    positive(b.repair_seconds)
                        && positive(b.pull_seconds)
                        && !b.collision.is_empty()
                        && b.collider().is_some(),
                    "Invalid source board collision/timing",
                )?;
                valid(
                    b.audio
                        .opening
                        .iter()
                        .chain(&b.audio.hover)
                        .all(|alias| !alias.is_empty())
                        && b.audio.repeat_pause.is_none_or(|[min, max]| {
                            nonnegative(min) && max.is_finite() && max >= min
                        })
                        && b.audio.effect_delay.iter().all(|value| nonnegative(*value))
                        && b.audio.effect_delay[1] >= b.audio.effect_delay[0]
                        && b.repair_effects.iter().flatten().all(|effect| !effect.name.is_empty() && effect.offset.iter().all(|value| value.is_finite())),
                    "Invalid native barrier board audio",
                )?;
                for attack in &b.attacks {
                    valid(
                        positive(attack.grab_seconds)
                            && positive(attack.tear_seconds)
                            && nonnegative(attack.destroy_seconds)
                            && attack.destroy_seconds < attack.tear_seconds
                            && !attack.grab_clip.is_empty()
                            && !attack.tear_clip.is_empty()
                            && attack.valid_motion(),
                        "Invalid native barrier attack slot",
                    )?;
                }
            }
        }
        for v in &self.navigation_nodes {
            r(&v.id.0, &v.zone.0, "zone")?;
            valid(
                v.position.iter().all(|x| x.is_finite())
                    && v.native.is_none_or(|native| native.r#type <= 21),
                "Invalid navigation position",
            )?;
        }
        for v in &self.navigation_links {
            r(&v.id.0, &v.from.0, "node")?;
            r(&v.id.0, &v.to.0, "node")?;
            optional(&v.id.0, &v.connection, "connection")?;
            valid(v.cost.is_none_or(positive), "Invalid navigation link cost")?;
            for id in &v.blockers {
                r(&v.id.0, &id.0, "connection")?;
            }
            valid(
                v.source_animation
                    .as_ref()
                    .is_none_or(|s| !s.is_empty() && s.len() < 128),
                "Invalid traversal source animation",
            )?;
            if let Some(native) = &v.negotiation {
                valid(
                    matches!(v.traversal, Traversal::Climb | Traversal::Jump)
                        && v.source_animation.is_some()
                        && positive(native.cost)
                        && v.cost.is_none_or(|cost| cost == native.cost)
                        && native.start.valid(17, &v.from, &self.navigation_nodes)
                        && native.end.valid(18, &v.to, &self.navigation_nodes),
                    "Invalid native negotiation endpoint or cost",
                )?;
            }
        }
        for v in &self.generators {
            r(&v.id.0, &v.placement.0, "placement")?;
            r(&v.id.0, &v.presentation.0, "presentation")?;
            r(&v.id.0, &v.capture_sound.0, "sound")?;
            r(&v.id.0, &v.active_sound.0, "sound")?;
            r(&v.id.0, &v.capture_enemy.0, "enemy")?;
            if let Some(reward)=&v.initial_reward {
                r(&v.id.0,&reward.presentation.0,"presentation")?;
                r(&v.id.0,&reward.powerup.0,"powerup")?;
                valid(reward.position.iter().all(|value|value.is_finite()) && reward.yaw.is_finite() && !reward.source_trigger.is_empty() && reward.source_trigger.len()<128,"Invalid initial generator reward")?;
            }
            for z in &v.zones {
                r(&v.id.0, &z.0, "zone")?;
            }
            valid(
                positive(v.capture_seconds)
                    && positive(v.radius)
                    && (1..=6).contains(&v.number)
                    && positive(v.decay_seconds)
                    && positive(v.shutdown_seconds)
                    && positive(v.recovery_seconds)
                    && v.model_position.iter().all(|v| v.is_finite())
                    && !v.emergence.is_empty()
                    && v.emergence.len() <= 128
                    && v.emergence.iter().all(|spot| {
                        spot.position.iter().all(|value| value.is_finite()) && spot.yaw.is_finite()
                    })
                    && v.attack_points.len() == 12
                    && v.attack_points.iter().all(|point| {
                        point
                            .position
                            .iter()
                            .chain(&point.face)
                            .all(|value| value.is_finite())
                    })
                    && v.clip_seconds.iter().all(|v| positive(*v)),
                "Invalid generator parameters",
            )?;
        }
        for v in self.robot_routes.iter().chain(&self.tank_routes) {
            valid(
                v.points.len() >= 2
                    && v.points.iter().flatten().all(|x| x.is_finite())
                    && positive(v.speed)
                    && nonnegative(v.cooldown),
                "Invalid transport route",
            )?;
            optional(&v.id.0, &v.presentation, "presentation")?;
        }
        for v in &self.weather {
            valid(
                positive(v.duration[0])
                    && v.duration[1].is_finite()
                    && v.duration[1] >= v.duration[0],
                "Invalid weather duration",
            )?;
            optional(&v.id.0, &v.sound, "sound")?;
        }
        for v in &self.digging {
            r(&v.id.0, &v.placement.0, "placement")?;
            for id in &v.loot {
                r(&v.id.0, &id.0, "any")?;
            }
            valid(!v.loot.is_empty(), "Dig table has no loot")?;
        }
        for v in &self.recipes {
            r(&v.id.0, &v.bench.0, "placement")?;
            r(&v.id.0, &v.result.0, "any")?;
            for id in &v.parts {
                r(&v.id.0, &id.0, "any")?;
            }
            valid(!v.parts.is_empty(), "Recipe has no parts")?;
        }
        for v in &self.staff_puzzles {
            r(&v.id.0, &v.weapon.0, "weapon")?;
            for id in &v.interactions {
                r(&v.id.0, &id.0, "interaction")?;
            }
            valid(
                !v.solution.is_empty() && !v.interactions.is_empty(),
                "Puzzle has no solution or targets",
            )?;
        }
        for v in &self.quest_transitions {
            r(&v.id.0, &v.quest.0, "quest")?;
            for id in &v.prerequisites {
                r(&v.id.0, &id.0, "any")?;
            }
            valid(
                !v.from.is_empty() && !v.to.is_empty() && !v.event.is_empty(),
                "Invalid quest transition",
            )?;
        }
        for v in &self.presentations {
            if let Some(path) = &v.scene {
                validate_asset_path(path)?;
            }
            for id in &v.animations {
                r(&v.id.0, &id.0, "animation")?;
            }
            for id in &v.sounds {
                r(&v.id.0, &id.0, "sound")?;
            }
        }
        for v in &self.sounds {
            validate_asset_path(&v.path)?;
            valid(
                nonnegative(v.volume)
                    && positive(v.pitch[0])
                    && v.pitch[1].is_finite()
                    && v.pitch[1] >= v.pitch[0]
                    && nonnegative(v.distance)
                    && nonnegative(v.distance_min)
                    && v.distance_min <= v.distance,
                "Invalid sound tuning",
            )?;
        }
        for v in &self.animations {
            validate_asset_path(&v.path)?;
            r(&v.id.0, &v.skeleton.0, "presentation")?;
            valid(positive(v.seconds), "Invalid native animation duration")?;
            valid(
                v.root_motion
                    .as_ref()
                    .is_none_or(AnimationRootMotion::valid),
                "Invalid native animation root motion",
            )?;
            valid(
                v.notifications.iter().all(|n| {
                    nonnegative(n.seconds) && n.seconds <= v.seconds + 0.001 && !n.event.is_empty()
                }) && v
                    .notifications
                    .windows(2)
                    .all(|w| w[0].seconds <= w[1].seconds),
                "Invalid animation notifications",
            )?;
        }
        for v in &self.ui_assets {
            validate_asset_path(&v.path)?;
            if let Some(font) = &v.font {
                let mut letters = BTreeSet::new();
                valid(
                    positive(font.height)
                        && font.atlas.iter().all(|v| positive(*v))
                        && !font.glyphs.is_empty()
                        && font.glyphs.iter().all(|g| {
                            char::from_u32(g.codepoint).is_some()
                                && letters.insert(g.codepoint)
                                && g.offset.iter().all(|v| v.is_finite())
                                && nonnegative(g.advance)
                                && g.size.iter().all(|v| nonnegative(*v))
                                && g.rect.iter().all(|v| nonnegative(*v))
                                && g.rect[0] <= g.rect[2]
                                && g.rect[1] <= g.rect[3]
                                && g.rect[2] <= font.atlas[0]
                                && g.rect[3] <= font.atlas[1]
                        }),
                    "Invalid bitmap font metrics",
                )?;
            }
        }
        for v in &self.localized_strings {
            valid(!v.key.is_empty(), "Empty localization key")?;
        }
        for v in &self.cinematics {
            valid(
                !v.frames.is_empty() && positive(v.frames_per_second),
                "Invalid cinematic sequence",
            )?;
            for path in &v.frames {
                validate_asset_path(path)?;
            }
            optional(&v.id.0, &v.audio, "sound")?;
        }
        Ok(())
    }
}
