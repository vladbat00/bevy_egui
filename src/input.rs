#[cfg(target_arch = "wasm32")]
use crate::text_agent::{is_mobile_safari, update_text_agent};
use crate::{
    helpers::{vec2_into_egui_pos2, QueryHelper},
    EguiContext, EguiContextSettings, EguiGlobalSettings, EguiInput, EguiOutput,
};
use bevy_ecs::prelude::*;
use bevy_input::{
    keyboard::{Key, KeyboardFocusLost, KeyboardInput},
    mouse::{MouseButton, MouseButtonInput, MouseScrollUnit, MouseWheel},
    touch::TouchInput,
    ButtonState,
};
use bevy_log as log;
use bevy_time::{Real, Time};
use bevy_window::{CursorMoved, Ime, Window};
use egui::Modifiers;

/// Cached pointer position, used to populate [`egui::Event::PointerButton`] events.
#[derive(Component, Default)]
pub struct EguiContextPointerPosition {
    /// Pointer position.
    pub position: egui::Pos2,
}

/// Stores an active touch id.
#[derive(Component, Default)]
pub struct EguiContextPointerTouchId {
    /// Active touch id.
    pub pointer_touch_id: Option<u64>,
}

/// Indicates whether [IME](https://en.wikipedia.org/wiki/Input_method) is enabled or disabled to avoid sending event duplicates.
#[derive(Component, Default)]
pub struct EguiContextImeState {
    /// Indicates whether IME is enabled.
    pub has_sent_ime_enabled: bool,
}

#[derive(Event)]
/// Wraps Egui events emitted by [`crate::EguiInputSet`] systems.
pub struct EguiInputEvent {
    /// Context to pass an event to.
    pub context: Entity,
    /// Wrapped event.
    pub event: egui::Event,
}

#[derive(Resource)]
/// Insert this resource when a pointer hovers over a non-window (e.g. world-space) [`EguiContext`] entity.
/// Also, make sure to update an [`EguiContextPointerPosition`] component of a hovered entity.
/// Both updates should happen during [`crate::EguiInputSet::InitReading`].
///
/// To learn how `bevy_egui` uses this resource, see the [`FocusedNonWindowEguiContext`] documentation.
pub struct HoveredNonWindowEguiContext(pub Entity);

/// Stores an entity of a focused non-window context (to push keyboard events to).
///
/// The resource won't exist if no context is focused, [`Option<Res<HoveredNonWindowEguiContext>>`] must be used to read from it.
/// If the [`HoveredNonWindowEguiContext`] resource exists, the [`FocusedNonWindowEguiContext`]
/// resource will get inserted on mouse button press or touch start event
/// (and removed if no hovered non-window context exists respectively).
///
/// Atm, it's up to users to update [`HoveredNonWindowEguiContext`] and [`EguiContextPointerPosition`].
/// We might be able to add proper `bevy_picking` support for world space UI once [`bevy_picking::backend::HitData`]
/// starts exposing triangle index or UV.
///
/// Updating focused contexts happens during [`crate::EguiInputSet::FocusContext`],
/// see [`write_pointer_button_events_system`] and [`write_window_touch_events_system`].
#[derive(Resource)]
pub struct FocusedNonWindowEguiContext(pub Entity);

/// Stores "pressed" state of modifier keys.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ModifierKeysState {
    /// Indicates whether the [`Key::Shift`] key is pressed.
    pub shift: bool,
    /// Indicates whether the [`Key::Control`] key is pressed.
    pub ctrl: bool,
    /// Indicates whether the [`Key::Alt`] key is pressed.
    pub alt: bool,
    /// Indicates whether the [`Key::Super`] (or [`Key::Meta`]) key is pressed.
    pub win: bool,
    is_macos: bool,
}

impl Default for ModifierKeysState {
    fn default() -> Self {
        let mut state = Self {
            shift: false,
            ctrl: false,
            alt: false,
            win: false,
            is_macos: false,
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            state.is_macos = cfg!(target_os = "macos");
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            let nav = window.navigator();
            if let Ok(user_agent) = nav.user_agent() {
                if user_agent.to_ascii_lowercase().contains("mac") {
                    state.is_macos = true;
                }
            }
        }

        state
    }
}

