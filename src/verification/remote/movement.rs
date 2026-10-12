use super::*;
use crate::movement::physics::SurfaceFlags;

#[derive(Resource, Default)]
struct OriginalSurfaces(Vec<(Entity, u32)>);

pub(super) fn surfaces(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        flags: Option<u32>,
    }
    let request: Request = parse(value)?;
    if let Some(flags) = request.flags {
        if world.get_resource::<OriginalSurfaces>().is_none() {
            let original = world
                .query::<(Entity, &SurfaceFlags)>()
                .iter(world)
                .map(|(entity, flags)| (entity, flags.0))
                .collect();
            world.insert_resource(OriginalSurfaces(original));
        }
        for mut surface in world.query::<&mut SurfaceFlags>().iter_mut(world) {
            surface.0 = flags;
        }
    } else if let Some(original) = world.remove_resource::<OriginalSurfaces>() {
        for (entity, flags) in original.0 {
            if let Some(mut surface) = world.get_mut::<SurfaceFlags>(entity) {
                surface.0 = flags;
            }
        }
    }
    let surfaces: Vec<_> = world
        .query::<&SurfaceFlags>()
        .iter(world)
        .map(|surface| surface.0)
        .collect();
    Ok(
        json!({"setup_applied":request.flags.is_some(),"gameplay_evidence":false,
        "colliders":surfaces.len(),"slick_colliders":surfaces.iter().filter(|flags| **flags & 2 != 0).count(),
        "flags":request.flags}),
    )
}
