use crate::{
    EguiContext, EguiContextSettings, EguiFullOutput, EguiGlobalSettings, EguiOutput,
    EguiRenderOutput, helpers, input::WindowToEguiContextMap,
};
use bevy_ecs::{
    entity::Entity,
    message::MessageWriter,
    system::{Commands, Local, Query, Res},
};
use bevy_platform::collections::HashMap;
use bevy_window::{CursorIcon, RequestRedraw};

/// Reads Egui output.
#[allow(clippy::too_many_arguments)]
pub fn process_output_system(
    mut commands: Commands,
    mut context_query: Query<(
        Entity,
        &mut EguiContext,
        &mut EguiFullOutput,
        &mut EguiRenderOutput,
        &mut EguiOutput,
        &EguiContextSettings,
    )>,
    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
    mut egui_clipboard: bevy_ecs::system::ResMut<crate::EguiClipboard>,
    mut request_redraw_writer: MessageWriter<RequestRedraw>,
    mut last_cursor_icon: Local<HashMap<Entity, egui::CursorIcon>>,
    egui_global_settings: Res<EguiGlobalSettings>,
    window_to_egui_context_map: Res<WindowToEguiContextMap>,
) {
    let mut should_request_redraw = false;

    for (entity, mut context, mut full_output, mut render_output, mut egui_output, settings) in
        context_query.iter_mut()
    {
        let ctx = context.get_mut();
        let Some(full_output) = full_output.0.take() else {
            bevy_log::error!(
                "bevy_egui pass output has not been prepared (if EguiSettings::run_manually is set to true, make sure to call egui::Context::run or egui::Context::begin_pass and egui::Context::end_pass)"
            );
            continue;
        };
        let egui::FullOutput {
            platform_output,
            shapes,
            textures_delta,
            pixels_per_point,
            viewport_output: _,
        } = full_output;
        let paint_jobs = ctx.tessellate(shapes, pixels_per_point);

        render_output.paint_jobs = paint_jobs;
        render_output.textures_delta = textures_delta;
        egui_output.platform_output = platform_output;

        for command in &egui_output.platform_output.commands {
            match command {
                egui::OutputCommand::CopyText(_text) =>
                {
                    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
                    if !_text.is_empty() {
                        egui_clipboard.set_text(_text);
                    }
                }
                egui::OutputCommand::CopyImage(_image) => {
                    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
                    egui_clipboard.set_image(_image);
                }
                egui::OutputCommand::OpenUrl(_url) => {
                    #[cfg(feature = "open_url")]
                    {
                        let egui::output::OpenUrl { url, new_tab } = _url;
                        let target = if *new_tab {
                            "_blank"
                        } else {
                            settings
                                .default_open_url_target
                                .as_deref()
                                .unwrap_or("_self")
                        };
                        if let Err(err) = webbrowser::open_browser_with_options(
                            webbrowser::Browser::Default,
                            url,
                            webbrowser::BrowserOptions::new().with_target_hint(target),
                        ) {
                            bevy_log::error!("Failed to open '{}': {:?}", url, err);
                        }
                    }
                }
            }
        }

        if egui_global_settings.enable_cursor_icon_updates && settings.enable_cursor_icon_updates {
            if let Some(window_entity) = window_to_egui_context_map.context_to_window.get(&entity) {
                let last_cursor_icon = last_cursor_icon.entry(entity).or_default();
                if *last_cursor_icon != egui_output.platform_output.cursor_icon {
                    commands
                        .entity(*window_entity)
                        .try_insert(CursorIcon::System(
                            helpers::egui_to_winit_cursor_icon(
                                egui_output.platform_output.cursor_icon,
                            )
                            .unwrap_or(bevy_window::SystemCursorIcon::Default),
                        ));
                    *last_cursor_icon = egui_output.platform_output.cursor_icon;
                }
            }
        }

        let needs_repaint = !render_output.is_empty();
        should_request_redraw |= ctx.has_requested_repaint() && needs_repaint;
    }

    if should_request_redraw {
        request_redraw_writer.write(RequestRedraw);
    }
}
