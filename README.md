[![Stand With Ukraine](https://raw.githubusercontent.com/vshymanskyy/StandWithUkraine/main/banner2-direct.svg)](https://stand-with-ukraine.pp.ua)

**Hey!** I'm the author of the crate, and I was born in Mariupol, Ukraine. When russians started the war in 2014, I moved to Kyiv. My parents, who had been staying in Mariupol till the start of the full-scale invasion, barely escaped the city alive. By the moment of writing (November 5th, 2023), we had [874 air raid alerts in Kyiv, and russians managed to bomb the city 132 times](https://air-alarms.in.ua/en/region/kyiv).

**If you are using this crate, please consider donating to any of the listed funds (see the banner above), that will mean a lot to me.**

# `bevy_egui`

[![Crates.io](https://img.shields.io/crates/v/bevy_egui.svg)](https://crates.io/crates/bevy_egui)
[![Documentation](https://docs.rs/bevy_egui/badge.svg)](https://docs.rs/bevy_egui)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/master/LICENSE)
[![Downloads](https://img.shields.io/crates/d/bevy_egui.svg)](https://crates.io/crates/bevy_egui)
[![CI](https://github.com/vladbat00/bevy_egui/actions/workflows/check.yml/badge.svg?branch=main)](https://github.com/vladbat00/bevy_egui/actions)

This crate provides an [Egui](https://github.com/emilk/egui) integration for the [Bevy](https://github.com/bevyengine/bevy) game engine.

**Trying out:**

A basic WASM example is live at [vladbat00.github.io/bevy_egui/ui](https://vladbat00.github.io/bevy_egui/ui/).

**Features:**
- Desktop and web platforms support
- Clipboard
- Opening URLs
- Multiple windows support (see [./examples/two_windows.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/two_windows.rs))
- Paint callback support (see [./examples/paint_callback.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/paint_callback.rs))
- Mobile web virtual keyboard (still rough around the edges and only works without `prevent_default_event_handling` set to `false` in the `WindowPlugin` settings)

![bevy_egui](bevy_egui.png)

## Dependencies

On Linux, this crate requires certain parts of [XCB](https://xcb.freedesktop.org/) to be installed on your system. On Debian-based systems, these can be installed with the following command:

```bash
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## Usage

Here's a minimal usage example:
```toml
# Cargo.toml
[dependencies]
bevy = "0.15"
bevy_egui = "0.33"
```

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        .add_systems(EguiContextPass, ui_example_system)
        .run();
}

fn ui_example_system(mut contexts: EguiContexts) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");
    });
}

```

Note that this example uses Egui in the [multi-pass mode]((https://docs.rs/egui/0.31.1/egui/#multi-pass-immediate-mode)).
If you don't want to be limited to the `EguiContextPass` schedule, you can use the single-pass mode,
but it may get deprecated in the future.

For more advanced examples, see the [examples](#Examples) section below.

### Note to developers of public plugins

If your plugin depends on `bevy_egui`, here are some hints on how to implement the support of both single-pass and multi-pass modes
(with respect to the `EguiPlugin::enable_multipass_for_primary_context` flag):
- Don't initialize `EguiPlugin` for the user, i.e. DO NOT use `add_plugins(EguiPlugin { ... })` in your code,
  users should be able to opt in or opt out of the multi-pass mode on their own.
- If you add UI systems, make sure they go into the `EguiContextPass` schedule - this will guarantee your plugin supports both the single-pass and multi-pass modes.

Your plugin code might look like this:

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiContextPass};

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        // Don't add the plugin for users, let them chose the default mode themselves
        // and just make sure they initialize EguiPlugin before yours.
        assert!(app.is_plugin_added::<EguiPlugin>());

        app.add_systems(EguiContextPass, ui_system);
    }
}

fn ui_system(contexts: EguiContexts) {
    // ...
}
```

## Examples

To run an example, use the following command (you may replace `ui` with a name of another example):

```bash
cargo run --example ui
```

### ui ([live page](https://vladbat00.github.io/bevy_egui/ui), source: [examples/ui.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/ui.rs))

Showcasing some more advanced UI, rendering images, hidpi scaling.

### color_test ([live page](https://vladbat00.github.io/bevy_egui/color_test), source: [examples/color_test.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/color_test.rs))

Rendering test from [egui.rs](https://egui.rs). We don't fully pass it, help is wanted ([#291](https://github.com/vladbat00/bevy_egui/issues/291)).

### side_panel_2d ([live page](https://vladbat00.github.io/bevy_egui/side_panel_2d), source: [examples/side_panel_2d.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/side_panel_2d.rs))

Showing how to display an Egui side panel and transform a camera with a perspective projection to make rendering centered relative to the remaining screen area.

### side_panel_3d ([live page](https://vladbat00.github.io/bevy_egui/side_panel_3d), source: [examples/side_panel_3d.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/side_panel_3d.rs))

Showing how to display an Egui side panel and transform a camera with a orthographic projection to make rendering centered relative to the remaining screen area.

### render_egui_to_image ([live page](https://vladbat00.github.io/bevy_egui/render_egui_to_image), source: [examples/render_egui_to_image.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/render_egui_to_image.rs))

Rendering UI to an image (texture) and then using it as a mesh material texture.

### render_to_image_widget ([live page](https://vladbat00.github.io/bevy_egui/render_to_image_widget), source: [examples/render_to_image_widget.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/render_to_image_widget.rs))

Rendering to a texture with Bevy and showing it as an Egui image widget.

### two_windows (source: [examples/two_windows.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/two_windows.rs))

Setting up two windows with an Egui context for each.

### paint_callback ([live page](https://vladbat00.github.io/bevy_egui/paint_callback), source: [examples/paint_callback.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/paint_callback.rs))

Using Egui paint callbacks.

### simple ([live page](https://vladbat00.github.io/bevy_egui/simple), source: [examples/simple.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/simple.rs))

The minimal usage example from this readme.

### run_manually ([live page](https://vladbat00.github.io/bevy_egui/run_manually), source: [examples/run_manually.rs](https://github.com/vladbat00/bevy_egui/blob/v0.33.0/examples/run_manually.rs))

The same minimal example demonstrating running Egui passes manually.

## See also

- [`jakobhellermann/bevy-inspector-egui`](https://github.com/jakobhellermann/bevy-inspector-egui)

## Bevy support table

**Note:** if you're looking for a `bevy_egui` version that supports `main` branch of Bevy, check out [open PRs](https://github.com/vladbat00/bevy_egui/pulls), there's a great chance we've already started working on the future Bevy release support.

| bevy | bevy_egui |
|------|-----------|
| 0.15 | 0.31-0.33 |
| 0.14 | 0.28-0.30 |
| 0.13 | 0.25-0.27 |
| 0.12 | 0.23-0.24 |
| 0.11 | 0.21-0.22 |
| 0.10 | 0.20      |
| 0.9  | 0.17-0.19 |
| 0.8  | 0.15-0.16 |
| 0.7  | 0.13-0.14 |
| 0.6  | 0.10-0.12 |
| 0.5  | 0.4-0.9   |
| 0.4  | 0.1-0.3   |
