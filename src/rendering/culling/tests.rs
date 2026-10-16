use super::*;

fn box_at(center: Vec3, radius: f32) -> Bounds {
    [
        (center - Vec3::splat(radius)).to_array(),
        (center + Vec3::splat(radius)).to_array(),
    ]
}

fn clip(eye: Vec3, direction: Vec3) -> Mat4 {
    bevy::math::proj::perspective_infinite_reverse(90_f32.to_radians(), 1., 4.)
        * Transform::from_translation(eye)
            .looking_to(direction, Vec3::Z)
            .to_matrix()
            .inverse()
}

fn rooms() -> VisibilityWorld {
    let cell = |center, surface| Cell {
        bounds: box_at(center, 10.),
        trees: vec![Tree {
            bounds: box_at(center, 10.),
            children: [0, 0],
            surfaces: [surface, 1],
            models: vec![surface as u16],
        }],
        portals: vec![],
    };
    VisibilityWorld {
        surface_count: 2,
        model_count: 2,
        // Tree indices address the source sorting table, not surfaces directly.
        sorted_surfaces: vec![1, 0],
        surface_bounds: vec![
            box_at(Vec3::new(30., 0., 0.), 1.),
            box_at(Vec3::new(10., 0., 0.), 1.),
        ],
        model_bounds: vec![
            box_at(Vec3::new(10., 0., 0.), 1.),
            box_at(Vec3::new(30., 0., 0.), 1.),
        ],
        cells: vec![
            cell(Vec3::new(10., 0., 0.), 0),
            cell(Vec3::new(30., 0., 0.), 1),
        ],
        occluders: vec![],
    }
}

fn portal(cell: usize, x: f32) -> Portal {
    Portal {
        cell,
        plane: Vec4::new(1., 0., 0., -x),
        vertices: vec![
            Vec3::new(x, -2., -2.),
            Vec3::new(x, 2., -2.),
            Vec3::new(x, 2., 2.),
            Vec3::new(x, -2., 2.),
        ],
        hull_axes: [Vec3::Y, Vec3::Z],
        bounds: [[x, -2., -2.], [x, 2., 2.]],
    }
}

#[test]
fn closed_cells_sorted_surface_indices_and_outside_world() {
    let world = rooms();
    world.validate();
    let mut vis = SourceVisibility::default();
    vis.update(
        &world,
        Some(0),
        Vec3::ZERO,
        clip(Vec3::ZERO, Vec3::X),
        Vec3::X,
    );
    assert_eq!(vis.surfaces, [false, true]);
    assert_eq!(vis.models, [true, false]);
    assert!(!vis.dynamic_visible(&world, &box_at(Vec3::new(30., 0., 0.), 1.)));
    assert!(vis.dynamic_visible(&world, &box_at(Vec3::new(20., 0., 0.), 1.)));
    assert!(vis.dynamic_visible(&world, &box_at(Vec3::new(100., 0., 0.), 1.)));
    assert!(!vis.dynamic_visible(&world, &box_at(Vec3::new(-100., 0., 0.), 1.)));
    assert!(!vis.dynamic_visible(&world, &box_at(Vec3::new(CULL_DISTANCE + 2., 0., 0.), 1.)));
    vis.update(&world, None, Vec3::ZERO, clip(Vec3::ZERO, Vec3::X), Vec3::X);
    assert_eq!(vis.surfaces, [true, true]);
    assert_eq!(vis.models, [true, true]);
}

#[test]
fn clipped_portal_rejects_hidden_geometry_and_keeps_doorway_view() {
    let mut world = rooms();
    world.cells[0].portals.push(portal(1, 20.));
    let mut vis = SourceVisibility::default();
    vis.update(
        &world,
        Some(0),
        Vec3::ZERO,
        clip(Vec3::ZERO, Vec3::X),
        Vec3::X,
    );
    assert_eq!(vis.surfaces, [true, true]);
    world.surface_bounds[0] = box_at(Vec3::new(30., 9., 0.), 1.);
    world.model_bounds[1] = world.surface_bounds[0];
    vis.update(
        &world,
        Some(0),
        Vec3::ZERO,
        clip(Vec3::ZERO, Vec3::X),
        Vec3::X,
    );
    assert_eq!(vis.surfaces, [false, true]);
    assert_eq!(vis.models, [true, false]);
    let eye = Vec3::new(20., 0., 0.);
    vis.update(&world, Some(0), eye, clip(eye, Vec3::X), Vec3::X);
    assert!(vis.surfaces[0], "near-coplanar doorway must not disappear");
}

