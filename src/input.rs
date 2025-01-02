#[cfg(target_arch = "wasm32")]
use crate::text_agent::{is_mobile_safari, update_text_agent};
use crate::{EguiClipboard, EguiContext, EguiInput, EguiOutput, EguiSettings};
use bevy_ecs::prelude::*;
use bevy_input::{
    keyboard::{Key, KeyboardFocusLost, KeyboardInput},
    mouse::{MouseButton, MouseButtonInput, MouseScrollUnit, MouseWheel},
    touch::TouchInput,
    ButtonState,
};
use bevy_time::{Real, Time};
use bevy_window::{CursorMoved, Ime, Window};
use crate::helpers::QueryHelper;

/// Cached pointer position, used to populate [`egui::Event::PointerButton`] events.
#[derive(Component, Default)]
pub struct EguiContextMousePosition {
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
pub struct EguiInputEvent {
    pub context: Entity,
    pub event: egui::Event,
}

/// Stores an entity of a focused context (to push keyboard events to).
///
/// The resource won't exist if no context is focused, [`Option<Res<HoveredEguiContext>>`] must be used to read from it.
///
/// TODO!
#[derive(Resource)]
pub struct FocusedEguiContext(pub Entity);

/// Stores "pressed" state of modifier keys.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ModifierKeysState {
    /// Indicates whether the [`Key::Shift`] key is pressed.
    pub shift: bool,
    /// Indicates whether the [`Key::Ctrl`] key is pressed.
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
                    *context_params.is_macos = true;
                }
            }
        }

        state
    }
}

impl ModifierKeysState {
    pub fn to_egui_modifiers(&self) -> egui::Modifiers {
        egui::Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: if self.is_macos { self.win } else { false },
            command: if self.is_macos { self.win } else { self.ctrl },
        }
    }

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

pub fn write_window_pointer_moved_events_system(
    mut cursor_moved_reader: EventReader<CursorMoved>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut egui_contexts: Query<
        (&EguiSettings, &mut EguiContextMousePosition),
        (With<EguiContext>, With<Window>),
    >,
) {
    for event in cursor_moved_reader.read() {
        let Some((context_settings, mut context_mouse_position)) =
            egui_contexts.get_some_mut(event.window)
        else {
            continue;
        };

        let scale_factor = context_settings.scale_factor;
        let (x, y): (f32, f32) = (event.position / scale_factor).into();
        let mouse_position = egui::pos2(x, y);
        context_mouse_position.position = mouse_position;
        egui_input_event_writer.send(EguiInputEvent {
            context: event.window,
            event: egui::Event::PointerMoved(mouse_position),
        });
    }
}

pub fn write_window_pointer_button_events_system(
    modifier_keys_state: Res<ModifierKeysState>,
    mut mouse_button_input_reader: EventReader<MouseButtonInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    egui_contexts: Query<&EguiContextMousePosition, (With<EguiContext>, With<Window>)>,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in mouse_button_input_reader.read() {
        let Some(context_mouse_position) = egui_contexts.get_some(event.window) else {
            continue;
        };

        // TODO! move to helpers
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
            context: event.window,
            event: egui::Event::PointerButton {
                pos: context_mouse_position.position,
                button,
                pressed,
                modifiers,
            },
        });
    }
}

pub fn write_window_mouse_wheel_events_system(
    modifier_keys_state: Res<ModifierKeysState>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in mouse_wheel_reader.read() {
        let delta = egui::vec2(event.x, event.y);
        let unit = match event.unit {
            MouseScrollUnit::Line => egui::MouseWheelUnit::Line,
            MouseScrollUnit::Pixel => egui::MouseWheelUnit::Point,
        };

        egui_input_event_writer.send(EguiInputEvent {
            context: event.window,
            event: egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            },
        });
    }
}

