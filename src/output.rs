use crate::{
    EguiClipboard, EguiContext, EguiContextSettings, EguiFullOutput, EguiGlobalSettings,
    EguiOutput, EguiRenderOutput, helpers,
    helpers::egui_pos2_into_vec2,
    input::{EguiInputEvent, WindowToEguiContextMap},
};
use bevy_app::AppExit;
use bevy_ecs::{
    change_detection::{Mut, ResMut},
    entity::Entity,
    message::MessageWriter,
    observer::On,
    system::{Commands, Local, Query, Res},
};
use bevy_log as log;
use bevy_math::CompassOctant;
use bevy_platform::collections::HashMap;
use bevy_render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy_window::{
    CursorGrabMode, CursorIcon, CursorOptions, MonitorSelection, RequestRedraw, Window,
    WindowLevel, WindowMode, WindowPosition, WindowTheme,
};
use egui::{
    CursorGrab, ResizeDirection, SystemTheme, UserData, ViewportCommand, ViewportId, ViewportOutput,
};
use std::sync::Arc;

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
    mut window_query: Query<(&mut Window, &mut CursorOptions)>,
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
            viewport_output,
        } = full_output;
        let paint_jobs = ctx.tessellate(shapes, pixels_per_point);

        render_output.paint_jobs = paint_jobs;
        render_output.textures_delta = textures_delta;
        egui_output.platform_output = platform_output;
        egui_output.viewport_output = viewport_output;
        egui_output.pixels_per_point = pixels_per_point;

        process_platform_output(
            #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
            &mut egui_clipboard,
            &mut egui_output,
            settings,
        );

        if egui_output.viewport_output.len() != 1
            || !egui_output.viewport_output.contains_key(&ViewportId::ROOT)
        {
            log::warn_once!("bevy_egui supports only a single viewport, which must be root");
        }
        if let Some(viewport) = egui_output.viewport_output.get(&ViewportId::ROOT)
            && let Some(window_entity) = window_to_egui_context_map.context_to_window.get(&entity)
            && let Ok((mut window, mut cursor_options)) = window_query.get_mut(*window_entity)
        {
            process_viewport_output(
                &mut commands,
                entity,
                viewport,
                &mut *window,
                &mut *cursor_options,
            );
        }

        if egui_global_settings.enable_cursor_icon_updates
            && settings.enable_cursor_icon_updates
            && let Some(window_entity) = window_to_egui_context_map.context_to_window.get(&entity)
        {
            let last_cursor_icon = last_cursor_icon.entry(entity).or_default();
            if *last_cursor_icon != egui_output.platform_output.cursor_icon {
                commands
                    .entity(*window_entity)
                    .try_insert(CursorIcon::System(
                        helpers::egui_to_winit_cursor_icon(egui_output.platform_output.cursor_icon)
                            .unwrap_or(bevy_window::SystemCursorIcon::Default),
                    ));
                *last_cursor_icon = egui_output.platform_output.cursor_icon;
            }
        }

        let needs_repaint = !render_output.is_empty();
        should_request_redraw |= ctx.has_requested_repaint() && needs_repaint;
    }

    if should_request_redraw {
        request_redraw_writer.write(RequestRedraw);
    }
}

fn process_platform_output(
    #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
    egui_clipboard: &mut ResMut<EguiClipboard>,
    egui_output: &mut Mut<EguiOutput>,
    settings: &EguiContextSettings,
) {
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
}

