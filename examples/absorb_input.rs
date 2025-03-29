use bevy::{
    color::palettes::{basic::PURPLE, css::YELLOW},
    prelude::*,
};
use bevy_egui::{egui, input::egui_wants_input, EguiContexts, EguiGlobalSettings, EguiPlugin};
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup_scene_system)
        .add_systems(Update, ui_system)
        // You can wrap your systems with the `egui_wants_input` run condition if you
        // want to disable them while Egui is using input.
        //
        // As an alternative (a less safe one), you can set `EguiGlobalSettings::enable_absorb_bevy_input_system`
        // to true to let Egui absorb all input events (see `ui_system` for the usage example).
        .add_systems(Update, input_system.run_if(not(egui_wants_input)))
        .run();
}

#[derive(Resource, Clone)]
struct Materials {
    yellow: MeshMaterial2d<ColorMaterial>,
    purple: MeshMaterial2d<ColorMaterial>,
}

fn setup_scene_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    let materials = Materials {
        yellow: MeshMaterial2d(color_materials.add(Color::from(YELLOW))),
        purple: MeshMaterial2d(color_materials.add(Color::from(PURPLE))),
    };
    commands.insert_resource(materials.clone());

    commands.spawn(Camera2d);
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        materials.purple,
        Transform::default().with_scale(Vec3::splat(128.)),
    ));
}

struct LoremIpsum(String);

impl Default for LoremIpsum {
    fn default() -> Self {
        Self("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string())
    }
}

#[derive(Default)]
struct LastEvents {
    keyboard_input: Option<KeyboardInput>,
    mouse_button_input: Option<MouseButtonInput>,
    mouse_wheel: Option<MouseWheel>,
}

fn ui_system(
    mut contexts: EguiContexts,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut text: Local<LoremIpsum>,
    mut last_events: Local<LastEvents>,
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
) {
    if let Some(ev) = keyboard_input_events.read().last() {
        last_events.keyboard_input = Some(ev.clone());
    }
    if let Some(ev) = mouse_button_input_events.read().last() {
        last_events.mouse_button_input = Some(*ev);
    }
    if let Some(ev) = mouse_wheel_events.read().last() {
        last_events.mouse_wheel = Some(*ev);
    }

    egui::Window::new("Absorb Input")
        .max_size([300.0, 200.0])
        .vscroll(true)
        .show(contexts.ctx_mut(), |ui| {
            ui.checkbox(
                &mut egui_global_settings.enable_absorb_bevy_input_system,
                "Absorb all input events",
            );

            ui.separator();

            ui.label(format!(
                "Last keyboard button event: {:?}",
                last_events.keyboard_input
            ));
            ui.label(format!(
                "Last mouse button event: {:?}",
                last_events.mouse_button_input
            ));
            ui.label(format!(
                "Last mouse wheel event: {:?}",
                last_events.mouse_wheel
            ));

            ui.separator();

            ui.label("A text field to test absorbing keyboard events");
            ui.text_edit_multiline(&mut text.0);
        });
}

fn input_system(
    materials: Res<Materials>,
    mesh: Single<(&mut Transform, &mut MeshMaterial2d<ColorMaterial>), Without<Camera2d>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_button_input: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_event_reader: EventReader<MouseWheel>,
) {
    let (mut transform, mut material) = mesh.into_inner();

    if mouse_button_input.just_pressed(MouseButton::Left) {
        *material = if materials.yellow.0 == material.0 {
            materials.purple.clone()
        } else {
            materials.yellow.clone()
        }
    }

    if keyboard_button_input.pressed(KeyCode::KeyA) {
        transform.translation.x -= 5.0;
    }
    if keyboard_button_input.pressed(KeyCode::KeyD) {
        transform.translation.x += 5.0;
    }
    if keyboard_button_input.pressed(KeyCode::KeyS) {
        transform.translation.y -= 5.0;
    }
    if keyboard_button_input.pressed(KeyCode::KeyW) {
        transform.translation.y += 5.0;
    }

    for ev in mouse_wheel_event_reader.read() {
        transform.scale += ev.y;
    }
}
