#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

//! This crate provides an [Egui](https://github.com/emilk/egui) integration for the [Bevy](https://github.com/bevyengine/bevy) game engine.
//!
//! **Trying out:**
//!
//! A basic WASM example is live at [vladbat00.github.io/bevy_egui/ui](https://vladbat00.github.io/bevy_egui/ui/).
//!
//! **Features:**
//! - Desktop and web platforms support
//! - Clipboard
//! - Opening URLs
//! - Multiple windows support (see [./examples/two_windows.rs](https://github.com/vladbat00/bevy_egui/blob/v0.29.0/examples/two_windows.rs))
//! - Paint callback support (see [./examples/paint_callback.rs](https://github.com/vladbat00/bevy_egui/blob/v0.29.0/examples/paint_callback.rs))
//! - Mobile web virtual keyboard (still rough around the edges and only works without `prevent_default_event_handling` set to `false` in the `WindowPlugin` settings)
//!
//! ## Dependencies
//!
//! On Linux, this crate requires certain parts of [XCB](https://xcb.freedesktop.org/) to be installed on your system. On Debian-based systems, these can be installed with the following command:
//!
//! ```bash
//! sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
//! ```
//!
//! ## Usage
//!
//! Here's a minimal usage example:
//!
//! ```no_run,rust
//! use bevy::prelude::*;
//! use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
//!         .add_systems(EguiContextPass, ui_example_system)
//!         .run();
//! }
//!
//! fn ui_example_system(mut contexts: EguiContexts) {
//!     egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
//!         ui.label("world");
//!     });
//! }
//! ```
//!
//! Note that this example uses Egui in the [multi-pass mode]((https://docs.rs/egui/0.31.1/egui/#multi-pass-immediate-mode)).
//! If you don't want to be limited to the [`EguiContextPass`] schedule, you can use the single-pass mode,
//! but it may get deprecated in the future.
//!
//! For more advanced examples, see the [examples](#examples) section below.
//!
//! ### Note to developers of public plugins
//!
//! If your plugin depends on `bevy_egui`, here are some hints on how to implement the support of both single-pass and multi-pass modes
//! (with respect to the [`EguiPlugin::enable_multipass_for_primary_context`] flag):
//! - Don't initialize [`EguiPlugin`] for the user, i.e. DO NOT use `add_plugins(EguiPlugin { ... })` in your code,
//!   users should be able to opt in or opt out of the multi-pass mode on their own.
//! - If you add UI systems, make sure they go into the [`EguiContextPass`] schedule - this will guarantee your plugin supports both the single-pass and multi-pass modes.
//!
//! Your plugin code might look like this:
//!
//! ```no_run,rust
//! # use bevy::prelude::*;
//! # use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass};
//!
//! pub struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn build(&self, app: &mut App) {
//!         // Don't add the plugin for users, let them chose the default mode themselves
//!         // and just make sure they initialize EguiPlugin before yours.
//!         assert!(app.is_plugin_added::<EguiPlugin>());
//!
//!         app.add_systems(EguiContextPass, ui_system);
//!     }
//! }
//!
//! fn ui_system(contexts: EguiContexts) {
//!     // ...
//! }
//! ```
//!
//! ## Examples
//!
//! To run an example, use the following command (you may replace `ui` with a name of another example):
//!
//! ```bash
//! cargo run --example ui
//! ```
//!
//! ### ui ([live page](https://vladbat00.github.io/bevy_egui/ui), source: [examples/ui.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/ui.rs))
//!
//! Showcasing some more advanced UI, rendering images, hidpi scaling.
//!
//! ### color_test ([live page](https://vladbat00.github.io/bevy_egui/color_test), source: [examples/color_test.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/color_test.rs))
//!
//! Rendering test from [egui.rs](https://egui.rs). We don't fully pass it, help is wanted ([#291](https://github.com/vladbat00/bevy_egui/issues/291)).
//!
//! ### side_panel_2d ([live page](https://vladbat00.github.io/bevy_egui/side_panel_2d), source: [examples/side_panel_2d.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/side_panel_2d.rs))
//!
//! Showing how to display an Egui side panel and transform a camera with a perspective projection to make rendering centered relative to the remaining screen area.
//!
//! ### side_panel_3d ([live page](https://vladbat00.github.io/bevy_egui/side_panel_3d), source: [examples/side_panel_3d.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/side_panel_3d.rs))
//!
//! Showing how to display an Egui side panel and transform a camera with a orthographic projection to make rendering centered relative to the remaining screen area.
//!
//! ### render_egui_to_image ([live page](https://vladbat00.github.io/bevy_egui/render_egui_to_image), source: [examples/render_egui_to_image.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/render_egui_to_image.rs))
//!
//! Rendering UI to an image (texture) and then using it as a mesh material texture.
//!
//! ### render_to_image_widget ([live page](https://vladbat00.github.io/bevy_egui/render_to_image_widget), source: [examples/render_to_image_widget.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/render_to_image_widget.rs))
//!
//! Rendering to a texture with Bevy and showing it as an Egui image widget.
//!
//! ### two_windows (source: [examples/two_windows.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/two_windows.rs))
//!
//! Setting up two windows with an Egui context for each.
//!
//! ### paint_callback ([live page](https://vladbat00.github.io/bevy_egui/paint_callback), source: [examples/paint_callback.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/paint_callback.rs))
//!
//! Using Egui paint callbacks.
//!
//! ### simple ([live page](https://vladbat00.github.io/bevy_egui/simple), source: [examples/simple.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/simple.rs))
//!
//! The minimal usage example from this readme.
//!
//! ### simple_multipass ([live page](https://vladbat00.github.io/bevy_egui/simple_multipass), source: [examples/simple_multipass.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/simple_multipass.rs))
//!
//! The same minimal example demonstrating running Egui passes manually.
//!
//! ## See also
//!
//! - [`bevy-inspector-egui`](https://github.com/jakobhellermann/bevy-inspector-egui)

/// Egui render node.
#[cfg(feature = "render")]
pub mod egui_node;
/// Helpers for converting Bevy types into Egui ones and vice versa.
pub mod helpers;
/// Systems for translating Bevy input events into Egui input.
pub mod input;
/// Systems for handling Egui output.
pub mod output;
/// Plugin systems for the render app.
#[cfg(feature = "render")]
pub mod render_systems;
/// Mobile web keyboard input support.
#[cfg(target_arch = "wasm32")]
pub mod text_agent;
/// Clipboard management for web.
#[cfg(all(feature = "manage_clipboard", target_arch = "wasm32",))]
pub mod web_clipboard;

use bevy_a11y::{AccessibilityRequested, AccessibilitySystem, ManageAccessibilityUpdates};
pub use egui;

