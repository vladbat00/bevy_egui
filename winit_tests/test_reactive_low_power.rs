use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_input::mouse::MouseMotion;
use bevy_winit::{UpdateMode, WinitSettings};
use std::time::Duration;

const WAIT_DURATION: Duration = Duration::from_millis(200);

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_example_system(mut contexts: EguiContexts) -> Result {
    egui::Window::new("TEST").show(contexts.ctx_mut()?, |ui| {
        ui.heading("STOP MOUSE MOVEMENT AND ANY OTHER INPUT TO LET THE TEST COMPLETE");
    });
    Ok(())
}

fn test_reactive_mode(
    time: Res<Time<Real>>,
    mut frame_counter: Local<usize>,
    mut mouse_motion_reader: MessageReader<MouseMotion>,
    mut app_exit_writer: MessageWriter<AppExit>,
) {
    if !mouse_motion_reader.is_empty() {
        *frame_counter = 0;
    }
    mouse_motion_reader.clear();

    if *frame_counter > 10 {
        assert!(
            crate::WAIT_DURATION.abs_diff(time.delta()) < Duration::from_millis(10),
            "delta time doesn't match the low power settings wait time: {:?}",
            time.delta()
        );
        app_exit_writer.write(AppExit::Success);
    }

    if time.elapsed_secs() > 10.0 {
        panic!("test timeout");
    }

    *frame_counter += 1;
}

fn main() {
    App::new()
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::reactive_low_power(WAIT_DURATION),
            unfocused_mode: UpdateMode::reactive_low_power(WAIT_DURATION),
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_camera_system)
        .add_systems(Update, test_reactive_mode)
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .run();
}