impl ModifierKeysState {
    /// Converts the struct to [`egui::Modifiers`].
    pub fn to_egui_modifiers(&self) -> egui::Modifiers {
        egui::Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: if self.is_macos { self.win } else { false },
            command: if self.is_macos { self.win } else { self.ctrl },
        }
    }

    /// Returns `true` if modifiers shouldn't prevent text input (we don't want to put characters on pressing Ctrl+A, etc).
    pub fn text_input_is_allowed(&self) -> bool {
        // Ctrl + Alt enables AltGr which is used to print special characters.
        !self.win && !self.ctrl || !self.is_macos && self.ctrl && self.alt
    }

    fn reset(&mut self) {
        self.shift = false;
        self.ctrl = false;
        self.alt = false;
        self.win = false;
    }
}

/// Reads [`KeyboardInput`] events to update the [`ModifierKeysState`] resource.
pub fn write_modifiers_keys_state_system(
    mut ev_keyboard_input: EventReader<KeyboardInput>,
    mut ev_focus: EventReader<KeyboardFocusLost>,
    mut modifier_keys_state: ResMut<ModifierKeysState>,
) {
    // If window focus is lost, clear all modifiers to avoid stuck keys.
    if !ev_focus.is_empty() {
        ev_focus.clear();
        modifier_keys_state.reset();
    }

    for event in ev_keyboard_input.read() {
        let KeyboardInput {
            logical_key, state, ..
        } = event;
        match logical_key {
            Key::Shift => {
                modifier_keys_state.shift = state.is_pressed();
            }
            Key::Control => {
                modifier_keys_state.ctrl = state.is_pressed();
            }
            Key::Alt => {
                modifier_keys_state.alt = state.is_pressed();
            }
            Key::Super | Key::Meta => {
                modifier_keys_state.win = state.is_pressed();
            }
            _ => {}
        };
    }
}

/// Reads [`MouseButtonInput`] events and wraps them into [`EguiInputEvent`] (only for window contexts).
pub fn write_window_pointer_moved_events_system(
    mut cursor_moved_reader: EventReader<CursorMoved>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut egui_contexts: Query<
        (&EguiContextSettings, &mut EguiContextPointerPosition),
        (With<EguiContext>, With<Window>),
    >,
) {
    for event in cursor_moved_reader.read() {
        let Some((context_settings, mut context_pointer_position)) =
            egui_contexts.get_some_mut(event.window)
        else {
            continue;
        };

        if !context_settings
            .input_system_settings
            .run_write_window_pointer_moved_events_system
        {
            continue;
        }

        let scale_factor = context_settings.scale_factor;
        let pointer_position = vec2_into_egui_pos2(event.position / scale_factor);
        context_pointer_position.position = pointer_position;
        egui_input_event_writer.send(EguiInputEvent {
            context: event.window,
            event: egui::Event::PointerMoved(pointer_position),
        });
    }
}

/// Reads [`MouseButtonInput`] events and wraps them into [`EguiInputEvent`], can redirect events to [`HoveredNonWindowEguiContext`],
/// inserts, updates or removes the [`FocusedNonWindowEguiContext`] resource based on a hovered context.
pub fn write_pointer_button_events_system(
    egui_global_settings: Res<EguiGlobalSettings>,
    mut commands: Commands,
    hovered_non_window_egui_context: Option<Res<HoveredNonWindowEguiContext>>,
    modifier_keys_state: Res<ModifierKeysState>,
    mut mouse_button_input_reader: EventReader<MouseButtonInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    egui_contexts: Query<(&EguiContextSettings, &EguiContextPointerPosition), With<EguiContext>>,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in mouse_button_input_reader.read() {
        let hovered_context = hovered_non_window_egui_context
            .as_deref()
            .map_or(event.window, |hovered| hovered.0);

        let Some((context_settings, context_pointer_position)) =
            egui_contexts.get_some(hovered_context)
        else {
            continue;
        };

        if !context_settings
            .input_system_settings
            .run_write_pointer_button_events_system
        {
            continue;
        }

        let button = match event.button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            MouseButton::Back => Some(egui::PointerButton::Extra1),
            MouseButton::Forward => Some(egui::PointerButton::Extra2),
            _ => None,
        };
        let Some(button) = button else {
            continue;
        };
        let pressed = match event.state {
            ButtonState::Pressed => true,
            ButtonState::Released => false,
        };
        egui_input_event_writer.send(EguiInputEvent {
            context: hovered_context,
            event: egui::Event::PointerButton {
                pos: context_pointer_position.position,
                button,
                pressed,
                modifiers,
            },
        });

        // If we are hovering over some UI in world space, we want to mark it as focused on mouse click.
        if egui_global_settings.enable_focused_non_window_context_updates && pressed {
            if let Some(hovered_non_window_egui_context) = &hovered_non_window_egui_context {
                commands.insert_resource(FocusedNonWindowEguiContext(
                    hovered_non_window_egui_context.0,
                ));
            } else {
                commands.remove_resource::<FocusedNonWindowEguiContext>();
            }
        }
    }
}

