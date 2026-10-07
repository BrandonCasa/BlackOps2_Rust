//! Versioned offline content contract. IDs remain independent of ECS entities.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

macro_rules! identifiers {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
    )+};
}
identifiers!(
    ContentId,
    PlacementId,
    ZoneId,
    WeaponId,
    PerkId,
    QuestId,
    InteractionId
);

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
                "Cannot read {}: {source}. Restore the runtime pack.",
                path.display()
            ),
            Self::Json { path, source } => write!(
                f,
                "Invalid {}: {source}. Regenerate/import game.json.",
                path.display()
            ),
            Self::UnsupportedVersion(v) => write!(
                f,
                "Unsupported content version {v}; expected 2. Regenerate/import the runtime pack."
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
    pub throwable: Option<crate::equipment::ThrowableDefinition>,
    #[serde(default)]
    pub mine: Option<crate::mines::MineDefinition>,
    #[serde(default)]
    pub lure: Option<crate::lures::LureDefinition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponAnimation {
    Idle,
    IdleEmpty,
    LastShot,
    Rechamber,
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
    ProneLeave,
    DiveEnter,
    DiveLoop,
    DiveLeave,
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
    pub hit_multipliers: [f32; 5],
    pub pellets: u32,
    pub penetration: f32,
    pub recoil_pitch: [f32; 2],
    pub recoil_yaw: [f32; 2],
    pub recoil_recovery: f32,
    pub spread: [f32; 4],
    pub ads_fov: f32,
    pub ads_seconds: [f32; 2],
    pub ads_zoom_fraction: [f32; 2],
    pub ads_pose: Vec<AdsPoseSample>,
    pub ads_sight: Option<AdsSight>,
    pub ads_enabled: bool,
    pub ads_translation: [f32; 3],
    pub ads_rotation: [f32; 4],
    pub burst_delay: f32,
    pub movement: [f32; 2],
    pub empty_reload: f32,
    pub shell_reload: bool,
    pub shell_seconds: f32,
    pub reload_start_seconds: f32,
    pub reload_start_insert: f32,
    pub reload_end_seconds: f32,
    pub equip_seconds: f32,
    pub unequip_seconds: f32,
    pub dual_wield: bool,
    pub projectile_speed: f32,
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
            hit_multipliers: [3.5, 1.25, 1., 0.85, 0.6],
            pellets: 1,
            penetration: 0.2,
            recoil_pitch: [0.2, 0.4],
            recoil_yaw: [-0.1, 0.1],
            recoil_recovery: 8.,
            spread: [3., 2., 1., 0.],
            ads_fov: 50.,
            ads_seconds: [0.2, 0.2],
            ads_zoom_fraction: [1., 1.],
            ads_pose: Vec::new(),
            ads_sight: None,
            ads_enabled: true,
            ads_translation: [0.; 3],
            ads_rotation: [0., 0., 0., 1.],
            burst_delay: 0.,
            movement: [1., 0.5],
            empty_reload: 1.8,
            shell_reload: false,
            shell_seconds: 0.5,
            reload_start_seconds: 0.,
            reload_start_insert: 0.,
            reload_end_seconds: 0.,
            equip_seconds: 0.5,
            unequip_seconds: 0.3,
            dual_wield: false,
            projectile_speed: 30.,
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
    pub speed: f32,
    pub damage: u32,
    pub presentation: Option<ContentId>,
    #[serde(default)]
    pub parts: Vec<ActorPart>,
    #[serde(default)]
    pub animation_actions: BTreeMap<EnemyAnimation, ContentId>,
    #[serde(default)]
    pub traversals: BTreeMap<String, EnemyTraversalDefinition>,
    #[serde(default = "default_attack_seconds")]
    pub attack_seconds: f32,
    #[serde(default)]
    pub attack_hits: Vec<f32>,
    #[serde(default = "default_death_seconds")]
    pub death_seconds: f32,
    #[serde(default = "default_corpse_seconds")]
    pub corpse_seconds: f32,
    #[serde(default = "default_bounds_margin")]
    pub bounds_margin: f32,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub hit_regions: Vec<HitVolumeDefinition>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitVolumeDefinition {
    pub id: String,
    pub region: crate::weapons::HitRegion,
    pub presentation: ContentId,
    pub start: String,
    pub end: String,
    pub radius: f32,
}
fn default_attack_seconds() -> f32 {
    1.8
}
fn default_death_seconds() -> f32 {
    3.
}
fn default_corpse_seconds() -> f32 {
    8.
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
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkDefinition {
    pub id: PerkId,
    pub name: String,
    pub price: u32,
    pub icon: Option<ContentId>,
    #[serde(default)]
    pub presentation: Option<ContentId>,
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
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationLink {
    pub id: ContentId,
    pub from: ContentId,
    pub to: ContentId,
    pub traversal: Traversal,
    pub connection: Option<ContentId>,
    #[serde(default)]
    pub blockers: Vec<ContentId>,
    #[serde(default)]
    pub source_animation: Option<String>,
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
    pub emergence: Vec<[f32; 3]>,
    pub clip_seconds: [f32; 8],
    pub capture_sound: ContentId,
    pub active_sound: ContentId,
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
    pub notifications: Vec<AnimationNotification>,
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
    pub powerups: Vec<PowerupDefinition>,
    pub zones: Vec<ZoneDefinition>,
    pub definitions: Vec<ContentId>,
    pub placements: Vec<Placement>,
    pub interactions: Vec<InteractionPlacement>,
    pub zone_connections: Vec<ZoneConnection>,
    pub navigation_nodes: Vec<NavigationNode>,
    pub navigation_links: Vec<NavigationLink>,
    pub generators: Vec<GeneratorDefinition>,
    pub robot_routes: Vec<RouteDefinition>,
    pub tank_routes: Vec<RouteDefinition>,
    pub weather: Vec<WeatherDefinition>,
    pub digging: Vec<DigDefinition>,
    pub recipes: Vec<CraftingRecipe>,
    pub staff_puzzles: Vec<StaffPuzzle>,
    pub quests: Vec<QuestDefinition>,
    pub quest_transitions: Vec<QuestTransition>,
    pub presentations: Vec<PresentationDefinition>,
    pub sounds: Vec<SoundDefinition>,
    pub animations: Vec<AnimationDefinition>,
    pub ui_assets: Vec<UiAsset>,
    pub cinematics: Vec<CinematicSequence>,
    #[serde(default)]
    pub localized_strings: Vec<LocalizedString>,
}

/// A validated index rejects a reference to an existing ID of the wrong kind.
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
            let m = &v.mechanics;
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
                    && positive(v.fire_interval)
                    && positive(v.reload_seconds)
                    && m.starting_reserve <= v.reserve
                    && m.pellets > 0
                    && m.pellets <= 64
                    && m.hit_multipliers.iter().all(|v| nonnegative(*v))
                    && m.spread.iter().all(|v| nonnegative(*v))
                    && nonnegative(m.penetration)
                    && m.damage_range.iter().all(|v| nonnegative(*v))
                    && m.damage_range[0] <= m.damage_range[1]
                    && m.recoil_pitch
                        .iter()
                        .chain(m.recoil_yaw.iter())
                        .all(|v| v.is_finite())
                    && m.recoil_pitch[0] <= m.recoil_pitch[1]
                    && m.recoil_yaw[0] <= m.recoil_yaw[1]
                    && positive(m.recoil_recovery)
                    && (1. ..=179.).contains(&m.ads_fov)
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
                    && m.ads_sight.as_ref().is_none_or(|s| {
                        matches!(s.joint.as_str(), "j_gun" | "tag_sights")
                            && s.point
                                .iter()
                                .chain(s.rotation.iter())
                                .all(|v| v.is_finite())
                            && (s.rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
                            && s.rotation[3].abs() >= (1_f32.to_radians()).cos()
                    })
                    && m.ads_seconds
                        .iter()
                        .chain(m.movement.iter())
                        .all(|v| positive(*v))
                    && [
                        m.empty_reload,
                        m.shell_seconds,
                        m.equip_seconds,
                        m.unequip_seconds,
                        m.projectile_lifetime,
                    ]
                    .into_iter()
                    .all(positive)
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
                    && m.melee_seconds.iter().all(|v| nonnegative(*v)),
                "Invalid weapon tuning",
            )?;
            if let FirePattern::Burst { shots } = m.fire {
                valid((1..=8).contains(&shots), "Invalid burst count")?;
            }
            if let Some(id) = &v.upgraded {
                r(&v.id.0, &id.0, "weapon")?;
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
                    && positive(v.speed)
                    && positive(v.attack_seconds)
                    && positive(v.death_seconds)
                    && positive(v.corpse_seconds)
                    && positive(v.bounds_margin)
                    && v.attack_hits
                        .iter()
                        .all(|t| nonnegative(*t) && *t <= v.attack_seconds)
                    && v.attack_hits.windows(2).all(|w| w[0] < w[1]),
                "Invalid enemy tuning",
            )?;
            optional(&v.id.0, &v.presentation, "presentation")?;
            let mut hit_ids = std::collections::BTreeSet::new();
            for hit in &v.hit_regions {
                valid(
                    !hit.id.is_empty()
                        && hit
                            .id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                        && hit_ids.insert(&hit.id)
                        && !hit.start.is_empty()
                        && !hit.end.is_empty()
                        && hit.radius.is_finite()
                        && hit.radius > 0.
                        && (v.presentation.as_ref() == Some(&hit.presentation)
                            || v.parts.iter().any(|p| p.presentation == hit.presentation)),
                    "Invalid enemy hit region",
                )?;
                index.resolve(&v.id.0, &hit.presentation.0, "presentation")?;
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
            for (alias, traversal) in &v.traversals {
                valid(
                    !alias.is_empty() && alias.len() < 128 && traversal.valid(),
                    "Invalid enemy traversal profile",
                )?;
                r(&v.id.0, &traversal.animation.0, "animation")?;
            }
        }
        for v in &self.perks {
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
                        && drink.activation_delay <= drink.lower_seconds,
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
                InteractionAction::Perk { perk } => r(&v.id.0, &perk.0, "perk")?,
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
            r(&v.id.0, &v.repair_sound.0, "sound")?;
            r(&v.id.0, &v.break_sound.0, "sound")?;
            valid(v.boards.len() == 6, "Invalid barrier board count")?;
            for b in &v.boards {
                r(&v.id.0, &b.presentation.0, "presentation")?;
                r(&v.id.0, &b.repair.0, "animation")?;
                r(&v.id.0, &b.pull.0, "animation")?;
                valid(
                    [
                        b.repair_seconds,
                        b.pull_seconds,
                        b.grab_seconds,
                        b.tear_seconds,
                    ]
                    .into_iter()
                    .all(positive)
                        && nonnegative(b.destroy_seconds)
                        && b.destroy_seconds < b.tear_seconds
                        && !b.grab_clip.is_empty()
                        && !b.tear_clip.is_empty(),
                    "Invalid barrier animation timing",
                )?;
                valid(b.valid_motion(), "Invalid barrier animation timing")?;
            }
        }
        for v in &self.navigation_nodes {
            r(&v.id.0, &v.zone.0, "zone")?;
            valid(
                v.position.iter().all(|x| x.is_finite()),
                "Invalid navigation position",
            )?;
        }
        for v in &self.navigation_links {
            r(&v.id.0, &v.from.0, "node")?;
            r(&v.id.0, &v.to.0, "node")?;
            optional(&v.id.0, &v.connection, "connection")?;
            for id in &v.blockers {
                r(&v.id.0, &id.0, "connection")?;
            }
            valid(
                v.source_animation
                    .as_ref()
                    .is_none_or(|s| !s.is_empty() && s.len() < 128),
                "Invalid traversal source animation",
            )?;
        }
        for v in &self.generators {
            r(&v.id.0, &v.placement.0, "placement")?;
            r(&v.id.0, &v.presentation.0, "presentation")?;
            r(&v.id.0, &v.capture_sound.0, "sound")?;
            r(&v.id.0, &v.active_sound.0, "sound")?;
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
                    && v.emergence.len() == 4
                    && v.emergence.iter().flatten().all(|v| v.is_finite())
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
                    && positive(v.distance)
                    && nonnegative(v.distance_min)
                    && v.distance_min <= v.distance,
                "Invalid sound tuning",
            )?;
        }
        for v in &self.animations {
            validate_asset_path(&v.path)?;
            r(&v.id.0, &v.skeleton.0, "presentation")?;
            valid(
                v.notifications
                    .iter()
                    .all(|n| nonnegative(n.seconds) && !n.event.is_empty())
                    && v.notifications
                        .windows(2)
                        .all(|w| w[0].seconds <= w[1].seconds),
                "Invalid animation notifications",
            )?;
        }
        for v in &self.ui_assets {
            validate_asset_path(&v.path)?;
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
