use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

#[derive(Clone, Deserialize)]
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
    pub model_parts: Vec<WeaponModelPart>,
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
    pub effects: WeaponEffects,
    #[serde(default)]
    pub attachments: BTreeMap<String, AttachmentDefinition>,
    #[serde(default)]
    pub throwable: Option<crate::equipment::ThrowableDefinition>,
    #[serde(default)]
    pub mine: Option<crate::mines::MineDefinition>,
    #[serde(default)]
    pub lure: Option<crate::lures::LureDefinition>,
}

#[derive(Clone, Default, Deserialize)]
pub struct WeaponEffects {
    pub flash: Option<String>,
    pub shell: Option<String>,
    pub last_shell: Option<String>,
    pub flash_offset: [f32; 3],
    pub shell_offset: [f32; 3],
    pub shell_rotation: [f32; 3],
    pub alternate_flash: bool,
    pub tracer: Option<String>,
    pub melee: Option<String>,
    pub melee_no_blood: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct WeaponModelPart {
    pub slot: u8,
    pub scene: String,
    pub tag: String,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub ads: bool,
}

#[derive(Clone, Deserialize)]
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
#[serde(default)]
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
    FirstEquip,
    EquipEmpty,
    QuickEquip,
    Unequip,
    UnequipEmpty,
    HoldFire,
    Melee,
    AdsEnter,
    AdsLeave,
    SprintEnter,
    SprintLoop,
    SprintLeave,
    SprintEnterEmpty,
    SprintLoopEmpty,
    SprintLeaveEmpty,
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
#[serde(tag = "mode", rename_all = "snake_case")]
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
#[serde(default)]
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
    pub impact_type: u32,
    pub projectile_stickiness: u32,
    pub projectile_clipmask: u32,
    pub projectile_explosion_force_normal_up: bool,
    pub weapon_class: Option<String>,
    pub spread: [f32; 4],
    pub ads_fov: f32,
    pub ads_seconds: [f32; 2],
    pub sprint_seconds: [f32; 3],
    pub sprint_duration_scale: f32,
    pub view_motion: WeaponViewMotion,
    pub crawl_seconds: [f32; 7],
    pub dive_seconds: [f32; 3],
    pub ads_zoom_fraction: [f32; 2],
    pub ads_sight: Option<AdsSight>,
    pub ads_enabled: bool,
    pub native_ads_pitch: Option<f32>,
    pub no_ads_auto_reload: bool,
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
    pub first_equip_seconds: f32,
    pub quick_equip_seconds: f32,
    pub empty_equip_seconds: f32,
    pub empty_unequip_seconds: f32,
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
            impact_type: 0,
            projectile_stickiness: 0,
            projectile_clipmask: 0,
            projectile_explosion_force_normal_up: false,
            weapon_class: None,
            spread: [3., 2., 1., 0.],
            ads_fov: 50.,
            ads_seconds: [0.2, 0.2],
            sprint_seconds: [0.; 3],
            sprint_duration_scale: 1.,
            view_motion: WeaponViewMotion::default(),
            crawl_seconds: [0.; 7],
            dive_seconds: [0.; 3],
            ads_zoom_fraction: [1., 1.],
            ads_sight: None,
            ads_enabled: true,
            native_ads_pitch: None,
            no_ads_auto_reload: false,
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
            first_equip_seconds: 0.,
            quick_equip_seconds: 0.,
            empty_equip_seconds: 0.,
            empty_unequip_seconds: 0.,
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

/// Native weapon-file values, in degrees and source inches.
#[derive(Clone, Default, Deserialize)]
pub struct WeaponViewMotion {
    pub scoped_ads: bool,
    pub sway: [f32; 6],
    pub ads_sway: [f32; 6],
    pub idle_amount: [f32; 2],
    pub idle_speed: [f32; 2],
    pub idle_stance: [f32; 2],
    pub sprint_bob: [f32; 2],
    pub dive_bob: [f32; 2],
    pub sprint_cycle: f32,
    pub dive_cycle: f32,
    pub ads_bob: f32,
    pub ads_view_bob: f32,
}

#[derive(Clone, Deserialize)]
pub struct AdsSight {
    pub joint: String,
    pub point: [f32; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Deserialize)]
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
pub struct EnemyRagdoll {
    pub bones: Vec<RagdollBone>,
    pub joints: Vec<RagdollJoint>,
    pub baselerp: Vec<RagdollBaseLerp>,
    pub self_pairs: Vec<[usize; 2]>,
}

#[derive(Clone, Deserialize)]
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
pub struct RagdollJoint {
    pub bone: usize,
    pub kind: RagdollJointKind,
    pub limits: Vec<RagdollLimit>,
}
#[derive(Clone, Deserialize)]
pub struct RagdollLimit {
    pub axis: [f32; 3],
    pub friction: f32,
    pub minimum: f32,
    pub maximum: f32,
}
#[derive(Clone, Deserialize)]
pub struct RagdollBaseLerp {
    pub joint: String,
    pub bone: usize,
    pub seconds: f32,
    pub base: [f32; 16],
}
#[derive(Clone, Deserialize)]
pub struct EnemyGibs {
    pub models: BTreeMap<crate::combat_effects::BodyGib, EnemyGibModels>,
    pub beheaded: ContentId,
    pub pieces: [EnemyGibPiece; 4],
    pub hat: Option<EnemyGibPiece>,
}
#[derive(Clone, Deserialize)]
pub struct EnemyGibPiece {
    pub presentation: ContentId,
    pub joint: String,
    pub physics: NativeModelPhysics,
}
#[derive(Clone, Deserialize)]
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

impl EnemyGibs {
    pub fn presentations(&self) -> impl Iterator<Item = &ContentId> {
        self.models
            .values()
            .flat_map(|models| [&models.body, &models.legs])
            .chain(std::iter::once(&self.beheaded))
    }
}
#[derive(Clone, Deserialize)]
pub struct EnemyGibModels {
    pub body: ContentId,
    pub legs: ContentId,
}
#[derive(Clone, Copy, Deserialize)]
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
}
#[derive(Clone, Deserialize)]
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
pub struct ActorPart {
    pub presentation: ContentId,
    pub socket: String,
    pub root_joint: Option<String>,
}
#[derive(Clone, Deserialize)]
pub struct EnemyTraversalDefinition {
    pub animation: ContentId,
    pub seconds: f32,
    pub motion: Vec<[f32; 4]>,
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
pub struct PerkSprintDefinition {
    pub duration_scale: f32,
    pub speed_scale: f32,
}
#[derive(Clone, Deserialize)]
pub struct PerkDrinkDefinition {
    pub bottle: WeaponId,
    pub seconds: f32,
    pub lower_seconds: f32,
    pub activation_delay: f32,
}
#[derive(Clone, Deserialize)]
pub struct PowerupDefinition {
    pub id: ContentId,
    pub duration: f32,
    pub presentation: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
pub struct ZoneDefinition {
    pub id: ZoneId,
    pub initially_open: bool,
}
#[derive(Clone, Deserialize)]
pub struct Placement {
    pub id: PlacementId,
    pub definition: ContentId,
    pub position: [f32; 3],
    pub yaw: f32,
    pub zone: ZoneId,
}
#[derive(Clone, Deserialize)]
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
#[serde(tag = "kind", rename_all = "snake_case")]
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
pub struct ZoneConnection {
    pub id: ContentId,
    pub from: ZoneId,
    pub to: ZoneId,
    pub initially_open: bool,
}
#[derive(Clone, Deserialize)]
pub struct NavigationNode {
    pub id: ContentId,
    pub position: [f32; 3],
    pub zone: ZoneId,
    #[serde(default)]
    pub native: Option<NativePathNode>,
}
#[derive(Clone, Copy, Deserialize)]
pub struct NativePathNode {
    pub r#type: u8,
    pub spawn_flags: i32,
}
#[derive(Clone, Deserialize)]
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

#[derive(Clone, PartialEq, Deserialize, Serialize)]
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
pub struct GeneratorReward {
    pub position: [f32; 3],
    pub yaw: f32,
    pub presentation: ContentId,
    pub powerup: ContentId,
    pub source_trigger: String,
}
#[derive(Clone, Deserialize)]
pub struct GeneratorEmergence {
    pub position: [f32; 3],
    pub yaw: f32,
}
#[derive(Clone, Deserialize)]
pub struct GeneratorAttackPoint {
    pub position: [f32; 3],
    pub face: [f32; 3],
    pub inaccessible: bool,
}
#[derive(Clone, Deserialize)]
pub struct RouteDefinition {
    pub id: ContentId,
    pub points: Vec<[f32; 3]>,
    pub speed: f32,
    pub cooldown: f32,
    pub presentation: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
pub struct WeatherDefinition {
    pub id: ContentId,
    pub duration: [f32; 2],
    pub snow: bool,
    pub sound: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
pub struct DigDefinition {
    pub id: ContentId,
    pub placement: PlacementId,
    pub snow_only: bool,
    pub loot: Vec<ContentId>,
}
#[derive(Clone, Deserialize)]
pub struct CraftingRecipe {
    pub id: ContentId,
    pub parts: Vec<ContentId>,
    pub result: ContentId,
    pub bench: PlacementId,
}
#[derive(Clone, Deserialize)]
pub struct StaffPuzzle {
    pub id: ContentId,
    pub weapon: WeaponId,
    pub interactions: Vec<InteractionId>,
    pub solution: Vec<String>,
    pub souls: u32,
}
#[derive(Clone, Deserialize)]
pub struct QuestTransition {
    pub id: ContentId,
    pub quest: QuestId,
    pub from: String,
    pub to: String,
    pub event: String,
    pub prerequisites: Vec<ContentId>,
}
#[derive(Clone, Deserialize)]
pub struct QuestDefinition {
    pub id: QuestId,
    pub initial_stage: String,
}
#[derive(Clone, Deserialize)]
pub struct PresentationDefinition {
    pub id: ContentId,
    pub scene: Option<String>,
    pub animations: Vec<ContentId>,
    pub sounds: Vec<ContentId>,
}
#[derive(Clone, Deserialize)]
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
pub struct AnimationRootMotion {
    pub seconds: f32,
    pub translation: Vec<[f32; 4]>,
    pub rotation: Vec<[f32; 5]>,
}
impl AnimationRootMotion {
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
pub struct AnimationNotification {
    pub seconds: f32,
    pub event: String,
}
#[derive(Clone, Deserialize)]
pub struct UiAsset {
    pub id: ContentId,
    pub path: String,
    #[serde(default)]
    pub material: Option<String>,
    #[serde(default)]
    pub sampler: Option<UiImageSampler>,
    #[serde(default)]
    pub font: Option<BitmapFont>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiImageSampler {
    pub clamp_u: bool,
    pub clamp_v: bool,
    pub clamp_w: bool,
    pub filter: String,
    pub mip_map: String,
}
#[derive(Clone, Deserialize)]
pub struct BitmapFont {
    pub height: f32,
    pub atlas: [f32; 2],
    pub glyphs: Vec<BitmapGlyph>,
}
#[derive(Clone, Deserialize)]
pub struct BitmapGlyph {
    pub codepoint: u32,
    pub offset: [f32; 2],
    pub advance: f32,
    pub size: [f32; 2],
    pub rect: [f32; 4],
}
#[derive(Clone, Deserialize)]
pub struct LocalizedString {
    pub id: ContentId,
    pub key: String,
    pub text: String,
}
#[derive(Clone, Deserialize)]
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
    pub localized_strings: Vec<LocalizedString>,
}