/// Reads [`CursorMoved`] events and wraps them into [`EguiInputEvent`] for a [`HoveredNonWindowEguiContext`] context (if one exists).
pub fn write_non_window_pointer_moved_events_system(
    hovered_non_window_egui_context: Option<Res<HoveredNonWindowEguiContext>>,
    mut cursor_moved_reader: EventReader<CursorMoved>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    egui_contexts: Query<(&EguiContextSettings, &EguiContextPointerPosition), With<EguiContext>>,
) {
    if cursor_moved_reader.is_empty() {
        return;
    }

    cursor_moved_reader.clear();
    let Some(HoveredNonWindowEguiContext(hovered_non_window_egui_context)) =
        hovered_non_window_egui_context.as_deref()
    else {
        return;
    };

    let Some((context_settings, context_pointer_position)) =
        egui_contexts.get_some(*hovered_non_window_egui_context)
    else {
        return;
    };

    if !context_settings
        .input_system_settings
        .run_write_non_window_pointer_moved_events_system
    {
        return;
    }

    egui_input_event_writer.send(EguiInputEvent {
        context: *hovered_non_window_egui_context,
        event: egui::Event::PointerMoved(context_pointer_position.position),
    });
}

/// Reads [`MouseWheel`] events and wraps them into [`EguiInputEvent`], can redirect events to [`HoveredNonWindowEguiContext`].
pub fn write_mouse_wheel_events_system(
    modifier_keys_state: Res<ModifierKeysState>,
    hovered_non_window_egui_context: Option<Res<HoveredNonWindowEguiContext>>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    egui_contexts: Query<&EguiContextSettings, With<EguiContext>>,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in mouse_wheel_reader.read() {
        let delta = egui::vec2(event.x, event.y);
        let unit = match event.unit {
            MouseScrollUnit::Line => egui::MouseWheelUnit::Line,
            MouseScrollUnit::Pixel => egui::MouseWheelUnit::Point,
        };

        let context = hovered_non_window_egui_context
            .as_deref()
            .map_or(event.window, |hovered| hovered.0);

        let Some(context_settings) = egui_contexts.get_some(context) else {
            continue;
        };

        if !context_settings
            .input_system_settings
            .run_write_mouse_wheel_events_system
        {
            continue;
        }

        egui_input_event_writer.send(EguiInputEvent {
            context,
            event: egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            },
        });
    }
}