use crate::input::*;
#[cfg(target_arch = "wasm32")]
use crate::text_agent::{
    install_text_agent_system, is_mobile_safari, process_safari_virtual_keyboard_system,
    write_text_agent_channel_events_system, SafariVirtualKeyboardTouchState, TextAgentChannel,
    VirtualTouchInfo,
};
#[cfg(feature = "render")]
use crate::{
    egui_node::{EguiPipeline, EGUI_SHADER_HANDLE},
    render_systems::{EguiRenderData, EguiTransforms, ExtractedEguiManagedTextures},
};
#[cfg(all(
    feature = "manage_clipboard",
    not(any(target_arch = "wasm32", target_os = "android"))
))]
use arboard::Clipboard;
use bevy_app::prelude::*;
#[cfg(feature = "render")]
use bevy_asset::{load_internal_asset, AssetEvent, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::*,
    query::{QueryData, QueryEntityError},
    schedule::{apply_deferred, InternedScheduleLabel, ScheduleLabel},
    system::SystemParam,
};
#[cfg(feature = "render")]
use bevy_image::{Image, ImageSampler};
use bevy_input::InputSystem;
use bevy_log as log;
#[cfg(feature = "picking")]
use bevy_picking::{
    backend::{HitData, PointerHits},
    pointer::{PointerId, PointerLocation},
};
use bevy_reflect::Reflect;
#[cfg(feature = "picking")]
use bevy_render::camera::NormalizedRenderTarget;
#[cfg(feature = "render")]
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::{LoadOp, SpecializedRenderPipelines},
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::HashSet;
use bevy_window::{PrimaryWindow, Window};
use bevy_winit::{accessibility::AccessKitAdapters, cursor::CursorIcon};
use output::process_output_system;
#[cfg(all(
    feature = "manage_clipboard",
    not(any(target_arch = "wasm32", target_os = "android"))
))]
use std::cell::{RefCell, RefMut};
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Adds all Egui resources and render graph nodes.
pub struct EguiPlugin {
    /// ## About Egui multi-pass mode
    ///
    /// _From the [Egui documentation](https://docs.rs/egui/0.31.1/egui/#multi-pass-immediate-mode):_
    ///
    /// By default, egui usually only does one pass for each rendered frame.
    /// However, egui supports multi-pass immediate mode.
    /// Another pass can be requested with [`egui::Context::request_discard`].
    ///
    /// This is used by some widgets to cover up "first-frame jitters".
    /// For instance, the [`egui::Grid`] needs to know the width of all columns before it can properly place the widgets.
    /// But it cannot know the width of widgets to come.
    /// So it stores the max widths of previous frames and uses that.
    /// This means the first time a `Grid` is shown it will _guess_ the widths of the columns, and will usually guess wrong.
    /// This means the contents of the grid will be wrong for one frame, before settling to the correct places.
    /// Therefore `Grid` calls [`egui::Context::request_discard`] when it is first shown, so the wrong placement is never
    /// visible to the end user.
    ///
    /// ## Usage
    ///
    /// Set this to `true` to enable an experimental support for the Egui multi-pass mode.
    ///
    /// Enabling the multi-pass mode will require your app to use the new [`EguiContextPass`] schedule:
    ///
    /// ```no_run,rust
    /// # use bevy::prelude::*;
    /// # use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass};
    /// fn main() {
    ///     App::new()
    ///         .add_plugins(DefaultPlugins)
    ///         .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
    ///         .add_systems(EguiContextPass, ui_example_system)
    ///         .run();
    /// }
    /// fn ui_example_system(contexts: EguiContexts) {
    ///     // ...
    /// }
    /// ```
    ///
    /// If you create multiple contexts (for example, when using multiple windows or rendering to an image),
    /// you need to define a custom schedule and assign it to additional contexts manually:
    ///
    /// ```no_run,rust
    /// # use bevy::prelude::*;
    /// # use bevy::ecs::schedule::ScheduleLabel;
    /// # use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass, EguiMultipassSchedule};
    /// #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    /// pub struct SecondWindowContextPass;
    ///
    /// fn main() {
    ///     App::new()
    ///         .add_plugins(DefaultPlugins)
    ///         .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
    ///         .add_systems(Startup, create_new_window_system)
    ///         .add_systems(EguiContextPass, ui_example_system)
    ///         .add_systems(SecondWindowContextPass, ui_example_system)
    ///         .run();
    /// }
    ///
    /// fn create_new_window_system(mut commands: Commands) {
    ///     commands.spawn((Window::default(), EguiMultipassSchedule::new(SecondWindowContextPass)));
    /// }
    ///
    /// fn ui_example_system(contexts: EguiContexts) {
    ///     // ...
    /// }
    /// ```
    ///
    /// In the future, the multi-pass mode will likely phase the single-pass one out.
    ///
    /// ## Note to developers of public plugins
    ///
    /// If your plugin depends on `bevy_egui`, here are some hints on how to implement the support of both single-pass and multi-pass modes
    /// (with respect to the [`EguiPlugin::enable_multipass_for_primary_context`] flag):
    /// - Don't initialize [`EguiPlugin`] for the user, i.e. DO NOT use `add_plugins(EguiPlugin { ... })` in your code,
    ///   users should be able to opt in or opt out of the multi-pass mode on their own.
    /// - If you add UI systems, make sure they go into the [`EguiContextPass`] schedule - this will guarantee your plugin supports both the single-pass and multi-pass modes.
    ///
    /// Your plugin code might look like this:
    ///
    /// ```no_run,rust
    /// # use bevy::prelude::*;
    /// # use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass};
    ///
    /// pub struct MyPlugin;
    ///
    /// impl Plugin for MyPlugin {
    ///     fn build(&self, app: &mut App) {
    ///         // Don't add the plugin for users, let them chose the default mode themselves
    ///         // and just make sure they initialize EguiPlugin before yours.
    ///         assert!(app.is_plugin_added::<EguiPlugin>());
    ///
    ///         app.add_systems(EguiContextPass, ui_system);
    ///     }
    /// }
    ///
    /// fn ui_system(contexts: EguiContexts) {
    ///     // ...
    /// }
    /// ```
    pub enable_multipass_for_primary_context: bool,
}

/// A resource for storing global plugin settings.
#[derive(Clone, Debug, Resource, Reflect)]
pub struct EguiGlobalSettings {
    /// Set this to `false` if you want to disable updating focused contexts by the plugin's systems
    /// (enabled by default).
    ///
    /// For more info, see the [`FocusedNonWindowEguiContext`] documentation.
    pub enable_focused_non_window_context_updates: bool,
    /// Controls running of the input systems.
    pub input_system_settings: EguiInputSystemSettings,
    /// Controls running of the [`absorb_bevy_input_system`] system, disabled by default.
    ///
    /// ## Considerations
    ///
    /// Enabling this system makes an assumption that `bevy_egui` takes priority in input handling
    /// over other plugins and systems. This should work ok as long as there's no other system
    /// clearing events the same way that might be in conflict with `bevy_egui`, and there's
    /// no other system that needs a non-interrupted flow of events.
    ///
    /// ## Alternative
    ///
    /// Apply `run_if(not(egui_wants_input))` to your systems that need to be disabled while
    /// Egui is using input.
    pub enable_absorb_bevy_input_system: bool,
}

impl Default for EguiGlobalSettings {
    fn default() -> Self {
        Self {
            enable_focused_non_window_context_updates: true,
            input_system_settings: EguiInputSystemSettings::default(),
            enable_absorb_bevy_input_system: false,
        }
    }
}

/// This resource is created if [`EguiPlugin`] is initialized with [`EguiPlugin::enable_multipass_for_primary_context`] set to `true`.
#[derive(Resource)]
pub struct EnableMultipassForPrimaryContext;

/// A component for storing Egui context settings.
#[derive(Clone, Debug, Component, Reflect)]
#[cfg_attr(feature = "render", derive(ExtractComponent))]
pub struct EguiContextSettings {
    /// If set to `true`, a user is expected to call [`egui::Context::run`] or [`egui::Context::begin_pass`] and [`egui::Context::end_pass`] manually.
    pub run_manually: bool,
    /// Global scale factor for Egui widgets (`1.0` by default).
    ///
    /// This setting can be used to force the UI to render in physical pixels regardless of DPI as follows:
    /// ```rust
    /// use bevy::{prelude::*, window::PrimaryWindow};
    /// use bevy_egui::EguiContextSettings;
    ///
    /// fn update_ui_scale_factor(mut windows: Query<(&mut EguiContextSettings, &Window), With<PrimaryWindow>>) {
    ///     if let Ok((mut egui_settings, window)) = windows.get_single_mut() {
    ///         egui_settings.scale_factor = 1.0 / window.scale_factor();
    ///     }
    /// }
    /// ```
    pub scale_factor: f32,
    /// Is used as a default value for hyperlink [target](https://www.w3schools.com/tags/att_a_target.asp) hints.
    /// If not specified, `_self` will be used. Only matters in a web browser.
    #[cfg(feature = "open_url")]
    pub default_open_url_target: Option<String>,
    /// Controls if Egui should capture pointer input when using [`bevy_picking`] (i.e. suppress `bevy_picking` events when a pointer is over an Egui window).
    #[cfg(feature = "picking")]
    pub capture_pointer_input: bool,
    /// Controls running of the input systems.
    pub input_system_settings: EguiInputSystemSettings,
}