#[test]
fn authored_occluder_culls_only_fully_contained_bounds() {
    let mut world = rooms();
    world.occluders.push(Occluder {
        flags: 1,
        points: portal(0, 5.).vertices.try_into().unwrap(),
    });
    let mut vis = SourceVisibility::default();
    vis.update(&world, None, Vec3::ZERO, clip(Vec3::ZERO, Vec3::X), Vec3::X);
    assert!(vis.occluded(&box_at(Vec3::new(10., 0., 0.), 1.)));
    assert!(!vis.occluded(&box_at(Vec3::new(3., 0., 0.), 1.)));
    assert!(!vis.occluded(&box_at(Vec3::new(10., 4., 0.), 1.)));
    world.occluders[0].flags = 0;
    vis.update(&world, None, Vec3::ZERO, clip(Vec3::ZERO, Vec3::X), Vec3::X);
    assert!(!vis.occluded(&box_at(Vec3::new(10., 0., 0.), 1.)));
}

#[test]
fn admission_filters_only_the_game_camera_and_keeps_viewmodel_override() {
    let world = rooms();
    let mut source = SourceVisibility::default();
    source.update(
        &world,
        Some(0),
        Vec3::ZERO,
        clip(Vec3::ZERO, Vec3::X),
        Vec3::X,
    );
    let mut app = App::new();
    app.insert_resource(world)
        .insert_resource(source)
        .add_systems(Update, filter_visibility);
    let hidden = app
        .world_mut()
        .spawn((GlobalTransform::IDENTITY, BspSurface(0)))
        .id();
    let visible = app
        .world_mut()
        .spawn((GlobalTransform::IDENTITY, BspSurface(1)))
        .id();
    let viewmodel = app
        .world_mut()
        .spawn((GlobalTransform::IDENTITY, BspSurface(0), NoFrustumCulling))
        .id();
    let class = TypeId::of::<super::super::native::NativeWorldSurface>();
    let mut entities = VisibleEntities::default();
    entities.get_mut(class).extend([hidden, visible, viewmodel]);
    let shadow = app.world_mut().spawn(entities.clone()).id();
    let camera = app
        .world_mut()
        .spawn((super::super::GameCamera, entities))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<VisibleEntities>(camera)
            .unwrap()
            .get(class),
        [visible, viewmodel]
    );
    assert_eq!(
        app.world()
            .get::<VisibleEntities>(shadow)
            .unwrap()
            .get(class),
        [hidden, visible, viewmodel]
    );
}

#[test]
fn returning_surface_receives_current_eye_relative_constants() {
    use super::super::world::{WorldSurfaceLighting, object_constants, source_frame};
    let mut app = App::new();
    app.add_systems(Update, object_constants);
    let surface = app
        .world_mut()
        .spawn((
            GlobalTransform::IDENTITY,
            WorldSurfaceLighting([Vec4::ZERO; 3]),
        ))
        .id();
    let class = TypeId::of::<super::super::native::NativeWorldSurface>();
    let mut visible = VisibleEntities::default();
    visible.push(surface, class);
    let camera = app
        .world_mut()
        .spawn((super::super::GameCamera, GlobalTransform::IDENTITY, visible))
        .id();
    app.update();
    let original = *app
        .world()
        .get::<super::super::native::NativeWorldObject>(surface)
        .unwrap();
    app.world_mut()
        .get_mut::<VisibleEntities>(camera)
        .unwrap()
        .clear(class);
    app.world_mut()
        .entity_mut(camera)
        .insert(GlobalTransform::from(Transform::from_xyz(2., 0., 0.)));
    app.update();
    assert!(
        *app.world()
            .get::<super::super::native::NativeWorldObject>(surface)
            .unwrap()
            == original
    );
    app.update();
    app.world_mut()
        .get_mut::<VisibleEntities>(camera)
        .unwrap()
        .push(surface, class);
    app.update();
    let current = app
        .world()
        .get::<super::super::native::NativeWorldObject>(surface)
        .unwrap();
    let frame = source_frame().inverse();
    let eye = frame.transform_point3(Vec3::new(2., 0., 0.));
    let expected = Mat4::from_translation(-eye) * frame;
    assert_eq!(current.world_rows, std::array::from_fn(|i| expected.row(i)));
    assert_ne!(current.world_rows, original.world_rows);
}