/// Reads [`KeyboardInput`] events and wraps them into [`EguiInputEvent`], can redirect events to [`FocusedNonWindowEguiContext`].
pub fn write_keyboard_input_events_system(
    modifier_keys_state: Res<ModifierKeysState>,
    focused_non_window_egui_context: Option<Res<FocusedNonWindowEguiContext>>,
    #[cfg(all(
        feature = "manage_clipboard",
        not(target_os = "android"),
        not(target_arch = "wasm32")
    ))]
    mut egui_clipboard: ResMut<crate::EguiClipboard>,
    mut keyboard_input_reader: EventReader<KeyboardInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    egui_contexts: Query<&EguiContextSettings, With<EguiContext>>,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in keyboard_input_reader.read() {
        let context = focused_non_window_egui_context
            .as_deref()
            .map_or(event.window, |context| context.0);

        let Some(context_settings) = egui_contexts.get_some(context) else {
            continue;
        };

        if !context_settings
            .input_system_settings
            .run_write_keyboard_input_events_system
        {
            continue;
        }

        if modifier_keys_state.text_input_is_allowed() && event.state.is_pressed() {
            match &event.logical_key {
                Key::Character(char) if char.matches(char::is_control).count() == 0 => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context,
                        event: egui::Event::Text(char.to_string()),
                    });
                }
                Key::Space => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context,
                        event: egui::Event::Text(" ".to_string()),
                    });
                }
                _ => (),
            }
        }

        let (Some(key), physical_key) = (
            crate::helpers::bevy_to_egui_key(&event.logical_key),
            crate::helpers::bevy_to_egui_physical_key(&event.key_code),
        ) else {
            continue;
        };

        let egui_event = egui::Event::Key {
            key,
            pressed: event.state.is_pressed(),
            repeat: false,
            modifiers,
            physical_key,
        };
        egui_input_event_writer.send(EguiInputEvent {
            context,
            event: egui_event,
        });

        // We also check that it's a `ButtonState::Pressed` event, as we don't want to
        // copy, cut or paste on the key release.
        #[cfg(all(
            feature = "manage_clipboard",
            not(target_os = "android"),
            not(target_arch = "wasm32")
        ))]
        if modifiers.command && event.state.is_pressed() {
            match key {
                egui::Key::C => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context,
                        event: egui::Event::Copy,
                    });
                }
                egui::Key::X => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context,
                        event: egui::Event::Cut,
                    });
                }
                egui::Key::V => {
                    if let Some(contents) = egui_clipboard.get_contents() {
                        egui_input_event_writer.send(EguiInputEvent {
                            context,
                            event: egui::Event::Text(contents),
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

/// Reads [`Ime`] events and wraps them into [`EguiInputEvent`], can redirect events to [`FocusedNonWindowEguiContext`].
pub fn write_ime_events_system(
    focused_non_window_egui_context: Option<Res<FocusedNonWindowEguiContext>>,
    mut ime_reader: EventReader<Ime>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut egui_contexts: Query<
        (
            Entity,
            &EguiContextSettings,
            &mut EguiContextImeState,
            &EguiOutput,
        ),
        With<EguiContext>,
    >,
) {
    for event in ime_reader.read() {
        let window = match &event {
            Ime::Preedit { window, .. }
            | Ime::Commit { window, .. }
            | Ime::Disabled { window }
            | Ime::Enabled { window } => *window,
        };
        let context = focused_non_window_egui_context
            .as_deref()
            .map_or(window, |context| context.0);

        let Some((_entity, context_settings, mut ime_state, _egui_output)) =
            egui_contexts.get_some_mut(context)
        else {
            continue;
        };

        if !context_settings
            .input_system_settings
            .run_write_ime_events_system
        {
            continue;
        }

        let ime_event_enable =
            |ime_state: &mut EguiContextImeState,
             egui_input_event_writer: &mut EventWriter<EguiInputEvent>| {
                if !ime_state.has_sent_ime_enabled {
                    egui_input_event_writer.send(EguiInputEvent {
                        context,
                        event: egui::Event::Ime(egui::ImeEvent::Enabled),
                    });
                    ime_state.has_sent_ime_enabled = true;
                }
            };

        let ime_event_disable =
            |ime_state: &mut EguiContextImeState,
             egui_input_event_writer: &mut EventWriter<EguiInputEvent>| {
                if !ime_state.has_sent_ime_enabled {
                    egui_input_event_writer.send(EguiInputEvent {
                        context,
                        event: egui::Event::Ime(egui::ImeEvent::Disabled),
                    });
                    ime_state.has_sent_ime_enabled = false;
                }
            };

        // Aligned with the egui-winit implementation: https://github.com/emilk/egui/blob/0f2b427ff4c0a8c68f6622ec7d0afb7ba7e71bba/crates/egui-winit/src/lib.rs#L348
        match event {
            Ime::Enabled { window: _ } => {
                ime_event_enable(&mut ime_state, &mut egui_input_event_writer);
            }
            Ime::Preedit {
                value,
                window: _,
                cursor: _,
            } => {
                ime_event_enable(&mut ime_state, &mut egui_input_event_writer);
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::Ime(egui::ImeEvent::Preedit(value.clone())),
                });
            }
            Ime::Commit { value, window: _ } => {
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::Ime(egui::ImeEvent::Commit(value.clone())),
                });
                ime_event_disable(&mut ime_state, &mut egui_input_event_writer);
            }
            Ime::Disabled { window: _ } => {
                ime_event_disable(&mut ime_state, &mut egui_input_event_writer);
            }
        }
    }
}