// Just to keep the PartialEq
impl PartialEq for EguiContextSettings {
    #[allow(clippy::let_and_return)]
    fn eq(&self, other: &Self) -> bool {
        let eq = self.scale_factor == other.scale_factor;
        #[cfg(feature = "open_url")]
        let eq = eq && self.default_open_url_target == other.default_open_url_target;
        eq
    }
}

impl Default for EguiContextSettings {
    fn default() -> Self {
        Self {
            run_manually: false,
            scale_factor: 1.0,
            #[cfg(feature = "open_url")]
            default_open_url_target: None,
            #[cfg(feature = "picking")]
            capture_pointer_input: true,
            input_system_settings: EguiInputSystemSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Reflect, PartialEq, Eq)]
/// All the systems are enabled by default. These settings exist within both [`EguiGlobalSettings`] and [`EguiContextSettings`].
pub struct EguiInputSystemSettings {
    /// Controls running of the [`write_modifiers_keys_state_system`] system.
    pub run_write_modifiers_keys_state_system: bool,
    /// Controls running of the [`write_window_pointer_moved_events_system`] system.
    pub run_write_window_pointer_moved_events_system: bool,
    /// Controls running of the [`write_pointer_button_events_system`] system.
    pub run_write_pointer_button_events_system: bool,
    /// Controls running of the [`write_window_touch_events_system`] system.
    pub run_write_window_touch_events_system: bool,
    /// Controls running of the [`write_non_window_pointer_moved_events_system`] system.
    pub run_write_non_window_pointer_moved_events_system: bool,
    /// Controls running of the [`write_mouse_wheel_events_system`] system.
    pub run_write_mouse_wheel_events_system: bool,
    /// Controls running of the [`write_non_window_touch_events_system`] system.
    pub run_write_non_window_touch_events_system: bool,
    /// Controls running of the [`write_keyboard_input_events_system`] system.
    pub run_write_keyboard_input_events_system: bool,
    /// Controls running of the [`write_ime_events_system`] system.
    pub run_write_ime_events_system: bool,
    /// Controls running of the [`write_text_agent_channel_events_system`] system.
    #[cfg(target_arch = "wasm32")]
    pub run_write_text_agent_channel_events_system: bool,
    /// Controls running of the [`web_clipboard::write_web_clipboard_events_system`] system.
    #[cfg(all(feature = "manage_clipboard", target_arch = "wasm32"))]
    pub run_write_web_clipboard_events_system: bool,
}

impl Default for EguiInputSystemSettings {
    fn default() -> Self {
        Self {
            run_write_modifiers_keys_state_system: true,
            run_write_window_pointer_moved_events_system: true,
            run_write_pointer_button_events_system: true,
            run_write_window_touch_events_system: true,
            run_write_non_window_pointer_moved_events_system: true,
            run_write_mouse_wheel_events_system: true,
            run_write_non_window_touch_events_system: true,
            run_write_keyboard_input_events_system: true,
            run_write_ime_events_system: true,
            #[cfg(target_arch = "wasm32")]
            run_write_text_agent_channel_events_system: true,
            #[cfg(all(feature = "manage_clipboard", target_arch = "wasm32"))]
            run_write_web_clipboard_events_system: true,
        }
    }
}

/// Use this schedule to run your UI systems with the primary Egui context.
/// (Mandatory if the context is running in the multi-pass mode.)
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EguiContextPass;

/// Add this component to your additional Egui contexts (e.g. when rendering to a new window or an image),
/// to enable multi-pass support. Note that each Egui context running in the multi-pass mode must use a unique schedule.
#[derive(Component, Clone)]
pub struct EguiMultipassSchedule(pub InternedScheduleLabel);

impl EguiMultipassSchedule {
    /// Constructs the component from a schedule label.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self(schedule.intern())
    }
}

/// Is used for storing Egui context input.
///
/// It gets reset during the [`crate::EguiInputSet::WriteEguiEvents`] system set.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
pub struct EguiInput(pub egui::RawInput);

/// Intermediate output buffer generated on an Egui pass end and consumed by the [`process_output_system`] system.
#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct EguiFullOutput(pub Option<egui::FullOutput>);

/// A resource for accessing clipboard.
///
/// The resource is available only if `manage_clipboard` feature is enabled.
#[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
#[derive(Default, bevy_ecs::system::Resource)]
pub struct EguiClipboard {
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: thread_local::ThreadLocal<Option<RefCell<Clipboard>>>,
    #[cfg(target_arch = "wasm32")]
    clipboard: web_clipboard::WebClipboard,
}

/// Is used for storing Egui shapes and textures delta.
#[derive(Component, Clone, Default, Debug)]
#[cfg_attr(feature = "render", derive(ExtractComponent))]
pub struct EguiRenderOutput {
    /// Pairs of rectangles and paint commands.
    ///
    /// The field gets populated during the [`EguiPostUpdateSet::ProcessOutput`] system (belonging to bevy's [`PostUpdate`])
    /// and processed during [`egui_node::EguiNode`]'s `update`.
    ///
    /// Value is wrapped in [`Arc`] to improve [`ExtractComponent`] performance.
    pub paint_jobs: Arc<Vec<egui::ClippedPrimitive>>,

    /// The change in egui textures since last frame.
    ///
    /// Value is wrapped in [`Arc`] to improve [`ExtractComponent`] performance.
    pub textures_delta: Arc<egui::TexturesDelta>,
}

impl EguiRenderOutput {
    /// Returns `true` if the output has no Egui shapes and no textures delta.
    pub fn is_empty(&self) -> bool {
        self.paint_jobs.is_empty() && self.textures_delta.is_empty()
    }
}

/// Stores last Egui output.
#[derive(Component, Clone, Default)]
pub struct EguiOutput {
    /// The field gets updated during [`process_output_system`] (in the [`EguiPostUpdateSet::ProcessOutput`] set, belonging to [`PostUpdate`]).
    pub platform_output: egui::PlatformOutput,
}

/// A component for storing `bevy_egui` context.
#[derive(Clone, Component, Default)]
#[cfg_attr(feature = "render", derive(ExtractComponent))]
#[require(
    EguiContextSettings,
    EguiInput,
    EguiContextPointerPosition,
    EguiContextPointerTouchId,
    EguiContextImeState,
    EguiFullOutput,
    EguiRenderOutput,
    EguiOutput,
    RenderTargetSize,
    CursorIcon
)]
pub struct EguiContext {
    ctx: egui::Context,
}

impl EguiContext {
    /// Borrows the underlying Egui context immutably.
    ///
    /// Even though the mutable borrow isn't necessary, as the context is wrapped into `RwLock`,
    /// using the immutable getter is gated with the `immutable_ctx` feature. Using the immutable
    /// borrow is discouraged as it may cause unpredictable blocking in UI systems.
    ///
    /// When the context is queried with `&mut EguiContext`, the Bevy scheduler is able to make
    /// sure that the context isn't accessed concurrently and can perform other useful work
    /// instead of busy-waiting.
    #[cfg(feature = "immutable_ctx")]
    #[must_use]
    pub fn get(&self) -> &egui::Context {
        &self.ctx
    }

    /// Borrows the underlying Egui context mutably.
    ///
    /// Even though the mutable borrow isn't necessary, as the context is wrapped into `RwLock`,
    /// using the immutable getter is gated with the `immutable_ctx` feature. Using the immutable
    /// borrow is discouraged as it may cause unpredictable blocking in UI systems.
    ///
    /// When the context is queried with `&mut EguiContext`, the Bevy scheduler is able to make
    /// sure that the context isn't accessed concurrently and can perform other useful work
    /// instead of busy-waiting.
    #[must_use]
    pub fn get_mut(&mut self) -> &mut egui::Context {
        &mut self.ctx
    }
}

#[cfg(not(feature = "render"))]
type EguiContextsFilter = With<Window>;

#[cfg(feature = "render")]
type EguiContextsFilter = Or<(With<Window>, With<EguiRenderToImage>)>;

