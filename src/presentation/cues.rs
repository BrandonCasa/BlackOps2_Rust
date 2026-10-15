use crate::GameState;
use bevy::prelude::*;
#[derive(Component, Default)]
pub(crate) struct CueParameters {
    pub script_vectors: [[f32; 4]; 4],
    pub script_available: u8,
    pub luminous_channel: Option<(usize, usize)>,
}
#[derive(Resource, Default)]
pub(crate) struct CueClock {
    running: bool,
    realtime: std::time::Duration,
    remainder: std::time::Duration,
    server: i32,
    snapshot: Option<i32>,
    render: Option<i32>,
    delta: i32,
    interpolation: i32,
    extrapolated: bool,
    paused: bool,
}

impl CueClock {
    pub(crate) fn render_milliseconds(&self) -> Option<i32> {
        self.render
    }

    pub(crate) fn server_milliseconds(&self) -> Option<i32> {
        self.running.then_some(self.server)
    }

    fn advance(
        &mut self,
        duration: std::time::Duration,
        paused: bool,
        scale: f32,
    ) -> Result<(), String> {
        if !self.running {
            return Ok(());
        }
        self.realtime += duration;
        let realtime = i32::try_from(self.realtime.as_millis())
            .map_err(|_| "Native client clock requires full server restart")?;
        if paused {
            self.paused = true;
            return Ok(());
        }
        self.remainder += duration;
        let ticks = self.remainder.as_millis() / 50;
        let previous = self.snapshot.unwrap_or(0);
        if ticks != 0 {
            let advance =
                i32::try_from(ticks * 50).map_err(|_| "Native server timestep overflow")?;
            self.server = self
                .server
                .checked_add(advance)
                .ok_or("Native server clock overflow")?;
            self.remainder -= std::time::Duration::from_millis(advance as u64);
            self.snapshot = Some(self.server);
        }
        let Some(snapshot) = self.snapshot else {
            return Ok(());
        };
        if self.render.is_none() || self.paused {
            self.render = Some(snapshot);
            self.delta = snapshot - realtime;
            self.extrapolated = false;
            if self.paused {
                self.interpolation = 40;
            }
            self.paused = false;
        }
        let candidate = self.delta + realtime;
        self.render = Some(candidate.max(self.render.unwrap()));
        self.extrapolated |= snapshot - self.interpolation <= candidate;
        if ticks == 0 {
            return Ok(());
        }
        self.interpolation = if self.interpolation < 11 {
            5
        } else {
            self.interpolation - 5
        };
        let target = snapshot - realtime - self.interpolation;
        let interval = if ticks > 1 { 50 } else { snapshot - previous };
        let desired = target - interval;
        let error = desired - self.delta;
        let magnitude = if error <= 0 {
            -error
        } else if interval as f32 <= scale * 1000. {
            (error - interval).max(0)
        } else {
            error
        };
        if magnitude as f32 > scale * 1000. {
            self.render = Some(snapshot);
            self.delta = target;
        } else if magnitude as f32 > scale * 100. {
            self.delta = (self.delta + desired) >> 1;
        } else if self.extrapolated {
            self.extrapolated = false;
            self.delta -= 2;
        } else {
            self.delta += (desired - self.delta).signum();
        }
        Ok(())
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<CueClock>()
        .add_systems(OnEnter(GameState::StartingSession), begin)
        .add_systems(OnEnter(GameState::MainMenu), reset)
        .add_systems(Update, advance)
        .add_systems(PostUpdate, (bind_materials, present).chain());
}
fn begin(mut clock: ResMut<CueClock>) {
    *clock = default();
    clock.running = true;
}
fn reset(mut clock: ResMut<CueClock>) {
    *clock = default();
}
pub(crate) fn advance(
    time: Res<Time>,
    virtual_time: Res<Time<Virtual>>,
    state: Res<State<GameState>>,
    mut clock: ResMut<CueClock>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    if let Err(error) = clock.advance(
        time.delta(),
        **state == GameState::Paused,
        virtual_time.relative_speed(),
    ) {
        failure.0 = error;
        next.set(GameState::LoadError);
    }
}

#[derive(Component)]
struct CueMaterial {
    owner: Entity,
    source: StandardMaterial,
    target: Handle<StandardMaterial>,
    eyes: bool,
}
fn bind_materials(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            Option<&bevy::gltf::GltfMaterialName>,
        ),
        Without<CueMaterial>,
    >,
    owners: Query<(), With<CueParameters>>,
    parents: Query<&ChildOf>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, handle, name) in &surfaces {
        let mut ancestor = entity;
        let owner = loop {
            if owners.contains(ancestor) {
                break Some(ancestor);
            };
            let Ok(parent) = parents.get(ancestor) else {
                break None;
            };
            ancestor = parent.parent();
        };
        let Some(owner) = owner else { continue };
        let Some(mut source) = materials.get(&handle.0).cloned() else {
            continue;
        };
        let eyes = name.is_some_and(|name| name.0.to_ascii_lowercase().contains("eye"));
        if !eyes && source.emissive == LinearRgba::BLACK {
            continue;
        }
        if eyes && source.emissive == LinearRgba::BLACK {
            source.emissive = source.base_color.to_linear();
            source.emissive_texture = source.base_color_texture.clone();
        }
        let target = materials.add(source.clone());
        commands.entity(entity).insert((
            MeshMaterial3d(target.clone()),
            CueMaterial {
                owner,
                source,
                target,
                eyes,
            },
        ));
    }
}
fn present(
    surfaces: Query<&CueMaterial>,
    owners: Query<&CueParameters>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for surface in &surfaces {
        let Ok(cue) = owners.get(surface.owner) else {
            continue;
        };
        let Some(mut material) = materials.get_mut(&surface.target) else {
            continue;
        };
        material.emissive = surface.source.emissive;
        let Some((slot, component)) = cue.luminous_channel else {
            continue;
        };
        if cue.script_available & (1 << slot) == 0 || (slot == 2 && !surface.eyes) {
            continue;
        }
        let value = cue.script_vectors[slot][component].max(0.);
        material.emissive = surface.source.emissive * if slot == 2 { value * value } else { value };
    }
}
