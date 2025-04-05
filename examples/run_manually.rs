use bevy::prelude::*;
use bevy_egui::{
    EguiContext, EguiContextSettings, EguiFullOutput, EguiInput, EguiPlugin, EguiStartupSet,
};
use std::num::NonZero;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        .add_systems(
            PreStartup,
            configure_context.after(EguiStartupSet::InitContexts),
        )
        .add_systems(Update, ui_example_system)
        .run();
}

fn configure_context(mut egui_settings: Query<&mut EguiContextSettings>) -> Result {
    egui_settings.single_mut()?.run_manually = true;
    Ok(())
}

fn ui_example_system(
    mut contexts: Query<(&mut EguiContext, &mut EguiInput, &mut EguiFullOutput)>,
) -> Result {
    let (mut ctx, mut egui_input, mut egui_full_output) = contexts.single_mut()?;

    let ui = |ctx: &egui::Context| {
        egui::Window::new("Hello").show(ctx, |ui| {
            let passes = ui
                .ctx()
                .viewport(|viewport| viewport.output.num_completed_passes)
                + 1;
            ui.label(format!("Passes: {}", passes));
            ui.ctx().request_discard("Trying to reach max limit");
        });
    };

    let ctx = ctx.get_mut();
    ctx.memory_mut(|memory| {
        memory.options.max_passes = NonZero::new(5).unwrap();
    });

    **egui_full_output = Some(ctx.run(egui_input.take(), ui));

    Ok(())
}
