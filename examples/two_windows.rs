use bevy::{
    prelude::*,
    render::camera::RenderTarget,
    window::{PresentMode, WindowRef, WindowResolution},
};
use bevy_egui::{BevyEguiApp, EguiContexts, EguiPlugin, OnEguiPass};

#[derive(Resource)]
struct Images {
    bevy_icon: Handle<Image>,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            default_to_multipass: true,
        })
        .init_resource::<SharedUiState>()
        .add_systems(Startup, load_assets_system)
        .add_systems(Startup, create_new_window_system)
        .add_egui_system(ui_first_window_system);

    app.run();
}

fn create_new_window_system(mut commands: Commands) {
    // Spawn a second window
    let second_window_id = commands
        .spawn(Window {
            title: "Second window".to_owned(),
            resolution: WindowResolution::new(800.0, 600.0),
            present_mode: PresentMode::AutoVsync,
            ..Default::default()
        })
        .observe(ui_second_window_system)
        .id();

    // second window camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(second_window_id)),
            ..Default::default()
        },
        Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn load_assets_system(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Images {
        bevy_icon: assets.load("icon.png"),
    });
}

#[derive(Default)]
struct UiState {
    input: String,
}

#[derive(Default, Resource)]
struct SharedUiState {
    shared_input: String,
}

fn ui_first_window_system(
    trigger: Trigger<OnEguiPass>,
    mut ui_state: Local<UiState>,
    mut shared_ui_state: ResMut<SharedUiState>,
    images: Res<Images>,
    mut contexts: EguiContexts,
) {
    let bevy_texture_id = contexts.add_image(images.bevy_icon.clone_weak());
    let Some(ctx) = contexts.try_ctx_for_entity_mut(trigger.entity()) else {
        return;
    };
    egui::Window::new("First Window")
        .vscroll(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut ui_state.input);
            });
            ui.horizontal(|ui| {
                ui.label("Shared input: ");
                ui.text_edit_singleline(&mut shared_ui_state.shared_input);
            });

            ui.add(egui::widgets::Image::new(egui::load::SizedTexture::new(
                bevy_texture_id,
                [256.0, 256.0],
            )));
        });
}

fn ui_second_window_system(
    trigger: Trigger<OnEguiPass>,
    mut ui_state: Local<UiState>,
    mut shared_ui_state: ResMut<SharedUiState>,
    images: Res<Images>,
    mut contexts: EguiContexts,
) {
    let bevy_texture_id = contexts.add_image(images.bevy_icon.clone_weak());
    let Some(ctx) = contexts.try_ctx_for_entity_mut(trigger.entity()) else {
        return;
    };
    egui::Window::new("Second Window")
        .vscroll(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Write something else: ");
                ui.text_edit_singleline(&mut ui_state.input);
            });
            ui.horizontal(|ui| {
                ui.label("Shared input: ");
                ui.text_edit_singleline(&mut shared_ui_state.shared_input);
            });

            ui.add(egui::widgets::Image::new(egui::load::SizedTexture::new(
                bevy_texture_id,
                [256.0, 256.0],
            )));
        });
}
