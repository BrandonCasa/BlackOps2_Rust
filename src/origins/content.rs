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
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub id: ContentId,
    pub health: u32,
    pub speed: f32,
    pub damage: u32,
    pub presentation: Option<ContentId>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkDefinition {
    pub id: PerkId,
    pub name: String,
    pub price: u32,
    pub icon: Option<ContentId>,
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
    Door { connection: ContentId, price: u32 },
    Weapon { weapon: WeaponId, price: u32 },
    Perk { perk: PerkId },
    Generator { generator: ContentId },
    Quest { quest: QuestId, event: String },
    Pickup { item: ContentId },
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
}
#[derive(Clone, Deserialize)]
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
pub struct CinematicSequence {
    pub id: ContentId,
    pub frames: Vec<String>,
    pub frames_per_second: f32,
    pub audio: Option<ContentId>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentCatalog {
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
                v.damage > 0
                    && v.magazine > 0
                    && positive(v.fire_interval)
                    && positive(v.reload_seconds),
                "Invalid weapon tuning",
            )?;
            if let Some(id) = &v.upgraded {
                r(&v.id.0, &id.0, "weapon")?;
            }
            optional(&v.id.0, &v.presentation, "presentation")?;
        }
        for v in &self.enemies {
            valid(
                v.health > 0 && v.damage > 0 && positive(v.speed),
                "Invalid enemy tuning",
            )?;
            optional(&v.id.0, &v.presentation, "presentation")?;
        }
        for v in &self.perks {
            optional(&v.id.0, &v.icon, "ui")?;
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
        }
        for v in &self.generators {
            r(&v.id.0, &v.placement.0, "placement")?;
            for z in &v.zones {
                r(&v.id.0, &z.0, "zone")?;
            }
            valid(
                positive(v.capture_seconds) && positive(v.radius),
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
                    && positive(v.distance),
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