pub fn write_keyboard_input_events_system(
    modifier_keys_state: Res<ModifierKeysState>,
    mut egui_clipboard: ResMut<EguiClipboard>,
    mut keyboard_input_reader: EventReader<KeyboardInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in keyboard_input_reader.read() {
        if modifier_keys_state.text_input_is_allowed() && event.state.is_pressed() {
            match &event.logical_key {
                Key::Character(char) if char.matches(char::is_control).count() == 0 => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::Text(char.to_string()),
                    });
                }
                Key::Space => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::Text(" ".into()),
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
            context: event.window,
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
                        context: event.window,
                        event: egui::Event::Copy,
                    });
                }
                egui::Key::X => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::Cut,
                    });
                }
                egui::Key::V => {
                    if let Some(contents) = egui_clipboard.get_contents() {
                        egui_input_event_writer.send(EguiInputEvent {
                            context: event.window,
                            event: egui::Event::Text(contents),
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn write_window_ime_events_system(
    mut ime_reader: EventReader<Ime>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut egui_contexts: Query<
        (&mut EguiContextImeState, &EguiOutput),
        (With<EguiContext>, With<Window>),
    >,
) {
    #[cfg(target_arch = "wasm32")]
    let mut editing_text = false;
    #[cfg(target_arch = "wasm32")]
    for (_ime_state, egui_output) in context_params.contexts.iter() {
        let platform_output = &context.egui_output.platform_output;
        if platform_output.ime.is_some() || platform_output.mutable_text_under_cursor {
            editing_text = true;
            break;
        }
    }

    for event in ime_reader.read() {
        let window = match &event {
            Ime::Preedit { window, .. }
            | Ime::Commit { window, .. }
            | Ime::Disabled { window }
            | Ime::Enabled { window } => *window,
        };

        let Some((mut ime_state, _egui_output)) = egui_contexts.get_some_mut(window) else {
            continue;
        };

        let ime_event_enable =
            |ime_state: &mut EguiContextImeState,
             egui_input_event_writer: &mut EventWriter<EguiInputEvent>| {
                if !ime_state.has_sent_ime_enabled {
                    egui_input_event_writer.send(EguiInputEvent {
                        context: window,
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
                        context: window,
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
                    context: window,
                    event: egui::Event::Ime(egui::ImeEvent::Preedit(value.clone())),
                });
            }
            Ime::Commit { value, window: _ } => {
                egui_input_event_writer.send(EguiInputEvent {
                    context: window,
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

pub fn write_window_touch_events_system(
    modifier_keys_state: Res<ModifierKeysState>,
    mut touch_input_reader: EventReader<TouchInput>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut egui_contexts: Query<
        (&EguiSettings, &mut EguiContextPointerTouchId),
        (With<EguiContext>, With<Window>),
    >,
) {
    let modifiers = modifier_keys_state.to_egui_modifiers();
    for event in touch_input_reader.read() {
        let Some((context_settings, mut context_pointer_touch_id)) =
            egui_contexts.get_some_mut(event.window)
        else {
            continue;
        };

        let touch_id = egui::TouchId::from(event.id);
        let scale_factor = context_settings.scale_factor;
        let touch_position: (f32, f32) = (event.position / scale_factor).into();

        // Emit touch event
        egui_input_event_writer.send(EguiInputEvent {
            context: event.window,
            event: egui::Event::Touch {
                device_id: egui::TouchDeviceId(event.window.to_bits()),
                id: touch_id,
                phase: match event.phase {
                    bevy_input::touch::TouchPhase::Started => egui::TouchPhase::Start,
                    bevy_input::touch::TouchPhase::Moved => egui::TouchPhase::Move,
                    bevy_input::touch::TouchPhase::Ended => egui::TouchPhase::End,
                    bevy_input::touch::TouchPhase::Canceled => egui::TouchPhase::Cancel,
                },
                pos: egui::pos2(touch_position.0, touch_position.1),
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
                        context: event.window,
                        event: egui::Event::PointerMoved(egui::pos2(
                            touch_position.0,
                            touch_position.1,
                        )),
                    });
                    // Then do mouse button input.
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::PointerButton {
                            pos: egui::pos2(touch_position.0, touch_position.1),
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers,
                        },
                    });
                }
                bevy_input::touch::TouchPhase::Moved => {
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::PointerMoved(egui::pos2(
                            touch_position.0,
                            touch_position.1,
                        )),
                    });
                }
                bevy_input::touch::TouchPhase::Ended => {
                    context_pointer_touch_id.pointer_touch_id = None;
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::PointerButton {
                            pos: egui::pos2(touch_position.0, touch_position.1),
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers,
                        },
                    });
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::PointerGone,
                    });

                    #[cfg(target_arch = "wasm32")]
                    if !is_mobile_safari() {
                        update_text_agent(editing_text);
                    }
                }
                bevy_input::touch::TouchPhase::Canceled => {
                    context_pointer_touch_id.pointer_touch_id = None;
                    egui_input_event_writer.send(EguiInputEvent {
                        context: event.window,
                        event: egui::Event::PointerGone,
                    });
                }
            }
        }
    }
}

/// Reads [`EguiInputEvent`] events and feeds them to Egui.
pub fn write_egui_input_system(
    modifier_keys_state: Res<ModifierKeysState>,
    mut egui_input_event_reader: EventReader<EguiInputEvent>,
    mut egui_contexts: Query<&mut EguiInput>,
    time: Res<Time<Real>>,
) {
    for EguiInputEvent { context, event } in egui_input_event_reader.read() {
        #[cfg(feature = "log_input_events")]
        bevy_log::info!("{context:?}: {event:?}");

        let mut egui_input = match egui_contexts.get_mut(*context) {
            Ok(egui_input) => egui_input,
            Err(err) => {
                bevy_log::error!(
                    "Failed to get an Egui context ({context:?}) for an event ({event:?}): {err:?}"
                );
                continue;
            }
        };

        egui_input.events.push(event.clone());
    }

    for mut egui_input in egui_contexts.iter_mut() {
        egui_input.modifiers = modifier_keys_state.to_egui_modifiers();
        egui_input.time = Some(time.elapsed_secs_f64());
    }
}