#[derive(SystemParam)]
/// A helper SystemParam that provides a way to get [`EguiContext`] with less boilerplate and
/// combines a proxy interface to the [`EguiUserTextures`] resource.
pub struct EguiContexts<'w, 's> {
    q: Query<
        'w,
        's,
        (
            Entity,
            &'static mut EguiContext,
            Option<&'static PrimaryWindow>,
        ),
        EguiContextsFilter,
    >,
    #[cfg(feature = "render")]
    user_textures: ResMut<'w, EguiUserTextures>,
}

impl EguiContexts<'_, '_> {
    /// Egui context of the primary window.
    #[must_use]
    pub fn ctx_mut(&mut self) -> &mut egui::Context {
        self.try_ctx_mut()
            .expect("`EguiContexts::ctx_mut` was called for an uninitialized context (primary window), make sure your system is run after [`EguiPreUpdateSet::InitContexts`] (or [`EguiStartupSet::InitContexts`] for startup systems)")
    }

    /// Fallible variant of [`EguiContexts::ctx_mut`].
    #[must_use]
    pub fn try_ctx_mut(&mut self) -> Option<&mut egui::Context> {
        self.q
            .iter_mut()
            .find_map(|(_window_entity, ctx, primary_window)| {
                if primary_window.is_some() {
                    Some(ctx.into_inner().get_mut())
                } else {
                    None
                }
            })
    }

    /// Egui context of a specific entity.
    #[must_use]
    pub fn ctx_for_entity_mut(&mut self, entity: Entity) -> &mut egui::Context {
        self.try_ctx_for_entity_mut(entity)
            .unwrap_or_else(|| panic!("`EguiContexts::ctx_for_window_mut` was called for an uninitialized context (entity {entity:?}), make sure your system is run after [`EguiPreUpdateSet::InitContexts`] (or [`EguiStartupSet::InitContexts`] for startup systems)"))
    }

    /// Fallible variant of [`EguiContexts::ctx_for_entity_mut`].
    #[must_use]
    #[track_caller]
    pub fn try_ctx_for_entity_mut(&mut self, entity: Entity) -> Option<&mut egui::Context> {
        self.q
            .iter_mut()
            .find_map(|(window_entity, ctx, _primary_window)| {
                if window_entity == entity {
                    Some(ctx.into_inner().get_mut())
                } else {
                    None
                }
            })
    }

    /// Allows to get multiple contexts at the same time. This function is useful when you want
    /// to get multiple window contexts without using the `immutable_ctx` feature.
    #[track_caller]
    pub fn ctx_for_entities_mut<const N: usize>(
        &mut self,
        ids: [Entity; N],
    ) -> Result<[&mut egui::Context; N], QueryEntityError> {
        self.q
            .get_many_mut(ids)
            .map(|arr| arr.map(|(_window_entity, ctx, _primary_window)| ctx.into_inner().get_mut()))
    }

    /// Egui context of the primary window.
    ///
    /// Even though the mutable borrow isn't necessary, as the context is wrapped into `RwLock`,
    /// using the immutable getter is gated with the `immutable_ctx` feature. Using the immutable
    /// borrow is discouraged as it may cause unpredictable blocking in UI systems.
    ///
    /// When the context is queried with `&mut EguiContext`, the Bevy scheduler is able to make
    /// sure that the context isn't accessed concurrently and can perform other useful work
    /// instead of busy-waiting.
    #[cfg(feature = "immutable_ctx")]
    #[must_use]
    pub fn ctx(&self) -> &egui::Context {
        self.try_ctx()
            .expect("`EguiContexts::ctx` was called for an uninitialized context (primary window), make sure your system is run after [`EguiPreUpdateSet::InitContexts`] (or [`EguiStartupSet::InitContexts`] for startup systems)")
    }

    /// Fallible variant of [`EguiContexts::ctx`].
    ///
    /// Even though the mutable borrow isn't necessary, as the context is wrapped into `RwLock`,
    /// using the immutable getter is gated with the `immutable_ctx` feature. Using the immutable
    /// borrow is discouraged as it may cause unpredictable blocking in UI systems.
    ///
    /// When the context is queried with `&mut EguiContext`, the Bevy scheduler is able to make
    /// sure that the context isn't accessed concurrently and can perform other useful work
    /// instead of busy-waiting.
    #[cfg(feature = "immutable_ctx")]
    #[must_use]
    pub fn try_ctx(&self) -> Option<&egui::Context> {
        self.q
            .iter()
            .find_map(|(_window_entity, ctx, primary_window)| {
                if primary_window.is_some() {
                    Some(ctx.get())
                } else {
                    None
                }
            })
    }

    /// Egui context of a specific window.
    ///
    /// Even though the mutable borrow isn't necessary, as the context is wrapped into `RwLock`,
    /// using the immutable getter is gated with the `immutable_ctx` feature. Using the immutable
    /// borrow is discouraged as it may cause unpredictable blocking in UI systems.
    ///
    /// When the context is queried with `&mut EguiContext`, the Bevy scheduler is able to make
    /// sure that the context isn't accessed concurrently and can perform other useful work
    /// instead of busy-waiting.
    #[must_use]
    #[cfg(feature = "immutable_ctx")]
    pub fn ctx_for_entity(&self, entity: Entity) -> &egui::Context {
        self.try_ctx_for_entity(entity)
            .unwrap_or_else(|| panic!("`EguiContexts::ctx_for_entity` was called for an uninitialized context (entity {entity:?}), make sure your system is run after [`EguiPreUpdateSet::InitContexts`] (or [`EguiStartupSet::InitContexts`] for startup systems)"))
    }

    /// Fallible variant of [`EguiContexts::ctx_for_entity`].
    ///
    /// Even though the mutable borrow isn't necessary, as the context is wrapped into `RwLock`,
    /// using the immutable getter is gated with the `immutable_ctx` feature. Using the immutable
    /// borrow is discouraged as it may cause unpredictable blocking in UI systems.
    ///
    /// When the context is queried with `&mut EguiContext`, the Bevy scheduler is able to make
    /// sure that the context isn't accessed concurrently and can perform other useful work
    /// instead of busy-waiting.
    #[must_use]
    #[track_caller]
    #[cfg(feature = "immutable_ctx")]
    pub fn try_ctx_for_entity(&self, entity: Entity) -> Option<&egui::Context> {
        self.q
            .iter()
            .find_map(|(window_entity, ctx, _primary_window)| {
                if window_entity == entity {
                    Some(ctx.get())
                } else {
                    None
                }
            })
    }

    /// Can accept either a strong or a weak handle.
    ///
    /// You may want to pass a weak handle if you control removing texture assets in your
    /// application manually and don't want to bother with cleaning up textures in Egui.
    /// (The cleanup happens in [`free_egui_textures_system`].)
    ///
    /// You'll want to pass a strong handle if a texture is used only in Egui and there are no
    /// handle copies stored anywhere else.
    #[cfg(feature = "render")]
    pub fn add_image(&mut self, image: Handle<Image>) -> egui::TextureId {
        self.user_textures.add_image(image)
    }

    /// Removes the image handle and an Egui texture id associated with it.
    #[cfg(feature = "render")]
    #[track_caller]
    pub fn remove_image(&mut self, image: &Handle<Image>) -> Option<egui::TextureId> {
        self.user_textures.remove_image(image)
    }

    /// Returns an associated Egui texture id.
    #[cfg(feature = "render")]
    #[must_use]
    #[track_caller]
    pub fn image_id(&self, image: &Handle<Image>) -> Option<egui::TextureId> {
        self.user_textures.image_id(image)
    }
}

/// Contexts with this component will render UI to a specified image.
///
/// You can create an entity just with this component, `bevy_egui` will initialize an [`EguiContext`]
/// automatically.
#[cfg(feature = "render")]
#[derive(Component, Clone, Debug, ExtractComponent)]
#[require(EguiContext)]
pub struct EguiRenderToImage {
    /// A handle of an image to render to.
    pub handle: Handle<Image>,
    /// Customizable [`LoadOp`] for the render node which will be created for this context.
    ///
    /// You'll likely want [`LoadOp::Clear`], unless you need to draw the UI on top of existing
    /// pixels of the image.
    pub load_op: LoadOp<wgpu_types::Color>,
}

