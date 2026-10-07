//! Bounded captures of native first-person equipment phases in existing game runs.
use super::*;
use crate::{
    equipment::{GrenadeInventory, TacticalInventory},
    mines::MineInventory,
};
#[derive(Resource, Default)]
struct Captured(std::collections::BTreeSet<String>);
pub fn install(app: &mut App) {
    app.init_resource::<Captured>().add_systems(Last, capture);
}
fn capture(
    mut commands: Commands,
    verification: Option<Res<Verification>>,
    state: Res<State<GameState>>,
    definition: Option<Res<GameDefinition>>,
    equipment: Res<crate::presentation::EquipmentPresentationEvidence>,
    inventories: Query<
        (
            &GrenadeInventory,
            &TacticalInventory,
            &MineInventory,
            Option<&crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
    mut captured: ResMut<Captured>,
) {
    let (Some(verification), Some(definition)) = (verification, definition) else {
        return;
    };
    if **state != GameState::Playing
        || !(verification.solo_revive
            || verification.perk_health
            || verification.speed_cola
            || verification.stamin_up
            || verification.mule_kick
            || verification.firing_perks
            || verification.defensive_perks
            || verification.mobility
            || verification.mud
            || verification.doors
            || verification.grenades
            || verification.semtex
            || verification.monkeys
            || verification.claymores)
    {
        return;
    }
    let Ok((lethal, tactical, mines, drinking)) = inventories.single() else {
        return;
    };
    let Some(selection) = drinking
        .map(|d| d.selection())
        .or_else(|| crate::presentation::equipment_selection(lethal, tactical, mines, &definition))
    else {
        return;
    };
    let progress = selection
        .timing
        .map(|(duration, remaining)| ((duration - remaining) / duration).clamp(0., 1.));
    let checkpoint = match progress {
        Some(p) if p < 0.35 => return,
        Some(p) if p >= 0.9 => "-90",
        Some(p) if p >= 0.65 => "-65",
        _ => "",
    };
    let phase = format!("{}:{:?}", selection.weapon.0, selection.action);
    let key = phase.clone() + checkpoint;
    if !equipment.phases.contains(&(phase.clone() + ":arms"))
        || !equipment.phases.contains(&(phase + ":model"))
        || !captured.0.insert(key)
    {
        return;
    }
    let name = format!(
        "equipment-{}-{:?}{checkpoint}.png",
        selection.weapon.0, selection.action
    );
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(verification.output.join(name)));
}
