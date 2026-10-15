#![cfg(feature = "game")]

use origins::assets::LaunchOptions;
use std::{ffi::OsString, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-assets-launch");
        std::fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!("{}-{}", std::process::id(), NEXT_ROOT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn arguments(&self, scenario: &[&str], root_first: bool) -> Vec<OsString> {
        let mut arguments: Vec<OsString> = scenario.iter().map(|argument| OsString::from(*argument)).collect();
        let root = [OsString::from("--asset-root"), self.0.clone().into_os_string()];
        if root_first { arguments.splice(0..0, root); } else { arguments.extend(root); }
        arguments
    }
}
impl Drop for Root {
    fn drop(&mut self) { std::fs::remove_dir(&self.0).unwrap(); }
}

#[test]
fn asset_root_is_required_absolute_existing_and_unique() {
    let root = Root::new();
    for arguments in [
        vec![],
        vec![OsString::from("--asset-root")],
        vec![OsString::from("--asset-root"), OsString::from("relative-assets")],
        vec![OsString::from("--asset-root"), root.0.join("missing").into_os_string()],
        vec![OsString::from("--asset-root"), root.0.clone().into_os_string(),
             OsString::from("--asset-root"), root.0.clone().into_os_string()],
    ] {
        assert!(LaunchOptions::parse(arguments.into_iter()).error.is_some());
    }
    let options = LaunchOptions::parse(root.arguments(&[], true).into_iter());
    assert!(options.error.is_none(), "{:?}", options.error);
    assert_eq!(options.asset_root, root.0);
}

#[test]
fn asset_root_before_or_after_scenarios_preserves_gameplay_flags() {
    let root = Root::new();
    let scenarios: &[(&str, fn(&LaunchOptions) -> bool)] = &[
        ("--verify-controls", |o| o.verify_controls),
        ("--verify-hazards", |o| o.verify_hazards),
        ("--verify-powerups", |o| o.verify_powerups),
        ("--verify-weapon-actions", |o| o.verify_weapon_actions),
        ("--verify-replacements", |o| o.verify_replacements),
        ("--verify-wall-weapons", |o| o.verify_wall_weapons),
        ("--verify-weapon-assemblies", |o| o.verify_weapon_assemblies),
        ("--verify-ads", |o| o.verify_ads),
        ("--verify-rounds", |o| o.verify_rounds),
        ("--verify-health", |o| o.verify_health),
        ("--verify-solo-revive", |o| o.verify_solo_revive),
        ("--verify-solo-bleedout", |o| o.verify_solo_bleedout),
        ("--verify-perk-health", |o| o.verify_perk_health),
        ("--verify-speed-cola", |o| o.verify_speed_cola),
        ("--verify-stamin-up", |o| o.verify_stamin_up),
        ("--verify-mule-kick", |o| o.verify_mule_kick),
        ("--verify-firing-perks", |o| o.verify_firing_perks),
        ("--verify-defensive-perks", |o| o.verify_defensive_perks),
        ("--verify-mobility", |o| o.verify_mobility),
        ("--verify-mud", |o| o.verify_mud),
        ("--verify-doors", |o| o.verify_doors),
        ("--verify-navigation", |o| o.verify_navigation),
        ("--verify-traversal", |o| o.verify_traversal),
        ("--verify-locomotion", |o| o.verify_locomotion),
        ("--verify-audio", |o| o.verify_audio),
        ("--verify-barriers", |o| o.verify_barriers),
        ("--verify-generators", |o| o.verify_generators),
        ("--verify-hit-regions", |o| o.verify_hit_regions),
        ("--verify-grenades", |o| o.verify_grenades),
        ("--verify-monkeys", |o| o.verify_monkeys),
        ("--verify-equipment-refills", |o| o.verify_equipment_refills),
        ("--verify-claymores", |o| o.verify_claymores),
        ("--verify-semtex", |o| o.verify_semtex),
    ];
    for &(flag, selected) in scenarios {
        for first in [true, false] {
            let options = LaunchOptions::parse(root.arguments(&[flag, "target/scenario-output"], first).into_iter());
            assert!(options.error.is_none(), "{flag}: {:?}", options.error);
            assert!(selected(&options), "{flag} lost its scenario selection");
            assert_eq!(options.verify, Some(PathBuf::from("target/scenario-output")));
            assert_eq!(options.asset_root, root.0);
        }
    }
}

#[test]
fn view_index_and_scenario_dependencies_survive_root_extraction() {
    let root = Root::new();
    for first in [true, false] {
        let options = LaunchOptions::parse(root.arguments(&["--benchmark-moving-view", "target/scenario-output", "3"], first).into_iter());
        assert!(options.error.is_none(), "{:?}", options.error);
        assert!(options.benchmark && options.benchmark_motion);
        assert_eq!(options.benchmark_view, Some(3));
        let options = LaunchOptions::parse(root.arguments(&["--verify-traversal", "target/scenario-output"], first).into_iter());
        assert!(options.verify_traversal && options.verify_navigation && options.verify_doors);
        let options = LaunchOptions::parse(root.arguments(&["--verify-solo-bleedout", "target/scenario-output"], first).into_iter());
        assert!(options.verify_solo_bleedout && options.verify_solo_revive);
        let options = LaunchOptions::parse(root.arguments(&["--verify-weapon-assemblies", "target/scenario-output", "--dual-wield-only"], first).into_iter());
        assert!(options.error.is_none(), "{:?}", options.error);
        assert!(options.verify_weapon_assemblies);
    }
}

#[test]
#[cfg(all(debug_assertions, feature = "dev-tools"))]
fn remote_scenario_accepts_root_on_either_side() {
    let root = Root::new();
    for first in [true, false] {
        let options = LaunchOptions::parse(root.arguments(&["--validation-remote", "15708"], first).into_iter());
        assert!(options.error.is_none(), "{:?}", options.error);
        assert_eq!(options.validation_remote, Some(15708));
    }
}