#[cfg(feature = "render")]
impl EguiRenderToImage {
    /// Creates a component from an image handle and sets [`EguiRenderToImage::load_op`] to [`LoadOp::Clear].
    pub fn new(handle: Handle<Image>) -> Self {
        Self {
            handle,
            load_op: LoadOp::Clear(wgpu_types::Color::TRANSPARENT),
        }
    }
}

/// A resource for storing `bevy_egui` user textures.
#[derive(Clone, bevy_ecs::system::Resource, ExtractResource)]
#[cfg(feature = "render")]
pub struct EguiUserTextures {
    textures: bevy_utils::HashMap<Handle<Image>, u64>,
    free_list: Vec<u64>,
}

#[cfg(feature = "render")]
impl Default for EguiUserTextures {
    fn default() -> Self {
        Self {
            textures: bevy_utils::HashMap::new(),
            free_list: vec![0],
        }
    }
}

#[cfg(feature = "render")]
impl EguiUserTextures {
    /// Can accept either a strong or a weak handle.
    ///
    /// You may want to pass a weak handle if you control removing texture assets in your
    /// application manually and don't want to bother with cleaning up textures in Egui.
    /// (The cleanup happens in [`free_egui_textures_system`].)
    ///
    /// You'll want to pass a strong handle if a texture is used only in Egui and there are no
    /// handle copies stored anywhere else.
    pub fn add_image(&mut self, image: Handle<Image>) -> egui::TextureId {
        let id = *self.textures.entry(image.clone()).or_insert_with(|| {
            let id = self
                .free_list
                .pop()
                .expect("free list must contain at least 1 element");
            log::debug!("Add a new image (id: {}, handle: {:?})", id, image);
            if self.free_list.is_empty() {
                self.free_list.push(id.checked_add(1).expect("out of ids"));
            }
            id
        });
        egui::TextureId::User(id)
    }

    /// Removes the image handle and an Egui texture id associated with it.
    pub fn remove_image(&mut self, image: &Handle<Image>) -> Option<egui::TextureId> {
        let id = self.textures.remove(image);
        log::debug!("Remove image (id: {:?}, handle: {:?})", id, image);
        if let Some(id) = id {
            self.free_list.push(id);
        }
        id.map(egui::TextureId::User)
    }

    /// Returns an associated Egui texture id.
    #[must_use]
    pub fn image_id(&self, image: &Handle<Image>) -> Option<egui::TextureId> {
        self.textures
            .get(image)
            .map(|&id| egui::TextureId::User(id))
    }
}

/// Stores physical size and scale factor, is used as a helper to calculate logical size.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "render", derive(ExtractComponent))]
pub struct RenderTargetSize {
    /// Physical width
    pub physical_width: f32,
    /// Physical height
    pub physical_height: f32,
    /// Scale factor
    pub scale_factor: f32,
}

impl RenderTargetSize {
    fn new(physical_width: f32, physical_height: f32, scale_factor: f32) -> Self {
        Self {
            physical_width,
            physical_height,
            scale_factor,
        }
    }

    /// Returns the width of the render target.
    #[inline]
    pub fn width(&self) -> f32 {
        self.physical_width / self.scale_factor
    }

    /// Returns the height of the render target.
    #[inline]
    pub fn height(&self) -> f32 {
        self.physical_height / self.scale_factor
    }
}

/// The names of `bevy_egui` nodes.
pub mod node {
    /// The main egui pass.
    pub const EGUI_PASS: &str = "egui_pass";
}

#[derive(SystemSet, Clone, Hash, Debug, Eq, PartialEq)]
/// The `bevy_egui` plugin startup system sets.
pub enum EguiStartupSet {
    /// Initializes Egui contexts for available windows.
    InitContexts,
}

/// System sets that run during the [`PreUpdate`] schedule.
#[derive(SystemSet, Clone, Hash, Debug, Eq, PartialEq)]
pub enum EguiPreUpdateSet {
    /// Initializes Egui contexts for newly created render targets.
    InitContexts,
    /// Reads Egui inputs (keyboard, mouse, etc) and writes them into the [`EguiInput`] resource.
    ///
    /// To modify the input, you can hook your system like this:
    ///
    /// `system.after(EguiPreUpdateSet::ProcessInput).before(EguiSet::BeginPass)`.
    ProcessInput,
    /// Begins the `egui` pass.
    BeginPass,
}

/// Subsets of the [`EguiPreUpdateSet::ProcessInput`] set.
#[derive(SystemSet, Clone, Hash, Debug, Eq, PartialEq)]
pub enum EguiInputSet {
    /// Reads key modifiers state and pointer positions.
    ///
    /// This is where [`HoveredNonWindowEguiContext`] should get inserted or removed.
    InitReading,
    /// Processes window mouse button click and touch events, updates [`FocusedNonWindowEguiContext`] based on [`HoveredNonWindowEguiContext`].
    FocusContext,
    /// Processes rest of the events for both window and non-window contexts.
    ReadBevyEvents,
    /// Feeds all the events into [`EguiInput`].
    WriteEguiEvents,
}

