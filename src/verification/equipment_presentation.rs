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
    if selection
        .timing
        .is_some_and(|(duration, remaining)| duration - remaining < duration * 0.35)
    {
        return;
    }
    let key = format!("{}:{:?}", selection.weapon.0, selection.action);
    if !equipment.phases.contains(&(key.clone() + ":arms"))
        || !equipment.phases.contains(&(key.clone() + ":model"))
        || !captured.0.insert(key)
    {
        return;
    }
    let name = format!(
        "equipment-{}-{:?}.png",
        selection.weapon.0, selection.action
    );
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(verification.output.join(name)));
}
