use bevy::{
    camera::{Viewport, visibility::RenderLayers},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext, egui,
};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.25, 0.25, 0.25)))
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup_system)
        .add_systems(EguiPrimaryContextPass, ui_example_system)
        .run();
}

// This function runs every frame. Therefore, updating the viewport after drawing the gui.
// With a resource which stores the dimensions of the panels, the update of the Viewport can
// be done in another system.
fn ui_example_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let mut left = egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Left resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width(); // height is ignored, as the panel has a hight of 100% of the screen

    let mut right = egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Right resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width(); // height is ignored, as the panel has a height of 100% of the screen

    let mut top = egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Top resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen
    let mut bottom = egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Bottom resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen

    // Scale from logical units to physical units.
    left *= window.scale_factor();
    right *= window.scale_factor();
    top *= window.scale_factor();
    bottom *= window.scale_factor();

    // -------------------------------------------------
    // |  left   |            top   ^^^^^^   |  right  |
    // |  panel  |           panel  height   |  panel  |
    // |         |                  vvvvvv   |         |
    // |         |---------------------------|         |
    // |         |                           |         |
    // |<-width->|          viewport         |<-width->|
    // |         |                           |         |
    // |         |---------------------------|         |
    // |         |          bottom   ^^^^^^  |         |
    // |         |          panel    height  |         |
    // |         |                   vvvvvv  |         |
    // -------------------------------------------------
    //
    // The upper left point of the viewport is the width of the left panel and the height of the
    // top panel
    //
    // The width of the viewport the width of the top/bottom panel
    // Alternative the width can be calculated as follow:
    // size.x = window width - left panel width - right panel width
    //
    // The height of the viewport is:
    // size.y = window height - top panel height - bottom panel height
    //
    // Therefore we use the alternative for the width, as we can callculate the Viewport as
    // following:

    let pos = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height())
        - pos
        - UVec2::new(right as u32, bottom as u32);

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });

    Ok(())
}

// Set up the example entities for the scene. The only important thing is a camera which
// renders directly to the window.
fn setup_system(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

    // Circle.
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(50.))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.1, 0.0)))),
        Transform::from_translation(Vec3::new(-150., 0., 0.)),
    ));

    // Rectangles.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50., 100.))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.5, 0.4, 0.3)))),
        Transform::from_translation(Vec3::new(-50., 0., 0.)),
    ));
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50., 100.))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.5, 0.4, 0.3)))),
        Transform::from_translation(Vec3::new(50., 0., 0.)),
    ));

    // Hexagon.
    commands.spawn((
        Mesh2d(meshes.add(RegularPolygon::new(50., 6))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.8, 0.7, 0.6)))),
        Transform::from_translation(Vec3::new(150., 0., 0.)),
    ));

    // World camera.
    commands.spawn(Camera2d);
    // Egui camera.
    commands.spawn((
        // The `PrimaryEguiContext` component requires everything needed to render a primary context.
        PrimaryEguiContext,
        Camera2d,
        // Setting RenderLayers to none makes sure we won't render anything apart from the UI.
        RenderLayers::none(),
        Camera {
            order: 1,
            ..default()
        },
    ));
}