/// System sets that run during the [`PostUpdate`] schedule.
#[derive(SystemSet, Clone, Hash, Debug, Eq, PartialEq)]
pub enum EguiPostUpdateSet {
    /// Ends Egui pass.
    EndPass,
    /// Processes Egui output, reads paint jobs for the renderer.
    ProcessOutput,
    /// Post-processing of Egui output (updates textures, browser virtual keyboard state, etc).
    PostProcessOutput,
}

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<EguiGlobalSettings>();
        app.register_type::<EguiContextSettings>();
        app.init_resource::<EguiGlobalSettings>();
        app.init_resource::<ModifierKeysState>();
        app.init_resource::<EguiWantsInput>();
        app.add_event::<EguiInputEvent>();

        if self.enable_multipass_for_primary_context {
            app.insert_resource(EnableMultipassForPrimaryContext);
        }

        #[cfg(feature = "render")]
        {
            app.init_resource::<EguiManagedTextures>();
            app.init_resource::<EguiUserTextures>();
            app.add_plugins(ExtractResourcePlugin::<EguiUserTextures>::default());
            app.add_plugins(ExtractResourcePlugin::<ExtractedEguiManagedTextures>::default());
            app.add_plugins(ExtractComponentPlugin::<EguiContext>::default());
            app.add_plugins(ExtractComponentPlugin::<EguiContextSettings>::default());
            app.add_plugins(ExtractComponentPlugin::<RenderTargetSize>::default());
            app.add_plugins(ExtractComponentPlugin::<EguiRenderOutput>::default());
            app.add_plugins(ExtractComponentPlugin::<EguiRenderToImage>::default());
        }

        #[cfg(target_arch = "wasm32")]
        app.init_non_send_resource::<SubscribedEvents>();

        #[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
        app.init_resource::<EguiClipboard>();

        app.configure_sets(
            PreUpdate,
            (
                EguiPreUpdateSet::InitContexts,
                EguiPreUpdateSet::ProcessInput.after(InputSystem),
                EguiPreUpdateSet::BeginPass,
            )
                .chain(),
        );
        app.configure_sets(
            PreUpdate,
            (
                EguiInputSet::InitReading,
                EguiInputSet::FocusContext,
                EguiInputSet::ReadBevyEvents,
                EguiInputSet::WriteEguiEvents,
            )
                .chain(),
        );
        app.configure_sets(
            PostUpdate,
            (
                EguiPostUpdateSet::EndPass,
                EguiPostUpdateSet::ProcessOutput,
                EguiPostUpdateSet::PostProcessOutput.before(AccessibilitySystem::Update),
            )
                .chain(),
        );

        // Startup systems.
        #[cfg(all(feature = "manage_clipboard", target_arch = "wasm32"))]
        {
            app.add_systems(PreStartup, web_clipboard::startup_setup_web_events_system);
        }
        app.add_systems(
            PreStartup,
            (
                setup_new_windows_system,
                apply_deferred,
                update_ui_size_and_scale_system,
            )
                .chain()
                .in_set(EguiStartupSet::InitContexts),
        );

        // PreUpdate systems.
        app.add_systems(
            PreUpdate,
            (
                setup_new_windows_system,
                apply_deferred,
                update_ui_size_and_scale_system,
            )
                .chain()
                .in_set(EguiPreUpdateSet::InitContexts),
        );
        app.add_systems(
            PreUpdate,
            (
                (
                    write_modifiers_keys_state_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_modifiers_keys_state_system
                    })),
                    write_window_pointer_moved_events_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_window_pointer_moved_events_system
                    })),
                )
                    .in_set(EguiInputSet::InitReading),
                (
                    write_pointer_button_events_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_pointer_button_events_system
                    })),
                    write_window_touch_events_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_window_touch_events_system
                    })),
                )
                    .in_set(EguiInputSet::FocusContext),
                (
                    write_non_window_pointer_moved_events_system.run_if(input_system_is_enabled(
                        |s| s.run_write_non_window_pointer_moved_events_system,
                    )),
                    write_non_window_touch_events_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_non_window_touch_events_system
                    })),
                    write_mouse_wheel_events_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_mouse_wheel_events_system
                    })),
                    write_keyboard_input_events_system.run_if(input_system_is_enabled(|s| {
                        s.run_write_keyboard_input_events_system
                    })),
                    write_ime_events_system
                        .run_if(input_system_is_enabled(|s| s.run_write_ime_events_system)),
                )
                    .in_set(EguiInputSet::ReadBevyEvents),
                (
                    write_egui_input_system,
                    absorb_bevy_input_system.run_if(|settings: Res<EguiGlobalSettings>| {
                        settings.enable_absorb_bevy_input_system
                    }),
                )
                    .in_set(EguiInputSet::WriteEguiEvents),
            )
                .chain()
                .in_set(EguiPreUpdateSet::ProcessInput),
        );
        app.add_systems(
            PreUpdate,
            begin_pass_system.in_set(EguiPreUpdateSet::BeginPass),
        );

        // Web-specific resources and systems.
        #[cfg(target_arch = "wasm32")]
        {
            use std::sync::{LazyLock, Mutex};

            let maybe_window_plugin = app.get_added_plugins::<bevy_window::WindowPlugin>();

            if !maybe_window_plugin.is_empty()
                && maybe_window_plugin[0].primary_window.is_some()
                && maybe_window_plugin[0]
                    .primary_window
                    .as_ref()
                    .unwrap()
                    .prevent_default_event_handling
            {
                app.init_resource::<TextAgentChannel>();

                let (sender, receiver) = crossbeam_channel::unbounded();
                static TOUCH_INFO: LazyLock<Mutex<VirtualTouchInfo>> =
                    LazyLock::new(|| Mutex::new(VirtualTouchInfo::default()));

                app.insert_resource(SafariVirtualKeyboardTouchState {
                    sender,
                    receiver,
                    touch_info: &TOUCH_INFO,
                });

                app.add_systems(
                    PreStartup,
                    install_text_agent_system.in_set(EguiStartupSet::InitContexts),
                );

                app.add_systems(
                    PreUpdate,
                    write_text_agent_channel_events_system
                        .run_if(input_system_is_enabled(|s| {
                            s.run_write_text_agent_channel_events_system
                        }))
                        .in_set(EguiPreUpdateSet::ProcessInput)
                        .in_set(EguiInputSet::ReadBevyEvents),
                );

                if is_mobile_safari() {
                    app.add_systems(
                        PostUpdate,
                        process_safari_virtual_keyboard_system
                            .in_set(EguiPostUpdateSet::PostProcessOutput),
                    );
                }
            }

            #[cfg(feature = "manage_clipboard")]
            app.add_systems(
                PreUpdate,
                web_clipboard::write_web_clipboard_events_system
                    .run_if(input_system_is_enabled(|s| {
                        s.run_write_web_clipboard_events_system
                    }))
                    .in_set(EguiPreUpdateSet::ProcessInput)
                    .in_set(EguiInputSet::ReadBevyEvents),
            );
        }

        // PostUpdate systems.
        app.add_systems(
            PostUpdate,
            (run_egui_context_pass_loop_system, end_pass_system)
                .chain()
                .in_set(EguiPostUpdateSet::EndPass),
        );
        app.add_systems(
            PostUpdate,
            (process_output_system, write_egui_wants_input_system)
                .in_set(EguiPostUpdateSet::ProcessOutput),
        );
        #[cfg(feature = "picking")]
        app.add_systems(PostUpdate, capture_pointer_input_system);

        #[cfg(feature = "render")]
        app.add_systems(
            PostUpdate,
            update_egui_textures_system.in_set(EguiPostUpdateSet::PostProcessOutput),
        )
        .add_systems(
            Render,
            render_systems::prepare_egui_transforms_system.in_set(RenderSet::Prepare),
        )
        .add_systems(
            Render,
            render_systems::queue_bind_groups_system.in_set(RenderSet::Queue),
        )
        .add_systems(
            Render,
            render_systems::queue_pipelines_system.in_set(RenderSet::Queue),
        )
        .add_systems(Last, free_egui_textures_system);

        #[cfg(feature = "render")]
        load_internal_asset!(
            app,
            EGUI_SHADER_HANDLE,
            "egui.wgsl",
            bevy_render::render_resource::Shader::from_wgsl
        );

        app.add_systems(
            PostUpdate,
            update_accessibility_system.in_set(EguiPostUpdateSet::PostProcessOutput),
        );
    }

    #[cfg(feature = "render")]
    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<egui_node::EguiPipeline>()
                .init_resource::<SpecializedRenderPipelines<EguiPipeline>>()
                .init_resource::<EguiTransforms>()
                .init_resource::<EguiRenderData>()
                .add_systems(
                    // Seems to be just the set to add/remove nodes, as it'll run before
                    // `RenderSet::ExtractCommands` where render nodes get updated.
                    ExtractSchedule,
                    (
                        render_systems::setup_new_egui_nodes_system,
                        render_systems::teardown_window_nodes_system,
                        render_systems::teardown_render_to_image_nodes_system,
                    ),
                )
                .add_systems(
                    Render,
                    render_systems::prepare_egui_transforms_system.in_set(RenderSet::Prepare),
                )
                .add_systems(
                    Render,
                    render_systems::prepare_egui_render_target_data.in_set(RenderSet::Prepare),
                )
                .add_systems(
                    Render,
                    render_systems::queue_bind_groups_system.in_set(RenderSet::Queue),
                )
                .add_systems(
                    Render,
                    render_systems::queue_pipelines_system.in_set(RenderSet::Queue),
                );
        }
    }
}

fn input_system_is_enabled(
    test: impl Fn(&EguiInputSystemSettings) -> bool,
) -> impl Fn(Res<EguiGlobalSettings>) -> bool {
    move |settings| test(&settings.input_system_settings)
}

/// Contains textures allocated and painted by Egui.
#[cfg(feature = "render")]
#[derive(bevy_ecs::system::Resource, Deref, DerefMut, Default)]
pub struct EguiManagedTextures(pub bevy_utils::HashMap<(Entity, u64), EguiManagedTexture>);

/// Represents a texture allocated and painted by Egui.
#[cfg(feature = "render")]
pub struct EguiManagedTexture {
    /// Assets store handle.
    pub handle: Handle<Image>,
    /// Stored in full so we can do partial updates (which bevy doesn't support).
    pub color_image: egui::ColorImage,
}

