//! The text agent is an `<input>` element used to trigger
//! mobile keyboard and IME input.

use crate::{
    EguiContextSettings, EguiInput, EguiOutput, EventClosure, SubscribedEvents,
    input::{EguiInputEvent, FocusedNonWindowEguiContext},
};
use bevy_ecs::prelude::*;
use bevy_log as log;
use bevy_platform::collections::HashSet;
use bevy_window::RequestRedraw;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{LazyLock, Mutex};
use wasm_bindgen::prelude::*;

static AGENT_ID: &str = "egui_text_agent";

// Stores if we are editing text, to react on touch events as a workaround for Safari.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct VirtualTouchInfo {
    editing_text: bool,
}

/// Channel for receiving events from a text agent.
#[derive(Resource)]
pub struct TextAgentChannel {
    sender: Sender<egui::Event>,
    receiver: Receiver<egui::Event>,
}

impl Default for TextAgentChannel {
    fn default() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }
}

/// Wraps [`VirtualTouchInfo`] and channels that notify when we need to update it.
#[derive(Resource)]
pub struct SafariVirtualKeyboardTouchState {
    pub(crate) sender: Sender<()>,
    pub(crate) receiver: Receiver<()>,
    pub(crate) touch_info: &'static LazyLock<Mutex<VirtualTouchInfo>>,
}

/// Listens to the [`SafariVirtualKeyboardTouchState`] channel and updates the wrapped [`VirtualTouchInfo`].
pub fn process_safari_virtual_keyboard_system(
    egui_contexts: Query<(&EguiInput, &EguiOutput)>,
    safari_virtual_keyboard_touch_state: Res<SafariVirtualKeyboardTouchState>,
) {
    let mut received = false;
    while let Ok(()) = safari_virtual_keyboard_touch_state.receiver.try_recv() {
        received = true;
    }
    if !received {
        return;
    }

    let mut editing_text = false;
    for (egui_input, egui_output) in egui_contexts.iter() {
        if !egui_input.focused {
            continue;
        }
        let platform_output = &egui_output.platform_output;
        if platform_output.ime.is_some() || platform_output.mutable_text_under_cursor {
            editing_text = true;
            break;
        }
    }

    match safari_virtual_keyboard_touch_state.touch_info.lock() {
        Ok(mut touch_info) => {
            touch_info.editing_text = editing_text;
        }
        Err(poisoned) => {
            let _unused = poisoned.into_inner();
        }
    };
}

/// Listens to the [`TextAgentChannel`] channel and wraps messages into [`EguiInputEvent`] events.
pub fn write_text_agent_channel_events_system(
    channel: Res<TextAgentChannel>,
    focused_non_window_egui_context: Option<Res<FocusedNonWindowEguiContext>>,
    egui_contexts: Query<(Entity, &EguiContextSettings)>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
    mut redraw_event: EventWriter<RequestRedraw>,
) -> Result {
    let mut processed_entities = HashSet::new();

    for (context_entity, _context_settings) in egui_contexts {
        let mut redraw = false;
        let context = focused_non_window_egui_context
            .as_deref()
            .map_or(context_entity, |context| context.0);

        if !processed_entities.insert(context) {
            continue;
        }

        let (_, context_settings) = egui_contexts.get(context)?;

        if !context_settings
            .input_system_settings
            .run_write_text_agent_channel_messages_system
        {
            continue;
        }

        while let Ok(event) = channel.receiver.try_recv() {
            redraw = true;
            egui_input_event_writer.write(EguiInputEvent { context, event });
        }
        if redraw {
            redraw_event.write(RequestRedraw);
        }
    }

    Ok(())
}

