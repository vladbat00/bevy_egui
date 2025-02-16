use crate::{helpers, EguiContext, EguiContextSettings, EguiFullOutput, EguiRenderOutput};
#[cfg(windows)]
use bevy_ecs::system::Local;
use bevy_ecs::{
    entity::Entity,
    event::EventWriter,
    system::{NonSend, Query},
};
use bevy_window::RequestRedraw;
use bevy_winit::{cursor::CursorIcon, EventLoopProxy, WakeUp};
use std::{sync::Arc, time::Duration};

/// Reads Egui output.
pub fn process_output_system(
    mut contexts: Query<(
        Entity,
        &mut EguiContext,
        &mut EguiFullOutput,
        &mut EguiRenderOutput,
        Option<&mut CursorIcon>,
        &EguiContextSettings,
    )>,
    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
    mut egui_clipboard: bevy_ecs::system::ResMut<crate::EguiClipboard>,
    mut event: EventWriter<RequestRedraw>,
    #[cfg(windows)] mut last_cursor_icon: Local<bevy_utils::HashMap<Entity, egui::CursorIcon>>,
    event_loop_proxy: Option<NonSend<EventLoopProxy<WakeUp>>>,
) {
    let mut should_request_redraw = false;

    for (_entity, mut context, mut full_output, mut render_output, cursor_icon, _settings) in
        contexts.iter_mut()
    {
        let ctx = context.get_mut();
        let Some(full_output) = full_output.0.take() else {
            bevy_log::error!("bevy_egui pass output has not been prepared (if EguiSettings::run_manually is set to true, make sure to call egui::Context::run or egui::Context::begin_pass and egui::Context::end_pass)");
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

        render_output.paint_jobs = Arc::new(paint_jobs);
        render_output.textures_delta = Arc::new(textures_delta);

        for command in platform_output.commands {
            match command {
                egui::OutputCommand::CopyText(_text) =>
                {
                    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
                    if !_text.is_empty() {
                        egui_clipboard.set_text(&_text);
                    }
                }
                egui::OutputCommand::CopyImage(_image) => {
                    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
                    egui_clipboard.set_image(&_image);
                }
                egui::OutputCommand::OpenUrl(_url) => {
                    #[cfg(feature = "open_url")]
                    {
                        let egui::output::OpenUrl { url, new_tab } = _url;
                        let target = if new_tab {
                            "_blank"
                        } else {
                            _settings
                                .default_open_url_target
                                .as_deref()
                                .unwrap_or("_self")
                        };
                        if let Err(err) = webbrowser::open_browser_with_options(
                            webbrowser::Browser::Default,
                            &url,
                            webbrowser::BrowserOptions::new().with_target_hint(target),
                        ) {
                            bevy_log::error!("Failed to open '{}': {:?}", url, err);
                        }
                    }
                }
            }
        }

        if let Some(mut cursor) = cursor_icon {
            let mut set_icon = || {
                *cursor = CursorIcon::System(
                    helpers::egui_to_winit_cursor_icon(platform_output.cursor_icon)
                        .unwrap_or(bevy_window::SystemCursorIcon::Default),
                );
            };

            #[cfg(windows)]
            {
                let last_cursor_icon = last_cursor_icon.entry(_entity).or_default();
                if *last_cursor_icon != platform_output.cursor_icon {
                    set_icon();
                    *last_cursor_icon = platform_output.cursor_icon;
                }
            }
            #[cfg(not(windows))]
            set_icon();
        }

        let needs_repaint = !render_output.is_empty();
        should_request_redraw |= ctx.has_requested_repaint() && needs_repaint;

        // The resource doesn't exist in the headless mode.
        if let Some(event_loop_proxy) = &event_loop_proxy {
            // A zero duration indicates that it's an outstanding redraw request, which gives Egui an
            // opportunity to settle the effects of interactions with widgets. Such repaint requests
            // are processed not immediately but on a next frame. In this case, we need to indicate to
            // winit, that it needs to wake up next frame as well even if there are no inputs.
            //
            // TLDR: this solves repaint corner cases of `WinitSettings::desktop_app()`.
            if let Some(Duration::ZERO) =
                ctx.viewport(|viewport| viewport.input.wants_repaint_after())
            {
                let _ = event_loop_proxy.send_event(WakeUp);
            }
        }
    }

    if should_request_redraw {
        event.send(RequestRedraw);
    }
}
