use super::*;
use crate::staff_upgrades::{
    ELEMENTS, StaffUpgrades,
    lights::{LightShow, cycle},
};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Setup {
    upgraded_staffs: Option<usize>,
    unlocked: Option<[bool; 4]>,
}

pub(super) fn schema(_: In<Option<Value>>) -> BrpResult {
    Ok(json!({"version":1,"methods":{
        "origins/quests/setup":{"effect":"Direct session fixture; upgrading through gameplay is outside this fixture",
            "params":{"upgraded_staffs":"optional 0..4 upgraded-staff gate count","unlocked":"optional [fire,ice,wind,lightning] charger availability"}},
        "origins/state":{"staffs":"upgrade charging, source light sequence, per-disc material colors and identities"}},
        "light_cycles":{"normal":cycle(false),"bellaso":cycle(true)}}))
}

pub(super) fn snapshot(world: &mut World) -> Value {
    let upgrades = world.resource::<StaffUpgrades>();
    let show = world.resource::<LightShow>();
    let mut result = json!({"rings":upgrades.rings,"unlocked":upgrades.unlocked,
        "inserted":upgrades.inserted,"charged":upgrades.charged,"souls":upgrades.souls,
        "lights":{"active":show.active,"mode":show.mode,"on":show.on,"elapsed":show.elapsed,
            "cycles":show.cycles,"sequence":show.sequence,"events":show.events}});
    result["light_materials"] = crate::rendering::opaque::quest_light_snapshot(world);
    result["light_bindings"] = crate::staff_upgrades::lights::binding_snapshot(world);
    result
}

pub(super) fn setup(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Setup = super::parse(value)?;
    if *world.resource::<State<GameState>>().get() != GameState::Playing {
        return Err(super::error("Quest setup requires Playing"));
    }
    if request.upgraded_staffs.is_some_and(|count| count > 4) {
        return Err(super::error("Upgraded staff fixture count exceeds four"));
    }
    if let Some(count) = request.upgraded_staffs {
        let mut quest = world.resource_mut::<crate::quests::MainQuest>();
        quest.upgraded = ELEMENTS.into_iter().take(count).collect();
    }
    if let Some(unlocked) = request.unlocked {
        world.resource_mut::<StaffUpgrades>().unlocked = unlocked;
    }
    Ok(json!({"applied":true,"scenario_setup_only":true}))
}