/// Adds bevy_egui components to newly created windows.
pub fn setup_new_windows_system(
    mut commands: Commands,
    new_windows: Query<(Entity, Option<&PrimaryWindow>), (Added<Window>, Without<EguiContext>)>,
    adapters: Option<NonSend<AccessKitAdapters>>,
    mut manage_accessibility_updates: ResMut<ManageAccessibilityUpdates>,
    enable_multipass_for_primary_context: Option<Res<EnableMultipassForPrimaryContext>>,
) {
    for (window, primary) in new_windows.iter() {
        let context = EguiContext::default();
        if let Some(adapters) = &adapters {
            if adapters.get(&window).is_some() {
                context.ctx.enable_accesskit();
                **manage_accessibility_updates = false;
            }
        }
        // See the list of required components to check the full list of components we add.
        let mut window_commands = commands.entity(window);
        window_commands.insert(context);
        if enable_multipass_for_primary_context.is_some() && primary.is_some() {
            window_commands.insert(EguiMultipassSchedule::new(EguiContextPass));
        }
    }
}

#[cfg(all(feature = "manage_clipboard", not(target_os = "android")))]
impl EguiClipboard {
    /// Places the text onto the clipboard.
    pub fn set_text(&mut self, contents: &str) {
        self.set_text_impl(contents);
    }

    /// Sets the internal buffer of clipboard contents.
    /// This buffer is used to remember the contents of the last "Paste" event.
    #[cfg(target_arch = "wasm32")]
    pub fn set_text_internal(&mut self, text: &str) {
        self.clipboard.set_text_internal(text);
    }

    /// Gets clipboard text content. Returns [`None`] if clipboard provider is unavailable or returns an error.
    #[must_use]
    pub fn get_text(&mut self) -> Option<String> {
        self.get_text_impl()
    }

    /// Places an image to the clipboard.
    pub fn set_image(&mut self, image: &egui::ColorImage) {
        self.set_image_impl(image);
    }