fn process_viewport_output(
    commands: &mut Commands,
    context_entity: Entity,
    viewport: &ViewportOutput,
    window: &mut Window,
    cursor_options: &mut CursorOptions,
) {
    debug_assert!(
        viewport.class == egui::ViewportClass::Root,
        "Viewport class must be Root"
    );

    for command in &viewport.commands {
        match command {
            ViewportCommand::Close => {
                commands.write_message(AppExit::Success);
            }
            ViewportCommand::CancelClose => {}
            ViewportCommand::Title(title) => {
                window.title = title.clone();
            }
            ViewportCommand::Transparent(transparent) => {
                window.transparent = *transparent;
            }
            ViewportCommand::Visible(visible) => {
                window.visible = *visible;
            }
            ViewportCommand::StartDrag => {
                window.start_drag_move();
            }
            ViewportCommand::OuterPosition(position) => {
                window.position = WindowPosition::At(egui_pos2_into_vec2(*position).as_ivec2());
            }
            ViewportCommand::InnerSize(inner_size) => {
                window.resolution.set(inner_size.x, inner_size.y);
            }
            ViewportCommand::MinInnerSize(min_size) => {
                window.resize_constraints.min_width = min_size.x;
                window.resize_constraints.min_height = min_size.y;
            }
            ViewportCommand::MaxInnerSize(max_size) => {
                window.resize_constraints.max_width = max_size.x;
                window.resize_constraints.max_height = max_size.y;
            }
            ViewportCommand::ResizeIncrements(resize) => {
                if let Some(resize) = resize {
                    window.resolution.set(
                        window.resolution.physical_width() as f32 + resize.x,
                        window.resolution.physical_height() as f32 + resize.y,
                    );
                }
            }
            ViewportCommand::BeginResize(direction) => {
                window.start_drag_resize(match direction {
                    ResizeDirection::North => CompassOctant::North,
                    ResizeDirection::South => CompassOctant::South,
                    ResizeDirection::East => CompassOctant::East,
                    ResizeDirection::West => CompassOctant::West,
                    ResizeDirection::NorthEast => CompassOctant::NorthEast,
                    ResizeDirection::SouthEast => CompassOctant::SouthEast,
                    ResizeDirection::NorthWest => CompassOctant::NorthWest,
                    ResizeDirection::SouthWest => CompassOctant::SouthWest,
                });
            }
            ViewportCommand::Resizable(resizable) => {
                window.resizable = *resizable;
            }
            ViewportCommand::EnableButtons {
                close,
                minimized,
                maximize,
            } => {
                window.enabled_buttons.close = *close;
                window.enabled_buttons.minimize = *minimized;
                window.enabled_buttons.maximize = *maximize;
            }
            ViewportCommand::Minimized(minimized) => {
                window.set_minimized(*minimized);
            }
            ViewportCommand::Maximized(maximized) => {
                window.set_maximized(*maximized);
            }
            ViewportCommand::Fullscreen(fullscreen) => {
                window.mode = if *fullscreen {
                    WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                } else {
                    WindowMode::Windowed
                };
            }
            ViewportCommand::SetMonitor(monitor) => {
                window.position = WindowPosition::Centered(MonitorSelection::Index(*monitor));
            }
            ViewportCommand::Decorations(decorations) => {
                window.decorations = *decorations;
            }
            ViewportCommand::WindowLevel(window_level) => {
                window.window_level = match window_level {
                    egui::WindowLevel::Normal => WindowLevel::Normal,
                    egui::WindowLevel::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
                    egui::WindowLevel::AlwaysOnTop => WindowLevel::AlwaysOnTop,
                };
            }
            ViewportCommand::Icon(_) => {
                log::warn_once!("bevy_egui doesn't support `ViewportCommand::Icon`");
            }
            ViewportCommand::IMERect(_) => {
                log::warn_once!("bevy_egui doesn't support `ViewportCommand::IMERect`");
            }
            ViewportCommand::IMEAllowed(_) => {
                log::warn_once!("bevy_egui doesn't support `ViewportCommand::IMEAllowed`");
            }
            ViewportCommand::IMEPurpose(_) => {
                log::warn_once!("bevy_egui doesn't support `ViewportCommand::IMEPurpose`");
            }
            ViewportCommand::Focus => {
                window.focused = true;
            }
            ViewportCommand::RequestUserAttention(_) => {
                log::warn_once!(
                    "bevy_egui doesn't support `ViewportCommand::RequestUserAttention`"
                );
            }
            ViewportCommand::SetTheme(system_theme) => {
                window.window_theme = match system_theme {
                    SystemTheme::SystemDefault => None,
                    SystemTheme::Light => Some(WindowTheme::Light),
                    SystemTheme::Dark => Some(WindowTheme::Dark),
                };
            }
            ViewportCommand::ContentProtected(_) => {
                log::warn_once!("bevy_egui doesn't support `ViewportCommand::ContentProtected`");
            }
            ViewportCommand::CursorPosition(cursor_position) => {
                window.set_cursor_position(Some(egui_pos2_into_vec2(*cursor_position)));
            }
            ViewportCommand::CursorGrab(cursor_grab) => {
                cursor_options.grab_mode = match cursor_grab {
                    CursorGrab::None => CursorGrabMode::None,
                    CursorGrab::Confined => CursorGrabMode::Confined,
                    CursorGrab::Locked => CursorGrabMode::Locked,
                };
            }
            ViewportCommand::CursorVisible(cursor_visible) => {
                cursor_options.visible = *cursor_visible;
            }
            ViewportCommand::MousePassthrough(mouse_passthrough) => {
                cursor_options.hit_test = *mouse_passthrough;
            }
            ViewportCommand::Screenshot(user_data) => {
                log::warn!("hi");
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(send_screenshot(context_entity, user_data.clone()));
            }
            ViewportCommand::RequestCut => {}
            ViewportCommand::RequestCopy => {}
            ViewportCommand::RequestPaste => {}
        }
    }
}

fn send_screenshot(
    context_entity: Entity,
    user_data: UserData,
) -> impl FnMut(On<'_, '_, ScreenshotCaptured>, MessageWriter<EguiInputEvent>) {
    move |trigger: On<bevy_render::view::screenshot::ScreenshotCaptured>, mut egui_input_writer: MessageWriter<EguiInputEvent>| {
        let user_data = user_data.clone();
        let mut pixels =
            Vec::with_capacity((trigger.image.width() * trigger.image.height()) as usize);
        for x in 0..trigger.image.width() {
            for y in 0..trigger.image.height() {
                match trigger.image.get_color_at(x, y) {
                    Ok(color) => {
                        let srgba = color.to_srgba();
                        pixels.push(egui::Color32::from_rgba_unmultiplied(
                            egui::ecolor::linear_u8_from_linear_f32(srgba.red),
                            egui::ecolor::linear_u8_from_linear_f32(srgba.green),
                            egui::ecolor::linear_u8_from_linear_f32(srgba.blue),
                            egui::ecolor::linear_u8_from_linear_f32(srgba.alpha),
                        ));
                    }
                    Err(err) => {
                        log::error!("Failed to read screenshot color data: {err:?}");
                        return;
                    }
                };
            }
        }

        egui_input_writer.write(EguiInputEvent {
            context: context_entity,
            event: egui::Event::Screenshot {
                viewport_id: ViewportId::ROOT,
                user_data,
                image: Arc::new(egui::ColorImage::new(
                    [
                        trigger.image.size().x as usize,
                        trigger.image.size().y as usize,
                    ],
                    pixels,
                )),
            },
        });
    }
}
