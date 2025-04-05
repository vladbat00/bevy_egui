use bevy::prelude::*;
use bevy_egui::{egui, EguiContextPass, EguiContexts, EguiPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        // Systems that create Egui widgets should be run during the `CoreSet::Update` set,
        // or after the `EguiPreUpdateSet::BeginPass` system (which belongs to the `CoreSet::PreUpdate` set).
        .add_systems(EguiContextPass, ui_example_system)
        .run();
}

fn ui_example_system(mut contexts: EguiContexts) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");
    });
}
