use crate::{input::{EguiInputEvent, FocusedNonWindowEguiContext}, string_from_js_value, EguiClipboard, EguiContext, EguiContextSettings, EventClosure, PrimaryEguiContext, SubscribedEvents};
use bevy_ecs::prelude::*;
use bevy_log as log;
use bevy_window::PrimaryWindow;
use crossbeam_channel::{Receiver, Sender};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Startup system to initialize web clipboard events.
pub fn startup_setup_web_events_system(
    mut egui_clipboard: ResMut<EguiClipboard>,
    mut subscribed_events: NonSendMut<SubscribedEvents>,
) {
    let (tx, rx) = crossbeam_channel::unbounded();
    egui_clipboard.clipboard.event_receiver = Some(rx);
    setup_clipboard_copy(&mut subscribed_events, tx.clone());
    setup_clipboard_cut(&mut subscribed_events, tx.clone());
    setup_clipboard_paste(&mut subscribed_events, tx);
}

/// Receives web clipboard events and wraps them as [`EguiInputEvent`] events.
pub fn write_web_clipboard_events_system(
    focused_non_window_egui_context: Option<Res<FocusedNonWindowEguiContext>>,
    // We can safely assume that we have only 1 window in WASM.
    egui_context: Single<(Entity, &EguiContextSettings), (With<PrimaryEguiContext>, With<EguiContext>)>,
    mut egui_clipboard: ResMut<EguiClipboard>,
    mut egui_input_event_writer: EventWriter<EguiInputEvent>,
) {
    let (primary_context, context_settings) = *egui_context;
    if !context_settings
        .input_system_settings
        .run_write_web_clipboard_events_system
    {
        return;
    }

    let context = focused_non_window_egui_context
        .as_deref()
        .map_or(primary_context, |context| context.0);
    while let Some(event) = egui_clipboard.try_receive_clipboard_event() {
        match event {
            crate::web_clipboard::WebClipboardEvent::Copy => {
                egui_input_event_writer.write(EguiInputEvent {
                    context,
                    event: egui::Event::Copy,
                });
            }
            crate::web_clipboard::WebClipboardEvent::Cut => {
                egui_input_event_writer.write(EguiInputEvent {
                    context,
                    event: egui::Event::Cut,
                });
            }
            crate::web_clipboard::WebClipboardEvent::Paste(text) => {
                egui_clipboard.set_text_internal(&text);
                egui_input_event_writer.write(EguiInputEvent {
                    context,
                    event: egui::Event::Paste(text),
                });
            }
        }
    }
}

/// Internal implementation of `[crate::EguiClipboard]` for web.
#[derive(Default)]
pub struct WebClipboard {
    event_receiver: Option<Receiver<WebClipboardEvent>>,
    contents: Option<String>,
}

/// Events sent by the `cut`/`copy`/`paste` listeners.
#[derive(Debug)]
pub enum WebClipboardEvent {
    /// Is sent whenever the `cut` event listener is called.
    Cut,
    /// Is sent whenever the `copy` event listener is called.
    Copy,
    /// Is sent whenever the `paste` event listener is called, includes the plain text content.
    Paste(String),
}

impl WebClipboard {
    /// Places the text onto the clipboard.
    pub fn set_text(&mut self, text: &str) {
        self.set_text_internal(text);
        set_clipboard_text(text.to_owned());
    }

    /// Sets the internal buffer of clipboard contents.
    /// This buffer is used to remember the contents of the last `paste` event.
    pub fn set_text_internal(&mut self, text: &str) {
        self.contents = Some(text.to_owned());
    }

    /// Gets clipboard contents. Returns [`None`] if the `copy`/`cut` operation have never been invoked yet,
    /// or the `paste` event has never been received yet.
    pub fn get_text(&mut self) -> Option<String> {
        self.contents.clone()
    }

    /// Places the image onto the clipboard.
    pub fn set_image(&mut self, image: &egui::ColorImage) {
        self.contents = None;
        set_clipboard_image(image);
    }

    /// Receives a clipboard event sent by the `copy`/`cut`/`paste` listeners.
    pub fn try_receive_clipboard_event(&self) -> Option<WebClipboardEvent> {
        let Some(rx) = &self.event_receiver else {
            log::error!("Web clipboard event receiver isn't initialized");
            return None;
        };

        match rx.try_recv() {
            Ok(event) => Some(event),
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(err @ crossbeam_channel::TryRecvError::Disconnected) => {
                log::error!("Failed to read a web clipboard event: {err:?}");
                None
            }
        }
    }
}

fn setup_clipboard_copy(subscribed_events: &mut SubscribedEvents, tx: Sender<WebClipboardEvent>) {
    let Some(window) = web_sys::window() else {
        log::error!("Failed to add the \"copy\" listener: no window object");
        return;
    };
    let Some(document) = window.document() else {
        log::error!("Failed to add the \"copy\" listener: no document object");
        return;
    };

    let closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::ClipboardEvent| {
        if tx.send(WebClipboardEvent::Copy).is_err() {
            log::error!("Failed to send a \"copy\" event: channel is disconnected");
        }
    });

    let listener = closure.as_ref().unchecked_ref();

    if let Err(err) = document.add_event_listener_with_callback("copy", listener) {
        log::error!(
            "Failed to add the \"copy\" event listener: {}",
            string_from_js_value(&err)
        );
        drop(closure);
        return;
    };
    subscribed_events
        .clipboard_event_closures
        .push(EventClosure {
            target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                &document,
            )
            .clone(),
            event_name: "copy".to_owned(),
            closure,
        });
}

