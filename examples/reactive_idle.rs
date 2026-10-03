//! Reproducer: with a reactive `WinitSettings`, an idle app drawing egui should
//! only tick on the reactive timeout (every 5 s), but it renders every frame.
//!
//! Run without touching the window:
//!
//!     cargo run --example reactive_idle
//!     REACTIVE_IDLE_WORKAROUND=1 cargo run --example reactive_idle
//!
//! The second run drops `ModifiersChanged` events whose modifiers did not change.
//! The app exits by itself after ~16 s; set `REACTIVE_IDLE_SECS=0` to keep it open.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    window::RequestRedraw,
    winit::{UpdateMode, WinitSettings},
};
use bevy_egui::{
    EguiContexts, EguiInput, EguiPlugin, EguiPreUpdateSet, EguiPrimaryContextPass, egui,
};
use std::time::{Duration, Instant};

const REPORT_INTERVAL: Duration = Duration::from_secs(2);

fn main() {
    let workaround = std::env::var_os("REACTIVE_IDLE_WORKAROUND").is_some();
    let run_for = std::env::var("REACTIVE_IDLE_SECS")
        .ok()
        .and_then(|secs| secs.parse().ok())
        .unwrap_or(16);
    println!("workaround (drop unchanged ModifiersChanged): {workaround}, run for: {run_for} s");

    let mut app = App::new();
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::reactive_low_power(Duration::from_secs(5)),
        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(5)),
    })
    .add_plugins(DefaultPlugins)
    .add_plugins(EguiPlugin::default())
    .init_resource::<Stats>()
    .insert_resource(RunFor(Duration::from_secs(run_for)))
    .add_systems(Startup, setup_camera_system)
    .add_systems(EguiPrimaryContextPass, ui_example_system)
    .add_systems(Last, report_system);
    if workaround {
        app.add_systems(
            PreUpdate,
            drop_unchanged_modifiers
                .after(EguiPreUpdateSet::ProcessInput)
                .before(EguiPreUpdateSet::BeginPass),
        );
    }
    app.run();
}

#[derive(Resource)]
struct RunFor(Duration);

#[derive(Resource)]
struct Stats {
    started: Instant,
    window_start: Instant,
    frames: u32,
    redraws: u32,
    causes: HashSet<String>,
}

impl Default for Stats {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            window_start: now,
            frames: 0,
            redraws: 0,
            causes: HashSet::default(),
        }
    }
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui_example_system(
    mut contexts: EguiContexts,
    mut stats: ResMut<Stats>,
    mut text: Local<String>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    egui::Window::new("Hello").show(ctx, |ui| {
        ui.label("Don't touch this window.");
        ui.label(format!("egui modifiers: {:?}", ui.input(|i| i.modifiers)));
        ui.text_edit_singleline(&mut *text);
        egui::ScrollArea::both().max_height(100.0).show(ui, |ui| {
            for row in 0..30 {
                ui.add(
                    egui::Label::new(format!("row {row}: {}", "wide text ".repeat(10))).extend(),
                );
            }
        });
    });
    for cause in ctx.repaint_causes() {
        stats.causes.insert(cause.to_string());
    }
    Ok(())
}

fn report_system(
    mut stats: ResMut<Stats>,
    run_for: Res<RunFor>,
    mut redraws: MessageReader<RequestRedraw>,
    mut exit: MessageWriter<AppExit>,
) {
    stats.frames += 1;
    stats.redraws += redraws.read().count() as u32;

    let now = Instant::now();
    let elapsed = now - stats.window_start;
    if elapsed >= REPORT_INTERVAL {
        println!(
            "[{:>5.1}s] over {:.1}s: frames={} redraw_msgs={} repaint_causes={:?}",
            (now - stats.started).as_secs_f32(),
            elapsed.as_secs_f32(),
            stats.frames,
            stats.redraws,
            stats.causes,
        );
        stats.window_start = now;
        stats.frames = 0;
        stats.redraws = 0;
        stats.causes.clear();
    }
    if !run_for.0.is_zero() && now - stats.started >= run_for.0 {
        exit.write(AppExit::Success);
    }
}

fn drop_unchanged_modifiers(
    mut inputs: Query<(Entity, &mut EguiInput)>,
    mut last: Local<HashMap<Entity, egui::Modifiers>>,
) {
    for (entity, mut input) in &mut inputs {
        input.0.events.retain(|event| match event {
            egui::Event::ModifiersChanged(now) => last.insert(entity, *now) != Some(*now),
            _ => true,
        });
    }
}
