//! Source-authored Origins weapon gameplay data and ordinary exported clip bindings.
use super::{
    AnimationDefinition, AnimationNotification, ContentId, GameDefinition, LocalizedString,
    PresentationDefinition, UiAsset, WeaponAnimation, WeaponDefinition,
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
#[derive(Deserialize)]
struct Catalog {
    weapons: Vec<WeaponDefinition>,
    presentations: Vec<PresentationDefinition>,
    animations: Vec<AnimationDefinition>,
    ui_assets: Vec<UiAsset>,
    definitions: Vec<ContentId>,
    localized_strings: Vec<LocalizedString>,
}
#[derive(Deserialize)]
struct SourceWeapon {
    info: BTreeMap<String, String>,
    scene: String,
    clip_bindings: BTreeMap<String, String>,
    #[serde(default)]
    parts: Vec<super::WeaponModelPart>,
}
#[derive(Deserialize)]
struct SourceClip {
    frames: u32,
    fps: f32,
    looped: bool,
    notes: Vec<(String, f32)>,
}
impl SourceClip {
    fn apply(
        &self,
        animation: &mut AnimationDefinition,
        audio: &mut crate::audio::AudioDefinition,
        note_sounds: &BTreeMap<&str, &str>,
    ) -> Result<(), String> {
        if self.fps <= 0. || !self.fps.is_finite() || self.frames == 0 {
            return Err(format!(
                "Weapon source track {} has invalid timing",
                animation.id.0
            ));
        }
        animation.seconds = self.frames as f32 / self.fps;
        animation.looping = self.looped;
        animation.notifications = self
            .notes
            .iter()
            .map(|(event, seconds)| AnimationNotification {
                event: event.clone(),
                seconds: (seconds * self.fps).round() / self.fps,
            })
            .collect();
        audio.timelines.insert(
            animation.id.0.clone(),
            self.notes
                .iter()
                .filter_map(|(note, seconds)| {
                    let alias = note
                        .strip_prefix("sndnt#")
                        .or_else(|| note_sounds.get(note.as_str()).copied())?;
                    Some(crate::audio::SoundNote {
                        seconds: (seconds * self.fps).round() / self.fps,
                        alias: alias.into(),
                    })
                })
                .collect(),
        );
        Ok(())
    }
}
fn canonical_clip(id: &mut ContentId) {
    if let Some((prefix, name)) = id.0.rsplit_once('.') {
        id.0 = format!("{prefix}.{}", name.to_ascii_lowercase());
    }
}
pub(crate) fn bind(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    let mut catalog: Catalog = super::read(&root.join("weapons.json"));
    let file = root.join("source-weapons.json");
    let facts: BTreeMap<String, SourceWeapon> =
        serde_json::from_slice(&std::fs::read(&file).map_err(|error| {
            format!(
                "Required weapon source bindings {}: {error}",
                file.display()
            )
        })?)
        .map_err(|error| format!("Invalid weapon source bindings: {error}"))?;
    let file = root.join("source-hands.json");
    let hands: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_slice(&std::fs::read(&file).map_err(|error| {
            format!("Required hands source bindings {}: {error}", file.display())
        })?)
        .map_err(|error| format!("Invalid hands source bindings: {error}"))?;
    let hands = hands
        .get("dempsey")
        .ok_or("Required named Dempsey source hand clip bindings absent")?;
    let tracks: BTreeMap<String, SourceClip> =
        super::read(&root.join("source-viewmodel-clips.json"));
    for weapon in &mut catalog.weapons {
        for clip in weapon.animation_actions.values_mut() {
            canonical_clip(clip);
        }
    }
    for presentation in &mut catalog.presentations {
        for clip in &mut presentation.animations {
            canonical_clip(clip);
        }
    }
    for animation in &mut catalog.animations {
        canonical_clip(&mut animation.id);
        let name = animation.id.0.rsplit('.').next().unwrap();
        if let Some(track) = tracks.get(name) {
            track.apply(animation, &mut game.audio, &BTreeMap::new())?;
        }
    }
    for weapon in &mut catalog.weapons {
        if (weapon.throwable.is_some() || weapon.mine.is_some())
            && weapon.mechanics.projectile_clipmask == 0
        {
            return Err(format!(
                "Required source explosion clipmask absent for {}",
                weapon.id.0
            ));
        }
        let name = weapon
            .id
            .0
            .strip_prefix("weapon.")
            .ok_or_else(|| format!("Invalid source weapon ID {}", weapon.id.0))?;
        let Some(source) = facts.get(name) else {
            continue;
        };
        weapon.model_parts = source.parts.clone();
        let required_parts = (1..=8)
            .filter(|index| {
                source
                    .info
                    .get(&format!("attachViewModel{index}"))
                    .is_some_and(|model| !model.is_empty())
            })
            .count();
        if weapon.model_parts.len() != required_parts {
            return Err(format!(
                "Weapon {name} requires {required_parts} source model pieces; refresh the weapon piece export"
            ));
        }
        let text = |key: &str| {
            source
                .info
                .get(key)
                .filter(|v| !v.is_empty())
                .map(String::as_str)
        };
        if let Some(tags) = text("hideTags") {
            weapon
                .attachments
                .entry("defaultattachment".into())
                .or_default()
                .hide_tags = tags.split_whitespace().map(str::to_owned).collect();
        }
        let number = |key: &str| -> Result<f32, String> {
            text(key)
                .unwrap_or("0")
                .parse::<f32>()
                .map_err(|_| format!("Weapon {name} has invalid {key}"))
        };
        let motion_number = |key: &str| -> Result<f32, String> {
            let value = text(key)
                .ok_or_else(|| format!("Weapon {name} lacks required motion value {key}"))?
                .parse::<f32>()
                .map_err(|_| format!("Weapon {name} has invalid motion value {key}"))?;
            if !value.is_finite() {
                return Err(format!("Weapon {name} has nonfinite motion value {key}"));
            }
            Ok(value)
        };
        let sway = |prefix: &str| -> Result<[f32; 6], String> {
            Ok([
                motion_number(&format!("{prefix}MaxAngle"))?,
                motion_number(&format!("{prefix}LerpSpeed"))?,
                motion_number(&format!("{prefix}PitchScale"))?,
                motion_number(&format!("{prefix}YawScale"))?,
                motion_number(&format!("{prefix}HorizScale"))?,
                motion_number(&format!("{prefix}VertScale"))?,
            ])
        };
        weapon.mechanics.sprint_duration_scale = motion_number("sprintDurationScale")?;
        weapon.mechanics.spread = [
            motion_number("hipSpreadStandMin")?,
            motion_number("hipSpreadDuckedMin")?,
            motion_number("hipSpreadProneMin")?,
            motion_number("adsSpread")?,
        ];
        weapon.mechanics.hip_spread = Some(super::content::WeaponHipSpread {
            maximum: [
                motion_number("hipSpreadMax")?,
                motion_number("hipSpreadDuckedMax")?,
                motion_number("hipSpreadProneMax")?,
            ],
            decay: motion_number("hipSpreadDecayRate")?,
            fire_add: motion_number("hipSpreadFireAdd")?,
            turn_add: motion_number("hipSpreadTurnAdd")?,
            move_add: motion_number("hipSpreadMoveAdd")?,
            stance_decay: [
                motion_number("hipSpreadDuckedDecay")?,
                motion_number("hipSpreadProneDecay")?,
            ],
        });
        weapon.mechanics.view_motion = super::content::WeaponViewMotion {
            scoped_ads: text("adsOverlayReticle").is_some_and(|value| value != "none"),
            sway: sway("sway")?,
            ads_sway: sway("adsSway")?,
            idle_amount: [
                motion_number("hipIdleAmount")?,
                motion_number("adsIdleAmount")?,
            ],
            idle_speed: [
                motion_number("hipIdleSpeed")?,
                motion_number("adsIdleSpeed")?,
            ],
            idle_stance: [
                motion_number("idleCrouchFactor")?,
                motion_number("idleProneFactor")?,
            ],
            sprint_bob: [motion_number("sprintBobH")?, motion_number("sprintBobV")?],
            dive_bob: [motion_number("dtpBobH")?, motion_number("dtpBobV")?],
            sprint_cycle: motion_number("sprintScale")?,
            dive_cycle: motion_number("dtpScale")?,
            ads_bob: motion_number("adsBobFactor")?,
            ads_view_bob: motion_number("adsViewBobMult")?,
        };
        if let Some(throwable) = &mut weapon.throwable {
            throwable.speed = number("projectileSpeed")? * crate::assets::SOURCE_UNIT_METERS;
            throwable.up_speed = number("projectileSpeedUp")? * crate::assets::SOURCE_UNIT_METERS;
            throwable.relative_up_speed =
                number("projectileSpeedRelativeUp")? * crate::assets::SOURCE_UNIT_METERS;
            throwable.rotating = text("rotate") == Some("1");
        }
        weapon.effects = super::content::WeaponEffects {
            flash: text("viewFlashEffect").map(str::to_owned),
            shell: text("viewShellEjectEffect").map(str::to_owned),
            last_shell: text("viewLastShotEjectEffect").map(str::to_owned),
            flash_offset: [
                number("viewFlashOffsetF")?,
                number("viewFlashOffsetR")?,
                number("viewFlashOffsetU")?,
            ],
            shell_offset: [
                number("viewShellEjectOffsetF")?,
                number("viewShellEjectOffsetR")?,
                number("viewShellEjectOffsetU")?,
            ],
            shell_rotation: [
                number("viewShellEjectRotationP")?,
                number("viewShellEjectRotationY")?,
                number("viewShellEjectRotationR")?,
            ],
            alternate_flash: text("useAltTagFlash") == Some("1"),
            tracer: text("tracerType").map(str::to_owned),
            melee: text("meleeImpactEffect").map(str::to_owned),
            melee_no_blood: text("meleeImpactNoBloodEffect").map(str::to_owned),
        };
        let Some(id) = &weapon.presentation else {
            continue;
        };
        let presentation = catalog
            .presentations
            .iter_mut()
            .find(|p| &p.id == id)
            .ok_or_else(|| format!("Missing weapon presentation {}", id.0))?;
        presentation.scene = Some(source.scene.clone());
        let prefix = weapon
            .animation_actions
            .get(&WeaponAnimation::Idle)
            .and_then(|id| id.0.rsplit_once('.').map(|(prefix, _)| prefix.to_owned()))
            .unwrap_or_else(|| {
                format!("animation.{}", id.0.strip_prefix("actor.").unwrap_or(&id.0))
            });
        for (action, key) in [
            (WeaponAnimation::AdsEnter, "adsUpAnim"),
            (WeaponAnimation::AdsLeave, "adsDownAnim"),
            (WeaponAnimation::FirstEquip, "firstRaiseAnim"),
            (WeaponAnimation::EquipEmpty, "emptyRaiseAnim"),
            (WeaponAnimation::QuickEquip, "quickRaiseAnim"),
            (WeaponAnimation::UnequipEmpty, "emptyDropAnim"),
            (WeaponAnimation::HoldFire, "holdFireAnim"),
            (WeaponAnimation::SprintEnterEmpty, "sprintInEmptyAnim"),
            (WeaponAnimation::SprintLoopEmpty, "sprintLoopEmptyAnim"),
            (WeaponAnimation::SprintLeaveEmpty, "sprintOutEmptyAnim"),
        ] {
            let Some(name) = text(key).map(str::to_lowercase) else {
                continue;
            };
            let clip_id = ContentId(format!("{prefix}.{name}"));
            weapon.animation_actions.insert(action, clip_id.clone());
            if !catalog.animations.iter().any(|a| a.id == clip_id) {
                catalog.animations.push(AnimationDefinition {
                    id: clip_id.clone(),
                    path: String::new(),
                    skeleton: id.clone(),
                    seconds: 0.,
                    looping: false,
                    notifications: Vec::new(),
                    root_motion: None,
                });
                presentation.animations.push(clip_id);
            }
        }
        if text("adsUpAnim").is_some() && text("adsDownAnim").is_some() {
            weapon.mechanics.native_ads_pitch = Some(number("adsAimPitch")?);
        }
        weapon.mechanics.no_ads_auto_reload = text("noADSAutoReload") == Some("1");
        weapon.mechanics.first_equip_seconds = number("firstRaiseTime")?;
        weapon.mechanics.quick_equip_seconds = number("quickRaiseTime")?;
        weapon.mechanics.empty_equip_seconds = number("emptyRaiseTime")?;
        weapon.mechanics.empty_unequip_seconds = number("emptyDropTime")?;
        for (event, key) in [
            ("shot", "fireSoundPlayer"),
            ("dry_fire", "emptyFireSoundPlayer"),
            ("equip", "raiseSoundPlayer"),
            ("first_equip", "firstRaiseSoundPlayer"),
            ("unequip", "putawaySoundPlayer"),
            ("ads_enter", "adsRaiseSoundPlayer"),
            ("ads_leave", "adsLowerSoundPlayer"),
            ("prime", "pullbackSoundPlayer"),
            ("melee", "meleeSwipeSoundPlayer"),
            ("hit", "meleeHitSound"),
            ("purchase", "pickupSoundPlayer"),
            ("ammo", "ammoPickupSoundPlayer"),
            ("reload", "reloadSoundPlayer"),
        ] {
            if let Some(alias) = text(key) {
                game.audio
                    .bindings
                    .insert(format!("{}.{}", weapon.id.0, event), alias.into());
            }
        }
        if let Some(alias) = text("lastShotSoundPlayer").or_else(|| text("fireSoundPlayer")) {
            game.audio
                .bindings
                .insert(format!("{}.last_shot", weapon.id.0), alias.into());
        }
        if let Some(alias) = text("reloadEmptySoundPlayer").or_else(|| text("reloadSoundPlayer")) {
            game.audio
                .bindings
                .insert(format!("{}.reload_empty", weapon.id.0), alias.into());
        }
        // zm_tomb.gsc::sndmeleeswipe owns these sounds; the weapon-file
        // placeholders are intentionally absent from the Origins sound banks.
        let melee = match name {
            "tomb_shield_zm" => Some("fly_riotshield_zm_swing".to_owned()),
            "one_inch_punch_fire_zm" => Some("wpn_one_inch_punch_fire_plr".into()),
            "one_inch_punch_air_zm" => Some("wpn_one_inch_punch_air_plr".into()),
            "one_inch_punch_ice_zm" => Some("wpn_one_inch_punch_ice_plr".into()),
            "one_inch_punch_lightning_zm" => Some("wpn_one_inch_punch_lightning_plr".into()),
            "one_inch_punch_zm" | "one_inch_punch_upgraded_zm" => {
                Some("wpn_one_inch_punch_plr".into())
            }
            name if name.starts_with("staff_") && name.contains("melee") => {
                Some("zmb_melee_staff_upgraded_plr".into())
            }
            "knife_zm" => Some("zmb_melee_whoosh_plr".into()),
            _ => None,
        };
        if let Some(alias) = melee {
            game.audio
                .bindings
                .insert(format!("{}.melee", weapon.id.0), alias);
        }
        let note_sounds: BTreeMap<_, _> = text("notetrackSoundMap")
            .unwrap_or("")
            .lines()
            .filter_map(|line| {
                let mut tokens = line.split_whitespace();
                Some((tokens.next()?, tokens.next()?))
            })
            .collect();
        for animation in catalog
            .animations
            .iter_mut()
            .filter(|animation| &animation.skeleton == id)
        {
            let name = animation
                .id
                .0
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            animation.path = source
                .clip_bindings
                .get(&name)
                .or_else(|| hands.get(&name))
                .cloned()
                .ok_or_else(|| {
                    format!("Required exported weapon animation {name} has no exact asset binding")
                })?;
            if let Some(track) = tracks.get(&name) {
                track.apply(animation, &mut game.audio, &note_sounds)?;
            } else if name.starts_with("viewmodel_") {
                return Err(format!("Required weapon source track {name}"));
            }
        }
    }
    game.content.weapons = catalog.weapons;
    game.content.presentations.extend(catalog.presentations);
    game.content.animations.extend(catalog.animations);
    game.content.ui_assets.extend(catalog.ui_assets);
    game.content
        .localized_strings
        .extend(catalog.localized_strings);
    game.content.definitions.extend(catalog.definitions);
    Ok(())
}

#[cfg(test)]
mod source_catalog_tests {
    use super::Catalog;

    #[test]
    fn source_notes_are_snapped_back_to_frames_at_the_clip_endpoint() {
        let track = super::SourceClip {
            frames: 2,
            fps: 30.,
            looped: false,
            notes: vec![("sndnt#last".into(), 0.0667)],
        };
        let mut animation = crate::origins::AnimationDefinition {
            id: crate::origins::ContentId("animation.test".into()),
            path: String::new(),
            skeleton: crate::origins::ContentId("actor.test".into()),
            seconds: 0.,
            looping: false,
            notifications: Vec::new(),
            root_motion: None,
        };
        let mut audio = crate::audio::AudioDefinition::default();
        track
            .apply(
                &mut animation,
                &mut audio,
                &std::collections::BTreeMap::new(),
            )
            .unwrap();
        assert_eq!(
            audio.timelines["animation.test"][0].seconds,
            animation.seconds
        );
        assert_eq!(animation.notifications[0].seconds, animation.seconds);
    }

    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_player_weapons_bind_native_ads_sounds_and_all_character_hands() {
        use super::*;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let game = GameDefinition::load_from_root(&root);
        let facts: BTreeMap<String, SourceWeapon> =
            super::super::read(&root.join("source-weapons.json"));
        let hands: BTreeMap<String, BTreeMap<String, String>> =
            super::super::read(&root.join("source-hands.json"));
        let mut ads_count = 0;
        for weapon in &game.content.weapons {
            let Some(source) = facts.get(weapon.id.0.strip_prefix("weapon.").unwrap()) else {
                continue;
            };
            if weapon.presentation.is_none() {
                continue;
            }
            assert_eq!(
                weapon.effects.flash.as_deref(),
                source
                    .info
                    .get("viewFlashEffect")
                    .filter(|value| !value.is_empty())
                    .map(String::as_str)
            );
            assert_eq!(
                weapon.effects.tracer.as_deref(),
                source
                    .info
                    .get("tracerType")
                    .filter(|value| !value.is_empty())
                    .map(String::as_str)
            );
            for part in &weapon.model_parts {
                let scene = part.scene.split('#').next().unwrap();
                assert!(root.join(scene).is_file(), "{scene}");
                assert!(game.scene_models.contains_key(scene), "{scene}");
                let clips = &game.scene_animations[scene];
                for clip in weapon.animation_actions.values() {
                    let name = clip.0.rsplit('.').next().unwrap();
                    assert!(
                        clips.contains_key(name),
                        "{} piece {} missing {name}",
                        weapon.id.0,
                        part.scene
                    );
                    if let Some(path) = &clips[name] {
                        assert!(root.join(path.split('#').next().unwrap()).is_file());
                    }
                }
            }
            if let Some(alias) = source.info.get("fireSoundPlayer").filter(|v| !v.is_empty()) {
                assert_eq!(
                    &game.audio.bindings[&format!("{}.shot", weapon.id.0)],
                    alias
                );
                let resolved =
                    crate::audio::resolve_alias(&game, alias).expect("Player shot alias");
                let native = &game.audio.aliases[resolved].native;
                assert!(native.iter().any(|entry| entry.sound.is_some()), "{alias}");
                for sound in native.iter().filter_map(|entry| entry.sound.as_ref()) {
                    let recording = game
                        .content
                        .sounds
                        .iter()
                        .find(|recording| recording.id == *sound)
                        .unwrap();
                    assert!(root.join(&recording.path).is_file(), "{}", recording.path);
                }
            }
            for (action, key) in [
                (WeaponAnimation::AdsEnter, "adsUpAnim"),
                (WeaponAnimation::AdsLeave, "adsDownAnim"),
                (WeaponAnimation::FirstEquip, "firstRaiseAnim"),
            ] {
                let Some(name) = source.info.get(key).filter(|v| !v.is_empty()) else {
                    continue;
                };
                let id = &weapon.animation_actions[&action];
                assert_eq!(id.0.rsplit('.').next().unwrap(), name.to_lowercase());
                let clip = game
                    .content
                    .animations
                    .iter()
                    .find(|a| &a.id == id)
                    .unwrap();
                assert!(clip.seconds > 0.);
                assert!(root.join(clip.path.split('#').next().unwrap()).is_file());
                for character in ["dempsey", "nikolai", "richtofen", "takeo"] {
                    let path = &hands[character][&name.to_lowercase()];
                    assert!(root.join(path.split('#').next().unwrap()).is_file());
                }
            }
            if source.info.get("adsUpAnim").is_some_and(|v| !v.is_empty()) {
                ads_count += 1;
                assert_eq!(
                    weapon.mechanics.native_ads_pitch.unwrap(),
                    source.info["adsAimPitch"].parse::<f32>().unwrap()
                );
            }
        }
        assert!(ads_count >= 90);
        assert!(
            (game.audio.aliases["wpn_870mcs_fire_plr"].native[0].volume_max - 32845. / 65535.)
                .abs()
                < 1e-6
        );
        assert_eq!(
            game.audio.bindings["weapon.c96_upgraded_zm.shot"],
            "wpn_rgunmk2_fire_plr"
        );
        for weapon in game
            .content
            .weapons
            .iter()
            .filter(|w| w.id.0 == "weapon.knife_zm")
        {
            let id = &weapon.animation_actions[&WeaponAnimation::Melee];
            assert_eq!(id.0, id.0.to_lowercase());
            assert_eq!(
                game.audio.bindings["weapon.knife_zm.melee"],
                "zmb_melee_whoosh_plr"
            );
        }
    }

    #[test]
    fn authored_catalog_deserializes_with_per_surface_throwable_tables() {
        let catalog: Catalog =
            serde_json::from_str(include_str!("../../gameplay/origins/weapons.json"))
                .expect("source-authored weapon data must match runtime types");
        let monkey = catalog
            .weapons
            .iter()
            .find(|weapon| weapon.id.0 == "weapon.cymbal_monkey_zm")
            .and_then(|weapon| weapon.throwable.as_ref())
            .expect("source monkey definition");
        assert_eq!(monkey.tangent_bounce[0], 0.5);
        assert_eq!(monkey.tangent_bounce[1], 0.6);
        assert_eq!(monkey.normal_bounce[6], 0.225);
        assert_eq!(monkey.normal_bounce[31], 0.5);
    }

    #[test]
    fn explosion_masks_preserve_native_plantable_and_body_rules() {
        let catalog: Catalog =
            serde_json::from_str(include_str!("../../gameplay/origins/weapons.json")).unwrap();
        for (name, mask, stickiness) in [
            ("weapon.frag_grenade_zm", 0x0280e893, 0),
            ("weapon.cymbal_monkey_zm", 0x0280e893, 0),
            ("weapon.sticky_grenade_zm", 0x0280e893, 1),
            ("weapon.claymore_zm", 0x0081e893, 4),
        ] {
            let weapon = catalog.weapons.iter().find(|w| w.id.0 == name).unwrap();
            assert_eq!(weapon.mechanics.projectile_clipmask, mask, "{name}");
            assert_eq!(weapon.mechanics.projectile_stickiness, stickiness, "{name}");
            assert_eq!(weapon.mechanics.impact_type, 7, "{name}");
        }
    }
}