fn setup_clipboard_cut(subscribed_events: &mut SubscribedEvents, tx: Sender<WebClipboardEvent>) {
    let Some(window) = web_sys::window() else {
        log::error!("Failed to add the \"cut\" listener: no window object");
        return;
    };
    let Some(document) = window.document() else {
        log::error!("Failed to add the \"cut\" listener: no document object");
        return;
    };

    let closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::ClipboardEvent| {
        if tx.send(WebClipboardEvent::Cut).is_err() {
            log::error!("Failed to send a \"cut\" event: channel is disconnected");
        }
    });

    let listener = closure.as_ref().unchecked_ref();

    if let Err(err) = document.add_event_listener_with_callback("cut", listener) {
        log::error!(
            "Failed to add the \"cut\" event listener: {}",
            string_from_js_value(&err)
        );
        drop(closure);
        return;
    };
    subscribed_events
        .clipboard_event_closures
        .push(EventClosure {
            target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                &document,
            )
            .clone(),
            event_name: "cut".to_owned(),
            closure,
        });
}

fn setup_clipboard_paste(subscribed_events: &mut SubscribedEvents, tx: Sender<WebClipboardEvent>) {
    let Some(window) = web_sys::window() else {
        log::error!("Failed to add the \"paste\" listener: no window object");
        return;
    };
    let Some(document) = window.document() else {
        log::error!("Failed to add the \"paste\" listener: no document object");
        return;
    };

    let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::ClipboardEvent| {
        let Some(clipboard_data) = event.clipboard_data() else {
            log::error!("Failed to access clipboard data");
            return;
        };
        match clipboard_data.get_data("text/plain") {
            Ok(data) => {
                if tx.send(WebClipboardEvent::Paste(data)).is_err() {
                    log::error!("Failed to send the \"paste\" event: channel is disconnected");
                }
            }
            Err(err) => {
                log::error!(
                    "Failed to read clipboard data: {}",
                    string_from_js_value(&err)
                );
            }
        }
    });

    let listener = closure.as_ref().unchecked_ref();

    if let Err(err) = document.add_event_listener_with_callback("paste", listener) {
        log::error!(
            "Failed to add the \"paste\" event listener: {}",
            string_from_js_value(&err)
        );
        drop(closure);
        return;
    };
    subscribed_events
        .clipboard_event_closures
        .push(EventClosure {
            target: <web_sys::Document as std::convert::AsRef<web_sys::EventTarget>>::as_ref(
                &document,
            )
            .clone(),
            event_name: "paste".to_owned(),
            closure,
        });
}

fn set_clipboard_text(contents: String) {
    spawn_local(async move {
        let Some(window) = web_sys::window() else {
            log::warn!("Failed to access the window object");
            return;
        };

        let clipboard = window.navigator().clipboard();

        let promise = clipboard.write_text(&contents);
        if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
            log::warn!(
                "Failed to write to clipboard: {}",
                string_from_js_value(&err)
            );
        }
    });
}

fn set_clipboard_image(image: &egui::ColorImage) {
    if let Some(window) = web_sys::window() {
        if !window.is_secure_context() {
            log::error!(
                "Clipboard is not available because we are not in a secure context. \
                See https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts"
            );
            return;
        }

        let png_bytes = to_image(image).and_then(|image| to_png_bytes(&image));
        let png_bytes = match png_bytes {
            Ok(png_bytes) => png_bytes,
            Err(err) => {
                log::error!("Failed to encode image to png: {err}");
                return;
            }
        };

        let mime = "image/png";

        let item = match create_clipboard_item(mime, &png_bytes) {
            Ok(item) => item,
            Err(err) => {
                log::error!("Failed to copy image: {}", string_from_js_value(&err));
                return;
            }
        };
        let items = js_sys::Array::of1(&item);
        let promise = window.navigator().clipboard().write(&items);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let future = async move {
            if let Err(err) = future.await {
                log::error!(
                    "Copy/cut image action failed: {}",
                    string_from_js_value(&err)
                );
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    }
}

fn to_image(image: &egui::ColorImage) -> Result<image::RgbaImage, String> {
    image::RgbaImage::from_raw(
        image.width() as _,
        image.height() as _,
        bytemuck::cast_slice(&image.pixels).to_vec(),
    )
    .ok_or_else(|| "Invalid IconData".to_owned())
}

fn to_png_bytes(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut png_bytes: Vec<u8> = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|err| err.to_string())?;
    Ok(png_bytes)
}

// https://github.com/emilk/egui/blob/08c5a641a17580fb6cfac947aaf95634018abeb7/crates/eframe/src/web/mod.rs#L267
fn create_clipboard_item(mime: &str, bytes: &[u8]) -> Result<web_sys::ClipboardItem, JsValue> {
    let array = js_sys::Uint8Array::from(bytes);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array);

    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);

    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)?;

    let items = js_sys::Object::new();

    // SAFETY: I hope so
    #[allow(unsafe_code, unused_unsafe)] // Weird false positive
    unsafe {
        js_sys::Reflect::set(&items, &JsValue::from_str(mime), &blob)?
    };

    let clipboard_item = web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&items)?;

    Ok(clipboard_item)
}
