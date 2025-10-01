use bevy::{camera::Viewport, ecs::schedule::ScheduleLabel, prelude::*, window::WindowResized};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiMultipassSchedule, EguiPlugin,
    EguiPrimaryContextPass, PrimaryEguiContext, egui,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            // You may want this set to `true` if you need virtual keyboard work in mobile browsers.
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }))
    .insert_resource(PlayersCount(2))
    .add_plugins(EguiPlugin::default())
    .add_systems(Startup, setup_system)
    .add_systems(Update, update_camera_viewports_system)
    .add_systems(EguiPrimaryContextPass, players_count_ui_system);
    register_ui_systems_for_player::<0>(&mut app);
    register_ui_systems_for_player::<1>(&mut app);
    register_ui_systems_for_player::<2>(&mut app);
    register_ui_systems_for_player::<3>(&mut app);
    app.run();
}

fn register_ui_systems_for_player<const N: u8>(app: &mut App) {
    app.add_systems(PlayerCamera::<N>, ui_example_system::<N>);
}

#[derive(Component, ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct PlayerCamera<const N: u8>;

#[derive(Resource)]
struct PlayersCount(u8);

fn setup_system(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Disable the automatic creation of a primary context to set it up manually for every camera.
    egui_global_settings.auto_create_primary_context = false;

    // Circular base.
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // Cube.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // Light.
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Cameras.
    commands.spawn((
        EguiMultipassSchedule(PlayerCamera::<0>.intern()),
        PlayerCamera::<0>,
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        EguiMultipassSchedule(PlayerCamera::<1>.intern()),
        PlayerCamera::<1>,
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        EguiMultipassSchedule(PlayerCamera::<2>.intern()),
        PlayerCamera::<2>,
        Camera3d::default(),
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        EguiMultipassSchedule(PlayerCamera::<3>.intern()),
        PlayerCamera::<3>,
        Camera3d::default(),
        Camera {
            order: 3,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PrimaryEguiContext,
        Camera3d::default(),
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

fn camera_position_and_size(index: u8, count: u32, window_size: UVec2) -> (UVec2, UVec2) {
    match index {
        0 => (
            UVec2::new(0, 0),
            UVec2::new(
                window_size.x / count.min(2),
                window_size.y / count.div_ceil(2),
            ),
        ),
        1 => (
            UVec2::new(window_size.x / 2, 0),
            UVec2::new(window_size.x / 2, window_size.y / count.div_ceil(2)),
        ),
        2 => (
            UVec2::new(0, window_size.y / 2),
            UVec2::new(window_size.x / (count / 2), window_size.y / 2),
        ),
        3 => (window_size / 2, window_size / 2),
        _ => unreachable!(),
    }
}

#[allow(clippy::type_complexity)]
fn update_camera_viewports_system(
    players_count: Res<PlayersCount>,
    window: Single<&Window>,
    mut window_resized_reader: MessageReader<WindowResized>,
    mut query: Query<(
        &mut Camera,
        AnyOf<(
            &PlayerCamera<0>,
            &PlayerCamera<1>,
            &PlayerCamera<2>,
            &PlayerCamera<3>,
        )>,
    )>,
) -> Result {
    // We need to dynamically resize the camera's viewports whenever the window size changes.
    // A resize_event is sent when the window is first created, allowing us to reuse this system for initial setup.
    if window_resized_reader.is_empty() && !players_count.is_changed() {
        return Ok(());
    }
    window_resized_reader.clear();

    let mut result: Vec<_> = query.iter_mut().collect();

    for (camera, _) in &mut result {
        camera.is_active = (camera.order as u8) < players_count.0;
        if !camera.is_active {
            continue;
        }

        let (physical_position, physical_size) = camera_position_and_size(
            camera.order as u8,
            players_count.0 as u32,
            window.physical_size(),
        );

        camera.viewport = Some(Viewport {
            physical_position,
            physical_size,
            ..default()
        });
    }

    Ok(())
}

fn players_count_ui_system(
    mut egui_contexts: EguiContexts,
    mut players_count: ResMut<PlayersCount>,
) -> Result {
    egui::Window::new("")
        .fixed_size([200.0, 30.0])
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(egui_contexts.ctx_mut()?, |ui| {
            ui.horizontal(|ui| {
                if ui.button("-").clicked() {
                    players_count.0 = (players_count.0 - 1).max(1);
                }
                ui.label(format!("Player count: {}", players_count.0));
                if ui.button("+").clicked() {
                    players_count.0 = (players_count.0 + 1).min(4);
                }
            })
        });
    Ok(())
}

fn ui_example_system<const N: u8>(
    mut input: Local<String>,
    mut context: Single<&mut EguiContext, With<PlayerCamera<N>>>,
) {
    egui::Window::new("Hello").show(context.get_mut(), |ui| {
        ui.label(format!("Player {N}"));
        ui.horizontal(|ui| {
            ui.label("Write something: ");
            ui.text_edit_singleline(&mut *input);
        });
    });
}