/// Installs a text agent on startup.
pub fn install_text_agent_system(
    mut subscribed_events: NonSendMut<SubscribedEvents>,
    text_agent_channel: Res<TextAgentChannel>,
    safari_virtual_keyboard_touch_state: Res<SafariVirtualKeyboardTouchState>,
) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().expect("document should have a body");
    let input = document
        .create_element("input")
        .expect("failed to create input")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("failed input type coercion");
    let input = std::rc::Rc::new(input);
    input.set_type("text");
    if let Err(err) = (&input as &web_sys::HtmlElement).set_autofocus(true) {
        log::warn!("Failed to set input autofocus: {err:?}");
    }
    input
        .set_attribute("autocapitalize", "off")
        .expect("failed to turn off autocapitalize");
    input.set_id(AGENT_ID);
    {
        let style = input.style();
        // Make the input hidden.
        style
            .set_property("background-color", "transparent")
            .expect("failed to set text_agent css properties");
        style
            .set_property("border", "none")
            .expect("failed to set text_agent css properties");
        style
            .set_property("outline", "none")
            .expect("failed to set text_agent css properties");
        style
            .set_property("width", "1px")
            .expect("failed to set text_agent css properties");
        style
            .set_property("height", "1px")
            .expect("failed to set text_agent css properties");
        style
            .set_property("caret-color", "transparent")
            .expect("failed to set text_agent css properties");
        style
            .set_property("position", "absolute")
            .expect("failed to set text_agent css properties");
        style
            .set_property("top", "0")
            .expect("failed to set text_agent css properties");
        style
            .set_property("left", "0")
            .expect("failed to set text_agent css properties");
    }
    // Set size as small as possible, in case user may click on it.
    input.set_size(1);
    if let Err(err) = (&input as &web_sys::HtmlElement).set_autofocus(true) {
        log::warn!("Failed to set input autofocus: {err:?}");
    }
    input.set_hidden(true);

    let sender = text_agent_channel.sender.clone();

    if let Some(true) = is_mobile() {
        let input_clone = input.clone();
        let sender_clone = sender.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::InputEvent| {
            #[cfg(feature = "log_input_events")]
            log::warn!(
                "Input event: is_composing={}, data={:?}",
                event.is_composing(),
                event.data()
            );
            let text = input_clone.value();

            if !text.is_empty() && !event.is_composing() {
                input_clone.set_value("");
                input_clone.blur().ok();
                input_clone.focus().ok();
                if let Err(err) = sender_clone.send(egui::Event::Text(text.clone())) {
                    log::error!("Failed to send input event: {:?}", err);
                }
            }
        }) as Box<dyn FnMut(_)>);
        input
            .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
            .expect("failed to create input listener");
        subscribed_events.input_event_closures.push(EventClosure {
            target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                &document,
            )
            .clone(),
            event_name: "virtual_keyboard_input".to_owned(),
            closure,
        });

        let input_clone = input.clone();
        let sender_clone = sender.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::CompositionEvent| {
            #[cfg(feature = "log_input_events")]
            log::warn!("Composition start: data={:?}", _event.data());
            input_clone.set_value("");
            let _ = sender_clone.send(egui::Event::Ime(egui::ImeEvent::Enabled));
        }) as Box<dyn FnMut(_)>);
        input
            .add_event_listener_with_callback("compositionstart", closure.as_ref().unchecked_ref())
            .expect("failed to create compositionstart listener");
        subscribed_events
            .composition_event_closures
            .push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_compositionstart".to_owned(),
                closure,
            });

        let sender_clone = sender.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::CompositionEvent| {
            #[cfg(feature = "log_input_events")]
            log::warn!("Composition update: data={:?}", event.data());
            let Some(text) = event.data() else { return };
            let event = egui::Event::Ime(egui::ImeEvent::Preedit(text));
            let _ = sender_clone.send(event);
        }) as Box<dyn FnMut(_)>);
        input
            .add_event_listener_with_callback("compositionupdate", closure.as_ref().unchecked_ref())
            .expect("failed to create compositionupdate listener");
        subscribed_events
            .composition_event_closures
            .push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_compositionupdate".to_owned(),
                closure,
            });

        let input_clone = input.clone();
        let sender_clone = sender.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::CompositionEvent| {
            #[cfg(feature = "log_input_events")]
            log::warn!("Composition end: data={:?}", event.data());
            let Some(text) = event.data() else { return };
            input_clone.set_value("");
            let event = egui::Event::Ime(egui::ImeEvent::Commit(text));
            let _ = sender_clone.send(event);
        }) as Box<dyn FnMut(_)>);
        input
            .add_event_listener_with_callback("compositionend", closure.as_ref().unchecked_ref())
            .expect("failed to create compositionend listener");
        subscribed_events
            .composition_event_closures
            .push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_compositionend".to_owned(),
                closure,
            });

        // Mobile safari doesn't let you set input focus outside of an event handler.
        if is_mobile_safari() {
            let safari_sender = safari_virtual_keyboard_touch_state.sender.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::TouchEvent| {
                #[cfg(feature = "log_input_events")]
                log::warn!("Touch start: {:?}", _event);
                let _ = safari_sender.send(());
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback("touchstart", closure.as_ref().unchecked_ref())
                .expect("failed to create touchstart listener");
            subscribed_events.touch_event_closures.push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_touchstart".to_owned(),
                closure,
            });

            let safari_touch_info_lock = safari_virtual_keyboard_touch_state.touch_info;
            let closure = Closure::wrap(Box::new(move |_event: web_sys::TouchEvent| {
                #[cfg(feature = "log_input_events")]
                log::warn!("Touch end: {:?}", _event);
                match safari_touch_info_lock.lock() {
                    Ok(touch_info) => {
                        update_text_agent(touch_info.editing_text);
                    }
                    Err(poisoned) => {
                        let _unused = poisoned.into_inner();
                    }
                };
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback("touchend", closure.as_ref().unchecked_ref())
                .expect("failed to create touchend listener");
            subscribed_events.touch_event_closures.push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_touchend".to_owned(),
                closure,
            });
        }

        let sender_clone = sender.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            #[cfg(feature = "log_input_events")]
            log::warn!("Keyboard event: {:?}", event);
            if event.is_composing() || event.key_code() == 229 {
                // https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }
            if "Backspace" == event.key() {
                let _ = sender_clone.send(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                    repeat: false,
                });
            }
        }) as Box<dyn FnMut(_)>);
        document
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .expect("failed to create keydown listener");
        subscribed_events
            .keyboard_event_closures
            .push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_keydown".to_owned(),
                closure,
            });

        let input_clone = input.clone();
        let sender_clone = sender.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            #[cfg(feature = "log_input_events")]
            log::warn!("{:?}", event);
            input_clone.focus().ok();
            if "Backspace" == event.key() {
                let _ = sender_clone.send(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                    repeat: false,
                });
            }
        }) as Box<dyn FnMut(_)>);
        document
            .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())
            .expect("failed to create keyup listener");
        subscribed_events
            .keyboard_event_closures
            .push(EventClosure {
                target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                    &document,
                )
                .clone(),
                event_name: "virtual_keyboard_keyup".to_owned(),
                closure,
            });
    }

    body.append_child(&input).expect("failed to append to body");
}