/// Reads [`TouchInput`] events and wraps them into [`EguiInputEvent`].
pub fn write_window_touch_events_system(
    mut commands: Commands,
    egui_global_settings: Res<EguiGlobalSettings>,
    hovered_non_window_egui_context: Option<Res<HoveredNonWindowEguiContext>>,
    modifier_keys_state: Res<ModifierKeysState>,
    mut touch_input_reader: EventReader<TouchInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut egui_contexts: Query<
        (
            &EguiContextSettings,
            &mut EguiContextPointerPosition,
            &mut EguiContextPointerTouchId,
            &EguiOutput,
        ),
        (With<EguiContext>, With<Window>),
    >,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in touch_input_reader.read() {
        let Some((
            context_settings,
            mut context_pointer_position,
            mut context_pointer_touch_id,
            output,
        )) = egui_contexts.get_some_mut(event.window)
        else {
            continue;
        };

        if egui_global_settings.enable_focused_non_window_context_updates {
            if let bevy_input::touch::TouchPhase::Started = event.phase {
                if let Some(hovered_non_window_egui_context) =
                    hovered_non_window_egui_context.as_deref()
                {
                    if let bevy_input::touch::TouchPhase::Started = event.phase {
                        commands.insert_resource(FocusedNonWindowEguiContext(
                            hovered_non_window_egui_context.0,
                        ));
                    }

                    continue;
                }

                commands.remove_resource::<FocusedNonWindowEguiContext>();
            }
        }

        if !context_settings
            .input_system_settings
            .run_write_window_touch_events_system
        {
            continue;
        }

        let scale_factor = context_settings.scale_factor;
        let touch_position = vec2_into_egui_pos2(event.position / scale_factor);
        context_pointer_position.position = touch_position;
        write_touch_event(
            &mut egui_input_event_writer,
            event,
            event.window,
            output,
            touch_position,
            modifiers,
            &mut context_pointer_touch_id,
        );
    }
}

/// Reads [`TouchInput`] events and wraps them into [`EguiInputEvent`] for a [`HoveredNonWindowEguiContext`] context (if one exists).
pub fn write_non_window_touch_events_system(
    focused_non_window_egui_context: Option<Res<FocusedNonWindowEguiContext>>,
    mut touch_input_reader: EventReader<TouchInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    modifier_keys_state: Res<ModifierKeysState>,
    mut egui_contexts: Query<
        (
            &EguiContextSettings,
            &EguiContextPointerPosition,
            &mut EguiContextPointerTouchId,
            &EguiOutput,
        ),
        With<EguiContext>,
    >,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in touch_input_reader.read() {
        let Some(&FocusedNonWindowEguiContext(focused_non_window_egui_context)) =
            focused_non_window_egui_context.as_deref()
        else {
            continue;
        };

        let Some((
            context_settings,
            context_pointer_position,
            mut context_pointer_touch_id,
            output,
        )) = egui_contexts.get_some_mut(focused_non_window_egui_context)
        else {
            continue;
        };

        if !context_settings
            .input_system_settings
            .run_write_non_window_touch_events_system
        {
            continue;
        }

        write_touch_event(
            &mut egui_input_event_writer,
            event,
            focused_non_window_egui_context,
            output,
            context_pointer_position.position,
            modifiers,
            &mut context_pointer_touch_id,
        );
    }
}

