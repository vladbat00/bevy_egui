use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_camera_system)
        .add_systems(EguiPrimaryContextPass, foo::ui_system)
        .run();
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
mod foo {
    use bevy::{
        prelude::*,
        tasks::{AsyncComputeTaskPool, Task, block_on, poll_once},
    };
    use bevy_egui::{EguiContexts, egui};

    #[derive(Default)]
    pub struct MyState {
        dropped_files: Vec<egui::DroppedFile>,
        picked_path: Option<String>,
    }

    type DialogResponse = Option<rfd::FileHandle>;

    // much of this ui is taken from https://github.com/emilk/egui/blob/c6bd30642a78d5ff244b064642c053f62967ef1b/examples/file_dialog/src/main.rs
    pub fn ui_system(
        mut contexts: EguiContexts,
        mut state: Local<MyState>,
        mut file_dialog: Local<Option<Task<DialogResponse>>>,
    ) -> Result {
        let ctx = contexts.ctx_mut()?;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Drag-and-drop files onto the window!");

            if let Some(file_response) = file_dialog
                .as_mut()
                .and_then(|task| block_on(poll_once(task)))
            {
                state.picked_path = file_response.map(|path| path.path().display().to_string());
                *file_dialog = None;
            }

            if ui.button("Open file…").clicked() {
                *file_dialog = Some(
                    AsyncComputeTaskPool::get().spawn(rfd::AsyncFileDialog::new().pick_file()),
                );
            }

            if let Some(picked_path) = &state.picked_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });
            }

            // Show dropped files (if any):
            if !state.dropped_files.is_empty() {
                ui.group(|ui| {
                    ui.label("Dropped files:");

                    for file in &state.dropped_files {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };

                        let mut additional_info = vec![];
                        if !file.mime.is_empty() {
                            additional_info.push(format!("type: {}", file.mime));
                        }
                        if let Some(bytes) = &file.bytes {
                            additional_info.push(format!("{} bytes", bytes.len()));
                        }
                        if !additional_info.is_empty() {
                            info += &format!(" ({})", additional_info.join(", "));
                        }

                        ui.label(info);
                    }
                });
            }
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                state.dropped_files.clone_from(&i.raw.dropped_files);
            }
        });

        ctx.input(|i| {
            if i.raw.modifiers.ctrl {
                info!("ctrl pressed");
            }
        });

        Ok(())
    }

    fn preview_files_being_dropped(ctx: &egui::Context) {
        use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
        use std::fmt::Write as _;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n{}", file.mime).ok();
                    } else {
                        text += "\n???";
                    }
                }
                text
            });

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }
    }
}

// do nothing for wasm/ios/android :(
#[cfg(any(target_os = "ios", target_os = "android", target_arch = "wasm32"))]
mod foo {
    pub fn ui_system() {}
}
