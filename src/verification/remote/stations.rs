use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    active_generators: Vec<u32>,
    normal_box: Option<usize>,
}

pub(super) fn setup(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Setup = parse(value)?;
    if request.active_generators.len() > 6
        || request
            .active_generators
            .iter()
            .any(|n| !(1..=6).contains(n))
        || request.normal_box.is_some_and(|n| n >= 6)
    {
        return Err(error("Invalid station fixture"));
    }
    let definitions = world
        .resource::<GameDefinition>()
        .content
        .generators
        .clone();
    let mut generators = world.resource_mut::<crate::generators::Generators>();
    for definition in definitions {
        if let Some(site) = generators.sites.get_mut(&definition.id) {
            site.state = if request
                .active_generators
                .contains(&(definition.number as u32))
            {
                crate::generators::GeneratorState::Active
            } else {
                crate::generators::GeneratorState::Inactive
            };
        }
    }
    if let Some(index) = request.normal_box {
        world.resource_mut::<crate::stations::Stations>().normal_box = index;
    }
    Ok(json!({"gameplay_evidence":false}))
}

pub(super) fn snapshot(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    Ok(crate::stations::snapshot(
        world.resource::<crate::stations::Stations>(),
    ))
}