    /// Receives a clipboard event sent by the `copy`/`cut`/`paste` listeners.
    #[cfg(target_arch = "wasm32")]
    pub fn try_receive_clipboard_event(&self) -> Option<web_clipboard::WebClipboardEvent> {
        self.clipboard.try_receive_clipboard_event()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_text_impl(&mut self, contents: &str) {
        if let Some(mut clipboard) = self.get() {
            if let Err(err) = clipboard.set_text(contents.to_owned()) {
                log::error!("Failed to set clipboard contents: {:?}", err);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn set_text_impl(&mut self, contents: &str) {
        self.clipboard.set_text(contents);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get_text_impl(&mut self) -> Option<String> {
        if let Some(mut clipboard) = self.get() {
            match clipboard.get_text() {
                Ok(contents) => return Some(contents),
                // We don't want to spam with this error as it usually means that the clipboard is either empty or has an incompatible format (e.g. image).
                Err(arboard::Error::ContentNotAvailable) => return Some("".to_string()),
                Err(err) => log::error!("Failed to get clipboard contents: {:?}", err),
            }
        };
        None
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::unnecessary_wraps)]
    fn get_text_impl(&mut self) -> Option<String> {
        self.clipboard.get_text()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_image_impl(&mut self, image: &egui::ColorImage) {
        if let Some(mut clipboard) = self.get() {
            if let Err(err) = clipboard.set_image(arboard::ImageData {
                width: image.width(),
                height: image.height(),
                bytes: std::borrow::Cow::Borrowed(bytemuck::cast_slice(&image.pixels)),
            }) {
                log::error!("Failed to set clipboard contents: {:?}", err);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn set_image_impl(&mut self, image: &egui::ColorImage) {
        self.clipboard.set_image(image);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get(&self) -> Option<RefMut<Clipboard>> {
        self.clipboard
            .get_or(|| {
                Clipboard::new()
                    .map(RefCell::new)
                    .map_err(|err| {
                        log::error!("Failed to initialize clipboard: {:?}", err);
                    })
                    .ok()
            })
            .as_ref()
            .map(|cell| cell.borrow_mut())
    }
}

/// The ordering value used for [`bevy_picking`].
#[cfg(feature = "picking")]
pub const PICKING_ORDER: f32 = 1_000_000.0;

/// Captures pointers on egui windows for [`bevy_picking`].
#[cfg(feature = "picking")]
pub fn capture_pointer_input_system(
    pointers: Query<(&PointerId, &PointerLocation)>,
    mut egui_context: Query<(Entity, &mut EguiContext, &EguiContextSettings), With<Window>>,
    mut output: EventWriter<PointerHits>,
) {
    use helpers::QueryHelper;

    for (pointer, location) in pointers
        .iter()
        .filter_map(|(i, p)| p.location.as_ref().map(|l| (i, l)))
    {
        if let NormalizedRenderTarget::Window(id) = location.target {
            if let Some((entity, mut ctx, settings)) = egui_context.get_some_mut(id.entity()) {
                if settings.capture_pointer_input && ctx.get_mut().wants_pointer_input() {
                    let entry = (entity, HitData::new(entity, 0.0, None, None));
                    output.send(PointerHits::new(
                        *pointer,
                        Vec::from([entry]),
                        PICKING_ORDER,
                    ));
                }
            }
        }
    }
}

/// Updates textures painted by Egui.
#[cfg(feature = "render")]
pub fn update_egui_textures_system(
    mut egui_render_output: Query<
        (Entity, &EguiRenderOutput),
        Or<(With<Window>, With<EguiRenderToImage>)>,
    >,
    mut egui_managed_textures: ResMut<EguiManagedTextures>,
    mut image_assets: ResMut<Assets<Image>>,
) {
    for (entity, egui_render_output) in egui_render_output.iter_mut() {
        for (texture_id, image_delta) in &egui_render_output.textures_delta.set {
            let color_image = egui_node::as_color_image(&image_delta.image);

            let texture_id = match texture_id {
                egui::TextureId::Managed(texture_id) => *texture_id,
                egui::TextureId::User(_) => continue,
            };

            let sampler = ImageSampler::Descriptor(
                egui_node::texture_options_as_sampler_descriptor(&image_delta.options),
            );
            if let Some(pos) = image_delta.pos {
                // Partial update.
                if let Some(managed_texture) = egui_managed_textures.get_mut(&(entity, texture_id))
                {
                    // TODO: when bevy supports it, only update the part of the texture that changes.
                    update_image_rect(&mut managed_texture.color_image, pos, &color_image);
                    let image =
                        egui_node::color_image_as_bevy_image(&managed_texture.color_image, sampler);
                    managed_texture.handle = image_assets.add(image);
                } else {
                    log::warn!("Partial update of a missing texture (id: {:?})", texture_id);
                }
            } else {
                // Full update.
                let image = egui_node::color_image_as_bevy_image(&color_image, sampler);
                let handle = image_assets.add(image);
                egui_managed_textures.insert(
                    (entity, texture_id),
                    EguiManagedTexture {
                        handle,
                        color_image,
                    },
                );
            }
        }
    }

    fn update_image_rect(dest: &mut egui::ColorImage, [x, y]: [usize; 2], src: &egui::ColorImage) {
        for sy in 0..src.height() {
            for sx in 0..src.width() {
                dest[(x + sx, y + sy)] = src[(sx, sy)];
            }
        }
    }
}

/// This system is responsible for deleting image assets of freed Egui-managed textures and deleting Egui user textures of removed Bevy image assets.
///
/// If you add textures via [`EguiContexts::add_image`] or [`EguiUserTextures::add_image`] by passing a weak handle,
/// the systems ensures that corresponding Egui textures are cleaned up as well.
#[cfg(feature = "render")]
pub fn free_egui_textures_system(
    mut egui_user_textures: ResMut<EguiUserTextures>,
    egui_render_output: Query<
        (Entity, &EguiRenderOutput),
        Or<(With<Window>, With<EguiRenderToImage>)>,
    >,
    mut egui_managed_textures: ResMut<EguiManagedTextures>,
    mut image_assets: ResMut<Assets<Image>>,
    mut image_events: EventReader<AssetEvent<Image>>,
) {
    for (entity, egui_render_output) in egui_render_output.iter() {
        for &texture_id in &egui_render_output.textures_delta.free {
            if let egui::TextureId::Managed(texture_id) = texture_id {
                let managed_texture = egui_managed_textures.remove(&(entity, texture_id));
                if let Some(managed_texture) = managed_texture {
                    image_assets.remove(&managed_texture.handle);
                }
            }
        }
    }

    for image_event in image_events.read() {
        if let AssetEvent::Removed { id } = image_event {
            egui_user_textures.remove_image(&Handle::<Image>::Weak(*id));
        }
    }
}

/// Helper function for outputting a String from a JsValue
#[cfg(target_arch = "wasm32")]
pub fn string_from_js_value(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:#?}"))
}

#[cfg(target_arch = "wasm32")]
struct EventClosure<T> {
    target: web_sys::EventTarget,
    event_name: String,
    closure: wasm_bindgen::closure::Closure<dyn FnMut(T)>,
}

/// Stores event listeners.
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub struct SubscribedEvents {
    #[cfg(feature = "manage_clipboard")]
    clipboard_event_closures: Vec<EventClosure<web_sys::ClipboardEvent>>,
    composition_event_closures: Vec<EventClosure<web_sys::CompositionEvent>>,
    keyboard_event_closures: Vec<EventClosure<web_sys::KeyboardEvent>>,
    input_event_closures: Vec<EventClosure<web_sys::InputEvent>>,
    touch_event_closures: Vec<EventClosure<web_sys::TouchEvent>>,
}

#[cfg(target_arch = "wasm32")]
impl SubscribedEvents {
    /// Use this method to unsubscribe from all stored events, this can be useful
    /// for gracefully destroying a Bevy instance in a page.
    pub fn unsubscribe_from_all_events(&mut self) {
        #[cfg(feature = "manage_clipboard")]
        Self::unsubscribe_from_events(&mut self.clipboard_event_closures);
        Self::unsubscribe_from_events(&mut self.composition_event_closures);
        Self::unsubscribe_from_events(&mut self.keyboard_event_closures);
        Self::unsubscribe_from_events(&mut self.input_event_closures);
        Self::unsubscribe_from_events(&mut self.touch_event_closures);
    }

    fn unsubscribe_from_events<T>(events: &mut Vec<EventClosure<T>>) {
        let events_to_unsubscribe = std::mem::take(events);

        if !events_to_unsubscribe.is_empty() {
            for event in events_to_unsubscribe {
                if let Err(err) = event.target.remove_event_listener_with_callback(
                    event.event_name.as_str(),
                    event.closure.as_ref().unchecked_ref(),
                ) {
                    log::error!(
                        "Failed to unsubscribe from event: {}",
                        string_from_js_value(&err)
                    );
                }
            }
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
#[allow(missing_docs)]
pub struct UpdateUiSizeAndScaleQuery {
    ctx: &'static mut EguiContext,
    egui_input: &'static mut EguiInput,
    render_target_size: &'static mut RenderTargetSize,
    egui_settings: &'static EguiContextSettings,
    window: Option<&'static Window>,
    #[cfg(feature = "render")]
    render_to_image: Option<&'static EguiRenderToImage>,
}

/// Updates UI [`egui::RawInput::screen_rect`] and calls [`egui::Context::set_pixels_per_point`].
pub fn update_ui_size_and_scale_system(
    mut contexts: Query<UpdateUiSizeAndScaleQuery>,
    #[cfg(feature = "render")] images: Res<Assets<Image>>,
) {
    for mut context in contexts.iter_mut() {
        let mut render_target_size = None;
        if let Some(window) = context.window {
            render_target_size = Some(RenderTargetSize::new(
                window.physical_width() as f32,
                window.physical_height() as f32,
                window.scale_factor(),
            ));
        }
        #[cfg(feature = "render")]
        if let Some(EguiRenderToImage { handle, .. }) = context.render_to_image {
            if let Some(image) = images.get(handle) {
                let size = image.size_f32();
                render_target_size = Some(RenderTargetSize {
                    physical_width: size.x,
                    physical_height: size.y,
                    scale_factor: 1.0,
                })
            } else {
                log::warn!("Invalid EguiRenderToImage handle: {handle:?}");
            }
        }

        let Some(new_render_target_size) = render_target_size else {
            log::error!("bevy_egui context without window or render to texture!");
            continue;
        };
        let width = new_render_target_size.physical_width
            / new_render_target_size.scale_factor
            / context.egui_settings.scale_factor;
        let height = new_render_target_size.physical_height
            / new_render_target_size.scale_factor
            / context.egui_settings.scale_factor;

        if width < 1.0 || height < 1.0 {
            continue;
        }

        context.egui_input.screen_rect = Some(egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(width, height),
        ));

        context.ctx.get_mut().set_pixels_per_point(
            new_render_target_size.scale_factor * context.egui_settings.scale_factor,
        );

        *context.render_target_size = new_render_target_size;
    }
}

/// Marks a pass start for Egui.
pub fn begin_pass_system(
    mut contexts: Query<
        (&mut EguiContext, &EguiContextSettings, &mut EguiInput),
        Without<EguiMultipassSchedule>,
    >,
) {
    for (mut ctx, egui_settings, mut egui_input) in contexts.iter_mut() {
        if !egui_settings.run_manually {
            ctx.get_mut().begin_pass(egui_input.take());
        }
    }
}

/// Marks a pass end for Egui.
pub fn end_pass_system(
    mut contexts: Query<
        (&mut EguiContext, &EguiContextSettings, &mut EguiFullOutput),
        Without<EguiMultipassSchedule>,
    >,
) {
    for (mut ctx, egui_settings, mut full_output) in contexts.iter_mut() {
        if !egui_settings.run_manually {
            **full_output = Some(ctx.get_mut().end_pass());
        }
    }
}

/// Updates the states of [`ManageAccessibilityUpdates`] and [`AccessKitAdapters`].
pub fn update_accessibility_system(
    requested: Res<AccessibilityRequested>,
    mut manage_accessibility_updates: ResMut<ManageAccessibilityUpdates>,
    outputs: Query<(Entity, &EguiOutput)>,
    mut adapters: NonSendMut<AccessKitAdapters>,
) {
    if requested.get() {
        for (entity, output) in &outputs {
            if let Some(adapter) = adapters.get_mut(&entity) {
                if let Some(update) = &output.platform_output.accesskit_update {
                    **manage_accessibility_updates = false;
                    adapter.update_if_active(|| update.clone());
                } else if !**manage_accessibility_updates {
                    **manage_accessibility_updates = true;
                }
            }
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
#[allow(missing_docs)]
pub struct MultiPassEguiQuery {
    entity: Entity,
    context: &'static mut EguiContext,
    input: &'static mut EguiInput,
    output: &'static mut EguiFullOutput,
    multipass_schedule: &'static EguiMultipassSchedule,
    settings: &'static EguiContextSettings,
}

/// Runs Egui contexts with the [`EguiMultipassSchedule`] component. If there are no contexts with
/// this component, runs the [`EguiContextPass`] schedule once independently.
pub fn run_egui_context_pass_loop_system(world: &mut World) {
    let mut contexts_query = world.query::<MultiPassEguiQuery>();
    let mut used_schedules = HashSet::new();

    let mut multipass_contexts: Vec<_> = contexts_query
        .iter_mut(world)
        .filter_map(|mut egui_context| {
            if egui_context.settings.run_manually {
                return None;
            }

            Some((
                egui_context.entity,
                egui_context.context.get_mut().clone(),
                egui_context.input.take(),
                egui_context.multipass_schedule.clone(),
            ))
        })
        .collect();

    for (entity, ctx, ref mut input, EguiMultipassSchedule(multipass_schedule)) in
        &mut multipass_contexts
    {
        if !used_schedules.insert(*multipass_schedule) {
            panic!("Each Egui context running in the multi-pass mode must have a unique schedule (attempted to reuse schedule {multipass_schedule:?})");
        }

        let output = ctx.run(input.take(), |_| {
            let _ = world.try_run_schedule(*multipass_schedule);
        });

        **contexts_query
            .get_mut(world, *entity)
            .expect("previously queried context")
            .output = Some(output);
    }

    if !used_schedules.contains(&ScheduleLabel::intern(&EguiContextPass)) {
        let _ = world.try_run_schedule(EguiContextPass);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }
}