fn write_touch_event(
    egui_input_event_writer: &mut EventWriter<EguiInputEvent>,
    event: &TouchInput,
    context: Entity,
    _output: &EguiOutput,
    pointer_position: egui::Pos2,
    modifiers: Modifiers,
    context_pointer_touch_id: &mut EguiContextPointerTouchId,
) {
    let touch_id = egui::TouchId::from(event.id);

    // Emit touch event
    egui_input_event_writer.send(EguiInputEvent {
        context,
        event: egui::Event::Touch {
            device_id: egui::TouchDeviceId(event.window.to_bits()),
            id: touch_id,
            phase: match event.phase {
                bevy_input::touch::TouchPhase::Started => egui::TouchPhase::Start,
                bevy_input::touch::TouchPhase::Moved => egui::TouchPhase::Move,
                bevy_input::touch::TouchPhase::Ended => egui::TouchPhase::End,
                bevy_input::touch::TouchPhase::Canceled => egui::TouchPhase::Cancel,
            },
            pos: pointer_position,
            force: match event.force {
                Some(bevy_input::touch::ForceTouch::Normalized(force)) => Some(force as f32),
                Some(bevy_input::touch::ForceTouch::Calibrated {
                    force,
                    max_possible_force,
                    ..
                }) => Some((force / max_possible_force) as f32),
                None => None,
            },
        },
    });

    // If we're not yet translating a touch, or we're translating this very
    // touch, …
    if context_pointer_touch_id.pointer_touch_id.is_none()
        || context_pointer_touch_id.pointer_touch_id.unwrap() == event.id
    {
        // … emit PointerButton resp. PointerMoved events to emulate mouse.
        match event.phase {
            bevy_input::touch::TouchPhase::Started => {
                context_pointer_touch_id.pointer_touch_id = Some(event.id);
                // First move the pointer to the right location.
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::PointerMoved(pointer_position),
                });
                // Then do mouse button input.
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::PointerButton {
                        pos: pointer_position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers,
                    },
                });
            }
            bevy_input::touch::TouchPhase::Moved => {
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::PointerMoved(pointer_position),
                });
            }
            bevy_input::touch::TouchPhase::Ended => {
                context_pointer_touch_id.pointer_touch_id = None;
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::PointerButton {
                        pos: pointer_position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                    },
                });
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::PointerGone,
                });

                #[cfg(target_arch = "wasm32")]
                if !is_mobile_safari() {
                    update_text_agent(
                        _output.platform_output.ime.is_some()
                            || _output.platform_output.mutable_text_under_cursor,
                    );
                }
            }
            bevy_input::touch::TouchPhase::Canceled => {
                context_pointer_touch_id.pointer_touch_id = None;
                egui_input_event_writer.send(EguiInputEvent {
                    context,
                    event: egui::Event::PointerGone,
                });
            }
        }
    }
}

/// Reads [`EguiInputEvent`] events and feeds them to Egui.
pub fn write_egui_input_system(
    focused_non_window_egui_context: Option<Res<FocusedNonWindowEguiContext>>,
    modifier_keys_state: Res<ModifierKeysState>,
    mut egui_input_event_reader: EventReader<EguiInputEvent>,
    mut egui_contexts: Query<(Entity, &mut EguiInput, Option<&Window>)>,
    time: Res<Time<Real>>,
) {
    for EguiInputEvent { context, event } in egui_input_event_reader.read() {
        #[cfg(feature = "log_input_events")]
        log::warn!("{context:?}: {event:?}");

        let (_, mut egui_input, _) = match egui_contexts.get_mut(*context) {
            Ok(egui_input) => egui_input,
            Err(err) => {
                log::error!(
                    "Failed to get an Egui context ({context:?}) for an event ({event:?}): {err:?}"
                );
                continue;
            }
        };

        egui_input.events.push(event.clone());
    }

    for (entity, mut egui_input, window) in egui_contexts.iter_mut() {
        egui_input.focused = focused_non_window_egui_context.as_deref().map_or_else(
            || window.is_some_and(|window| window.focused),
            |context| context.0 == entity,
        );
        egui_input.modifiers = modifier_keys_state.to_egui_modifiers();
        egui_input.time = Some(time.elapsed_secs_f64());
    }
}
