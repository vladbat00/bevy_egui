use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{PresentMode, WindowRef, WindowResolution},
};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_egui::{
    EguiContext, EguiGlobalSettings, EguiMultipassSchedule, EguiPlugin, EguiPrimaryContextPass,
    EguiTextureHandle, EguiUserTextures, PrimaryEguiContext,
};

#[derive(Resource)]
struct Images {
    bevy_icon: Handle<Image>,
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SecondWindowContextPass;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .init_resource::<SharedUiState>()
        .add_systems(Startup, load_assets_system)
        .add_systems(Startup, create_new_window_system)
        .add_systems(EguiPrimaryContextPass, ui_first_window_system)
        .add_systems(SecondWindowContextPass, ui_second_window_system);

    app.run();
}

fn create_new_window_system(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
) {
    // Disable the automatic creation of a primary context to set it up manually.
    egui_global_settings.auto_create_primary_context = false;

    // Spawn a camera for the primary window.
    commands.spawn((Camera3d::default(), PrimaryEguiContext));

    // Spawn the second window and its camera.
    let second_window_id = commands
        .spawn(Window {
            title: "Second window".to_owned(),
            resolution: WindowResolution::new(800, 600),
            present_mode: PresentMode::AutoVsync,
            ..Default::default()
        })
        .id();
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(second_window_id)),
            ..Default::default()
        },
        EguiMultipassSchedule::new(SecondWindowContextPass),
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
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut ui_state: Local<UiState>,
    mut shared_ui_state: ResMut<SharedUiState>,
    images: Res<Images>,
    mut egui_ctx: Single<&mut EguiContext, With<PrimaryEguiContext>>,
) {
    let bevy_texture_id =
        egui_user_textures.add_image(EguiTextureHandle::Weak(images.bevy_icon.id()));
    egui::Window::new("First Window")
        .vscroll(true)
        .show(egui_ctx.get_mut(), |ui| {
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
    mut egui_user_textures: ResMut<EguiUserTextures>,
    mut ui_state: Local<UiState>,
    mut shared_ui_state: ResMut<SharedUiState>,
    images: Res<Images>,
    mut egui_ctx: Single<&mut EguiContext, Without<PrimaryEguiContext>>,
) {
    let bevy_texture_id =
        egui_user_textures.add_image(EguiTextureHandle::Weak(images.bevy_icon.id()));
    egui::Window::new("Second Window")
        .vscroll(true)
        .show(egui_ctx.get_mut(), |ui| {
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
