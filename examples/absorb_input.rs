use bevy::{
    color::palettes::{basic::PURPLE, css::YELLOW},
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
    },
    prelude::*,
};
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, egui,
    input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_scene_system)
        .add_systems(EguiPrimaryContextPass, ui_system)
        // You can wrap your systems with the `egui_wants_any_pointer_input`, `egui_wants_any_keyboard_input` run conditions if you
        // want to disable them while Egui is using input.
        //
        // As an alternative (a less safe one), you can set `EguiGlobalSettings::enable_absorb_bevy_input_system`
        // to true to let Egui absorb all input events (see `ui_system` for the usage example).
        .add_systems(
            Update,
            keyboard_input_system.run_if(not(egui_wants_any_keyboard_input)),
        )
        .add_systems(
            Update,
            pointer_input_system.run_if(not(egui_wants_any_pointer_input)),
        )
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
struct LastMessages {
    keyboard_input: Option<KeyboardInput>,
    mouse_button_input: Option<MouseButtonInput>,
    mouse_wheel: Option<MouseWheel>,
}

#[allow(clippy::too_many_arguments)]
fn ui_system(
    mut contexts: EguiContexts,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut text: Local<LoremIpsum>,
    mut last_messages: Local<LastMessages>,
    mut keyboard_input_reader: MessageReader<KeyboardInput>,
    mut mouse_button_input_reader: MessageReader<MouseButtonInput>,
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
) -> Result {
    if let Some(message) = keyboard_input_reader.read().last() {
        last_messages.keyboard_input = Some(message.clone());
    }
    if let Some(message) = mouse_button_input_reader.read().last() {
        last_messages.mouse_button_input = Some(*message);
    }
    if let Some(message) = mouse_wheel_reader.read().last() {
        last_messages.mouse_wheel = Some(*message);
    }

    egui::Window::new("Absorb Input")
        .max_size([300.0, 200.0])
        .vscroll(true)
        .show(contexts.ctx_mut()?, |ui| {
            ui.checkbox(
                &mut egui_global_settings.enable_absorb_bevy_input_system,
                "Absorb all input messages",
            );

            ui.separator();

            ui.label(format!(
                "Last keyboard button message: {:?}",
                last_messages.keyboard_input
            ));
            ui.label(format!(
                "Last mouse button message: {:?}",
                last_messages.mouse_button_input
            ));
            ui.label(format!(
                "Last mouse wheel message: {:?}",
                last_messages.mouse_wheel
            ));

            ui.separator();

            ui.label("A text field to test absorbing keyboard inputs");
            ui.text_edit_multiline(&mut text.0);
        });

    Ok(())
}

fn keyboard_input_system(
    mesh: Single<&mut Transform, Without<Camera2d>>,
    keyboard_button_input: Res<ButtonInput<KeyCode>>,
) {
    let mut transform = mesh.into_inner();

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
}

fn pointer_input_system(
    materials: Res<Materials>,
    mesh: Single<(&mut Transform, &mut MeshMaterial2d<ColorMaterial>), Without<Camera2d>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
) {
    let (mut transform, mut material) = mesh.into_inner();

    if mouse_button_input.just_pressed(MouseButton::Left) {
        *material = if materials.yellow.0 == material.0 {
            materials.purple.clone()
        } else {
            materials.yellow.clone()
        }
    }

    for message in mouse_wheel_reader.read() {
        transform.scale += message.y;
    }
}