#[test]
fn source_lod_uses_strict_thresholds_and_raw_distance_fallback() {
    let selected = |adjusted, raw| {
        super::super::models::source_lod([100., 200., 300.].into_iter(), adjusted, raw)
    };
    assert_eq!(selected(100., 100.), Some(1));
    assert_eq!(selected(300., 250.), Some(2));
    assert_eq!(selected(300., 300.), None);
}

#[test]
fn source_clip_epsilon_and_multiple_portal_hull_paths() {
    let mut output = vec![];
    assert!(!clip_polygon(
        &[Vec3::ZERO, Vec3::Y, Vec3::Z],
        Vec4::X,
        &mut output
    ));
    assert!(clip_polygon(
        &[
            Vec3::new(1., 0., 0.),
            Vec3::new(-1., 1., 0.),
            Vec3::new(1., 1., 1.)
        ],
        Vec4::X,
        &mut output
    ));
    assert!(output.iter().any(|p| (p.x - 0.001).abs() < 0.00001));
    let mut points = vec![
        Vec2::ZERO,
        Vec2::X,
        Vec2::Y,
        Vec2::ONE,
        Vec2::splat(0.5),
        Vec2::ZERO,
    ];
    let mut convex = vec![];
    hull(&mut points, &mut convex);
    assert_eq!(convex, [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]);
}

#[test]
#[ignore = "requires the authorized local Origins export"]
fn exported_origins_visibility_sweep() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let bsp: super::super::reflection::ReflectionWorld =
        serde_json::from_slice(&std::fs::read(root.join("world.json")).unwrap()).unwrap();
    bsp.validate();
    let world = VisibilityWorld::load(&root, 6469, 14480, bsp.cell_count());
    let game: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("game.json")).unwrap()).unwrap();
    let mut vis = SourceVisibility::default();
    let mut times = vec![];
    for position in game["spawn_points"]
        .as_array()
        .unwrap()
        .iter()
        .chain(std::iter::once(&game["spawn"]))
        .chain(game["survey_views"].as_array().unwrap().iter())
    {
        let p = position.as_array().unwrap();
        let metric = Vec3::new(
            p[0].as_f64().unwrap() as f32,
            p[1].as_f64().unwrap() as f32 + 1.65,
            p[2].as_f64().unwrap() as f32,
        );
        let eye = super::super::world::source_frame()
            .inverse()
            .transform_point3(metric);
        for step in 0..12 {
            let angle = step as f32 * std::f32::consts::TAU / 12.;
            let direction = Vec3::new(angle.cos(), angle.sin(), 0.);
            let clip = clip(eye, direction);
            let started = std::time::Instant::now();
            vis.update(&world, bsp.cell(eye.to_array()), eye, clip, direction);
            times.push(started.elapsed().as_secs_f64() * 1000.);
            let surfaces = vis.surfaces.iter().filter(|v| **v).count();
            let models = vis.models.iter().filter(|v| **v).count();
            let frustum_surfaces = world
                .surface_bounds
                .iter()
                .filter(|b| vis.base.intersects(b))
                .count();
            let frustum_models = world
                .model_bounds
                .iter()
                .filter(|b| vis.base.intersects(b))
                .count();
            assert!(surfaces <= frustum_surfaces && models <= frustum_models);
            println!(
                "cell={:?} yaw={step} surfaces={surfaces}/{frustum_surfaces} models={models}/{frustum_models} tree_visits={} portal_visits={}",
                bsp.cell(eye.to_array()),
                vis.tree_visits,
                vis.portal_visits
            );
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{} source visibility samples: median {:.3}ms, p95 {:.3}ms, max {:.3}ms (CPU sweep, not game FPS)",
        times.len(),
        times[times.len() / 2],
        times[times.len() * 95 / 100],
        times[times.len() - 1]
    );
}