/// Focus or blur text agent to toggle mobile keyboard.
pub fn update_text_agent(editing_text: bool) {
    use web_sys::HtmlInputElement;

    let window = match web_sys::window() {
        Some(window) => window,
        None => {
            bevy_log::error!("No window found");
            return;
        }
    };
    let document = match window.document() {
        Some(doc) => doc,
        None => {
            bevy_log::error!("No document found");
            return;
        }
    };
    let input: HtmlInputElement = match document.get_element_by_id(AGENT_ID) {
        Some(ele) => ele,
        None => {
            bevy_log::error!("Agent element not found");
            return;
        }
    }
    .dyn_into()
    .unwrap();

    let keyboard_open = !input.hidden();

    if editing_text {
        // Open the keyboard.
        input.set_hidden(false);
        match input.focus().ok() {
            Some(_) => {}
            None => {
                bevy_log::error!("Unable to set focus");
            }
        }
    } else if keyboard_open {
        // Close the keyboard.
        if input.blur().is_err() {
            bevy_log::error!("Agent element not found");
            return;
        }

        input.set_hidden(true);
    }
}

pub(crate) fn is_mobile_safari() -> bool {
    (|| {
        let user_agent = web_sys::window()?.navigator().user_agent().ok()?;
        let is_ios = user_agent.contains("iPhone")
            || user_agent.contains("iPad")
            || user_agent.contains("iPod");
        let is_safari = user_agent.contains("Safari");
        Some(is_ios && is_safari)
    })()
    .unwrap_or(false)
}

fn is_mobile() -> Option<bool> {
    const MOBILE_DEVICE: [&str; 6] = ["Android", "iPhone", "iPad", "iPod", "webOS", "BlackBerry"];

    let user_agent = web_sys::window()?.navigator().user_agent().ok()?;
    let is_mobile = MOBILE_DEVICE.iter().any(|&name| user_agent.contains(name));
    Some(is_mobile)
}
