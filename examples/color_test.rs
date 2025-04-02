use bevy::{
    prelude::{MeshMaterial2d, *},
    render::render_resource::LoadOp,
    window::PrimaryWindow,
};
use bevy_egui::{
    helpers::vec2_into_egui_pos2,
    input::{EguiContextPointerPosition, HoveredNonWindowEguiContext},
    BevyEguiApp, EguiContext, EguiContextSettings, EguiContexts, EguiInputSet, EguiPlugin,
    EguiRenderToImage, OnEguiPass,
};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            default_to_multipass: true,
        })
        .init_resource::<AppState>()
        .add_systems(Startup, setup_system)
        .add_systems(
            PreUpdate,
            update_egui_hovered_context.in_set(EguiInputSet::InitReading),
        )
        .add_systems(Update, update_image_size_system)
        .add_egui_system(ui_system)
        .run();
}

#[derive(Eq, PartialEq)]
enum DisplayedUi {
    Regular,
    MeshImage,
    EguiTextureImage,
}

#[derive(Resource)]
struct AppState {
    displayed_ui: DisplayedUi,
    color_test: ColorTest,
    top_panel_height: u32,
    mesh_image_entity: Entity,
    egui_texture_image_entity: Entity,
    egui_texture_image_handle: Handle<bevy::image::Image>,
    egui_texture_image_id: egui::TextureId,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            displayed_ui: DisplayedUi::Regular,
            color_test: Default::default(),
            top_panel_height: 0,
            mesh_image_entity: Entity::PLACEHOLDER,
            egui_texture_image_entity: Entity::PLACEHOLDER,
            egui_texture_image_handle: Handle::default(),
            egui_texture_image_id: egui::TextureId::User(0),
        }
    }
}

fn setup_system(
    mut commands: Commands,
    mut egui_contexts: EguiContexts,
    mut app_state: ResMut<AppState>,
    mut meshes: ResMut<Assets<bevy::prelude::Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<bevy::image::Image>>,
) {
    let size = Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };
    let mut image = bevy::image::Image {
        // You should use `0` so that the pixels are transparent.
        data: vec![0; (size.width * size.height * 4) as usize],
        ..default()
    };
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    image.texture_descriptor.size = size;

    let mesh_image_handle = images.add(image.clone());
    let egui_texture_image_handle = images.add(image);

    app_state.mesh_image_entity = commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::new(256.0, 256.0))),
            MeshMaterial2d(materials.add(mesh_image_handle.clone())),
            EguiRenderToImage {
                handle: mesh_image_handle,
                load_op: LoadOp::Clear(Color::srgb_u8(43, 44, 47).to_linear().into()),
            },
        ))
        .observe(mesh_ui_system)
        .id();

    app_state.egui_texture_image_entity = commands
        .spawn(EguiRenderToImage {
            handle: egui_texture_image_handle.clone(),
            load_op: LoadOp::Clear(Color::srgb_u8(43, 44, 47).to_linear().into()),
        })
        .id();
    app_state.egui_texture_image_handle = egui_texture_image_handle.clone_weak();
    app_state.egui_texture_image_id =
        egui_contexts.add_image(egui_texture_image_handle.clone_weak());

    commands.spawn(Camera2d);
}

fn update_image_size_system(
    mut prev_top_panel_height: Local<u32>,
    mut prev_window_size: Local<UVec2>,
    window: Single<&Window, With<PrimaryWindow>>,
    app_state: Res<AppState>,
    mut images: ResMut<Assets<bevy::image::Image>>,
    mut meshes: ResMut<Assets<bevy::prelude::Mesh>>,
    mut egui_render_to_image_query: Query<(
        &EguiRenderToImage,
        Option<&Mesh2d>,
        Option<&mut Transform>,
    )>,
) {
    if *prev_window_size == window.physical_size()
        && *prev_top_panel_height == app_state.top_panel_height
    {
        return;
    }

    *prev_window_size = window.physical_size();
    *prev_top_panel_height = app_state.top_panel_height;

    let new_height = window.physical_height() - app_state.top_panel_height;

    for (egui_render_to_image, mesh_handle, transform) in egui_render_to_image_query.iter_mut() {
        let image = images
            .get_mut(&egui_render_to_image.handle)
            .expect("Expected a created image");
        image
            .data
            .resize((window.physical_width() * new_height * 4) as usize, 0);
        image.texture_descriptor.size.width = window.physical_width();
        image.texture_descriptor.size.height = new_height;

        if let Some((mesh_handle, mut transform)) = mesh_handle.zip(transform) {
            *meshes
                .get_mut(mesh_handle)
                .expect("Expected a created mesh") =
                Rectangle::new(window.physical_width() as f32, new_height as f32).into();
            transform.translation.y = *prev_top_panel_height as f32 / -2.0;
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_egui_hovered_context(
    mut commands: Commands,
    app_state: Res<AppState>,
    mut cursor_moved_reader: EventReader<CursorMoved>,
    mut egui_contexts: Query<
        (
            Entity,
            &mut EguiContextPointerPosition,
            &EguiContextSettings,
            Option<&Mesh2d>,
        ),
        (With<EguiContext>, Without<PrimaryWindow>),
    >,
) {
    for (entity, mut context_pointer_position, settings, mesh) in egui_contexts.iter_mut() {
        if !matches!(
            (&app_state.displayed_ui, mesh),
            (DisplayedUi::MeshImage, Some(_)) | (DisplayedUi::EguiTextureImage, None)
        ) {
            continue;
        }

        // We expect to reach this code only once since we can have only 1 active context matching the conditions.
        for event in cursor_moved_reader.read() {
            let scale_factor = settings.scale_factor;
            let pointer_position = vec2_into_egui_pos2(event.position / scale_factor)
                - Vec2::new(0.0, app_state.top_panel_height as f32);
            if pointer_position.y < 0.0 {
                commands.remove_resource::<HoveredNonWindowEguiContext>();
                continue;
            }

            context_pointer_position.position = pointer_position;
            commands.insert_resource(HoveredNonWindowEguiContext(entity));
        }
    }
}

fn ui_system(
    _trigger: Trigger<OnEguiPass>,
    mut app_state: ResMut<AppState>,
    mut contexts: EguiContexts,
    images: Res<Assets<bevy::image::Image>>,
) {
    let ctx = contexts.ctx_mut();
    app_state.top_panel_height = egui::TopBottomPanel::top("top_panel")
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut app_state.displayed_ui,
                    DisplayedUi::Regular,
                    "Regular UI",
                );
                ui.selectable_value(
                    &mut app_state.displayed_ui,
                    DisplayedUi::MeshImage,
                    "Render to image (mesh)",
                );
                ui.selectable_value(
                    &mut app_state.displayed_ui,
                    DisplayedUi::EguiTextureImage,
                    "Render to image (Egui user texture)",
                );
            });
        })
        .response
        .rect
        .height() as u32;

    match app_state.displayed_ui {
        DisplayedUi::Regular => {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    app_state.color_test.ui(ui);
                });
            });
        }
        DisplayedUi::MeshImage => {}
        DisplayedUi::EguiTextureImage => {
            let egui_texture_image = images
                .get(&app_state.egui_texture_image_handle)
                .expect("Expected a created image");
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    ui.image(egui::load::SizedTexture::new(
                        app_state.egui_texture_image_id,
                        [
                            egui_texture_image.texture_descriptor.size.width as f32,
                            egui_texture_image.texture_descriptor.size.height as f32,
                        ],
                    ));
                });

            let egui_texture_image_entity_ctx =
                contexts.ctx_for_entity_mut(app_state.egui_texture_image_entity);
            egui::CentralPanel::default().show(egui_texture_image_entity_ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    app_state.color_test.ui(ui);
                });
            });
        }
    }
}

fn mesh_ui_system(
    _trigger: Trigger<OnEguiPass>,
    mut app_state: ResMut<AppState>,
    mut contexts: EguiContexts,
) {
    let mesh_image_ctx = contexts.ctx_for_entity_mut(app_state.mesh_image_entity);
    egui::CentralPanel::default().show(mesh_image_ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            app_state.color_test.ui(ui);
        });
    });
}

//
// Copy-pasted from https://github.com/emilk/egui/blob/0.30.0/crates/egui_demo_lib/src/rendering_test.rs.
//

use egui::{
    epaint, lerp, pos2, vec2, widgets::color_picker::show_color, Align2, Color32, FontId, Image,
    Mesh, Pos2, Rect, Response, Rgba, RichText, Sense, Shape, Stroke, TextureHandle,
    TextureOptions, Ui, Vec2,
};
use std::collections::HashMap;
use wgpu_types::{Extent3d, TextureUsages};

const GRADIENT_SIZE: Vec2 = vec2(256.0, 18.0);

const BLACK: Color32 = Color32::BLACK;
const GREEN: Color32 = Color32::GREEN;
const RED: Color32 = Color32::RED;
const TRANSPARENT: Color32 = Color32::TRANSPARENT;
const WHITE: Color32 = Color32::WHITE;

/// A test for sanity-checking and diagnosing egui rendering backends.
pub struct ColorTest {
    tex_mngr: TextureManager,
    vertex_gradients: bool,
    texture_gradients: bool,
}

impl Default for ColorTest {
    fn default() -> Self {
        Self {
            tex_mngr: Default::default(),
            vertex_gradients: true,
            texture_gradients: true,
        }
    }
}

impl ColorTest {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("This is made to test that the egui rendering backend is set up correctly.");
            ui.add(egui::Label::new("❓").sense(egui::Sense::click()))
                .on_hover_text("The texture sampling should be sRGB-aware, and every other color operation should be done in gamma-space (sRGB). All colors should use pre-multiplied alpha");
        });

        ui.separator();

        pixel_test(ui);

        ui.separator();

        ui.collapsing("Color test", |ui| {
            self.color_test(ui);
        });

        ui.separator();

        ui.heading("Text rendering");

        text_on_bg(ui, Color32::from_gray(200), Color32::from_gray(230)); // gray on gray
        text_on_bg(ui, Color32::from_gray(140), Color32::from_gray(28)); // dark mode normal text

        // Matches Mac Font book (useful for testing):
        text_on_bg(ui, Color32::from_gray(39), Color32::from_gray(255));
        text_on_bg(ui, Color32::from_gray(220), Color32::from_gray(30));

        ui.separator();

        blending_and_feathering_test(ui);
    }

    fn color_test(&mut self, ui: &mut Ui) {
        ui.label("If the rendering is done right, all groups of gradients will look uniform.");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.vertex_gradients, "Vertex gradients");
            ui.checkbox(&mut self.texture_gradients, "Texture gradients");
        });

        ui.heading("sRGB color test");
        ui.label("Use a color picker to ensure this color is (255, 165, 0) / #ffa500");
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0; // No spacing between gradients
            let g = Gradient::one_color(Color32::from_rgb(255, 165, 0));
            self.vertex_gradient(ui, "orange rgb(255, 165, 0) - vertex", WHITE, &g);
            self.tex_gradient(ui, "orange rgb(255, 165, 0) - texture", WHITE, &g);
        });

        ui.separator();

        ui.label("Test that vertex color times texture color is done in gamma space:");
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0; // No spacing between gradients

            let tex_color = Color32::from_rgb(64, 128, 255);
            let vertex_color = Color32::from_rgb(128, 196, 196);
            let ground_truth = mul_color_gamma(tex_color, vertex_color);

            ui.horizontal(|ui| {
                let color_size = ui.spacing().interact_size;
                ui.label("texture");
                show_color(ui, tex_color, color_size);
                ui.label(" * ");
                show_color(ui, vertex_color, color_size);
                ui.label(" vertex color =");
            });
            {
                let g = Gradient::one_color(ground_truth);
                self.vertex_gradient(ui, "Ground truth (vertices)", WHITE, &g);
                self.tex_gradient(ui, "Ground truth (texture)", WHITE, &g);
            }

            ui.horizontal(|ui| {
                let g = Gradient::one_color(tex_color);
                let tex = self.tex_mngr.get(ui.ctx(), &g);
                let texel_offset = 0.5 / (g.0.len() as f32);
                let uv = Rect::from_min_max(pos2(texel_offset, 0.0), pos2(1.0 - texel_offset, 1.0));
                ui.add(
                    Image::from_texture((tex.id(), GRADIENT_SIZE))
                        .tint(vertex_color)
                        .uv(uv),
                )
                .on_hover_text(format!("A texture that is {} texels wide", g.0.len()));
                ui.label("GPU result");
            });
        });

        ui.separator();

        // TODO(emilk): test color multiplication (image tint),
        // to make sure vertex and texture color multiplication is done in linear space.

        ui.label("Gamma interpolation:");
        self.show_gradients(ui, WHITE, (RED, GREEN), Interpolation::Gamma);

        ui.separator();

        self.show_gradients(ui, RED, (TRANSPARENT, GREEN), Interpolation::Gamma);

        ui.separator();

        self.show_gradients(ui, WHITE, (TRANSPARENT, GREEN), Interpolation::Gamma);

        ui.separator();

        self.show_gradients(ui, BLACK, (BLACK, WHITE), Interpolation::Gamma);
        ui.separator();
        self.show_gradients(ui, WHITE, (BLACK, TRANSPARENT), Interpolation::Gamma);
        ui.separator();
        self.show_gradients(ui, BLACK, (TRANSPARENT, WHITE), Interpolation::Gamma);
        ui.separator();

        ui.label("Additive blending: add more and more blue to the red background:");
        self.show_gradients(
            ui,
            RED,
            (TRANSPARENT, Color32::from_rgb_additive(0, 0, 255)),
            Interpolation::Gamma,
        );

        ui.separator();

        ui.label("Linear interpolation (texture sampling):");
        self.show_gradients(ui, WHITE, (RED, GREEN), Interpolation::Linear);
    }

    fn show_gradients(
        &mut self,
        ui: &mut Ui,
        bg_fill: Color32,
        (left, right): (Color32, Color32),
        interpolation: Interpolation,
    ) {
        let is_opaque = left.is_opaque() && right.is_opaque();

        ui.horizontal(|ui| {
            let color_size = ui.spacing().interact_size;
            if !is_opaque {
                ui.label("Background:");
                show_color(ui, bg_fill, color_size);
            }
            ui.label("gradient");
            show_color(ui, left, color_size);
            ui.label("-");
            show_color(ui, right, color_size);
        });

        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0; // No spacing between gradients
            if is_opaque {
                let g = Gradient::ground_truth_gradient(left, right, interpolation);
                self.vertex_gradient(ui, "Ground Truth (CPU gradient) - vertices", bg_fill, &g);
                self.tex_gradient(ui, "Ground Truth (CPU gradient) - texture", bg_fill, &g);
            } else {
                let g = Gradient::ground_truth_gradient(left, right, interpolation)
                    .with_bg_fill(bg_fill);
                self.vertex_gradient(
                    ui,
                    "Ground Truth (CPU gradient, CPU blending) - vertices",
                    bg_fill,
                    &g,
                );
                self.tex_gradient(
                    ui,
                    "Ground Truth (CPU gradient, CPU blending) - texture",
                    bg_fill,
                    &g,
                );
                let g = Gradient::ground_truth_gradient(left, right, interpolation);
                self.vertex_gradient(ui, "CPU gradient, GPU blending - vertices", bg_fill, &g);
                self.tex_gradient(ui, "CPU gradient, GPU blending - texture", bg_fill, &g);
            }

            let g = Gradient::endpoints(left, right);

            match interpolation {
                Interpolation::Linear => {
                    // texture sampler is sRGBA aware, and should therefore be linear
                    self.tex_gradient(ui, "Texture of width 2 (test texture sampler)", bg_fill, &g);
                }
                Interpolation::Gamma => {
                    // vertex shader uses gamma
                    self.vertex_gradient(
                        ui,
                        "Triangle mesh of width 2 (test vertex decode and interpolation)",
                        bg_fill,
                        &g,
                    );
                }
            }
        });
    }

    fn tex_gradient(&mut self, ui: &mut Ui, label: &str, bg_fill: Color32, gradient: &Gradient) {
        if !self.texture_gradients {
            return;
        }
        ui.horizontal(|ui| {
            let tex = self.tex_mngr.get(ui.ctx(), gradient);
            let texel_offset = 0.5 / (gradient.0.len() as f32);
            let uv = Rect::from_min_max(pos2(texel_offset, 0.0), pos2(1.0 - texel_offset, 1.0));
            ui.add(
                Image::from_texture((tex.id(), GRADIENT_SIZE))
                    .bg_fill(bg_fill)
                    .uv(uv),
            )
            .on_hover_text(format!(
                "A texture that is {} texels wide",
                gradient.0.len()
            ));
            ui.label(label);
        });
    }

    fn vertex_gradient(&self, ui: &mut Ui, label: &str, bg_fill: Color32, gradient: &Gradient) {
        if !self.vertex_gradients {
            return;
        }
        ui.horizontal(|ui| {
            vertex_gradient(ui, bg_fill, gradient).on_hover_text(format!(
                "A triangle mesh that is {} vertices wide",
                gradient.0.len()
            ));
            ui.label(label);
        });
    }
}

fn vertex_gradient(ui: &mut Ui, bg_fill: Color32, gradient: &Gradient) -> Response {
    let (rect, response) = ui.allocate_at_least(GRADIENT_SIZE, Sense::hover());
    if bg_fill != Default::default() {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(rect, bg_fill);
        ui.painter().add(Shape::mesh(mesh));
    }
    {
        let n = gradient.0.len();
        assert!(n >= 2);
        let mut mesh = Mesh::default();
        for (i, &color) in gradient.0.iter().enumerate() {
            let t = i as f32 / (n as f32 - 1.0);
            let x = lerp(rect.x_range(), t);
            mesh.colored_vertex(pos2(x, rect.top()), color);
            mesh.colored_vertex(pos2(x, rect.bottom()), color);
            if i < n - 1 {
                let i = i as u32;
                mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
                mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
            }
        }
        ui.painter().add(Shape::mesh(mesh));
    }
    response
}

#[derive(Clone, Copy)]
enum Interpolation {
    Linear,
    Gamma,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct Gradient(pub Vec<Color32>);

impl Gradient {
    pub fn one_color(srgba: Color32) -> Self {
        Self(vec![srgba, srgba])
    }

    pub fn endpoints(left: Color32, right: Color32) -> Self {
        Self(vec![left, right])
    }

    pub fn ground_truth_gradient(
        left: Color32,
        right: Color32,
        interpolation: Interpolation,
    ) -> Self {
        match interpolation {
            Interpolation::Linear => Self::ground_truth_linear_gradient(left, right),
            Interpolation::Gamma => Self::ground_truth_gamma_gradient(left, right),
        }
    }

    pub fn ground_truth_linear_gradient(left: Color32, right: Color32) -> Self {
        let left = Rgba::from(left);
        let right = Rgba::from(right);

        let n = 255;
        Self(
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    Color32::from(lerp(left..=right, t))
                })
                .collect(),
        )
    }

    pub fn ground_truth_gamma_gradient(left: Color32, right: Color32) -> Self {
        let n = 255;
        Self(
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    left.lerp_to_gamma(right, t)
                })
                .collect(),
        )
    }

    /// Do premultiplied alpha-aware blending of the gradient on top of the fill color
    /// in gamma-space.
    pub fn with_bg_fill(self, bg: Color32) -> Self {
        Self(
            self.0
                .into_iter()
                .map(|fg| {
                    let a = fg.a() as f32 / 255.0;
                    Color32::from_rgba_premultiplied(
                        (bg[0] as f32 * (1.0 - a) + fg[0] as f32).round() as u8,
                        (bg[1] as f32 * (1.0 - a) + fg[1] as f32).round() as u8,
                        (bg[2] as f32 * (1.0 - a) + fg[2] as f32).round() as u8,
                        (bg[3] as f32 * (1.0 - a) + fg[3] as f32).round() as u8,
                    )
                })
                .collect(),
        )
    }

    pub fn to_pixel_row(&self) -> Vec<Color32> {
        self.0.clone()
    }
}

#[derive(Default)]
struct TextureManager(HashMap<Gradient, TextureHandle>);

impl TextureManager {
    fn get(&mut self, ctx: &egui::Context, gradient: &Gradient) -> &TextureHandle {
        self.0.entry(gradient.clone()).or_insert_with(|| {
            let pixels = gradient.to_pixel_row();
            let width = pixels.len();
            let height = 1;
            ctx.load_texture(
                "color_test_gradient",
                epaint::ColorImage {
                    size: [width, height],
                    pixels,
                },
                TextureOptions::LINEAR,
            )
        })
    }
}

/// A visual test that the rendering is correctly aligned on the physical pixel grid.
///
/// Requires eyes and a magnifying glass to verify.
pub fn pixel_test(ui: &mut Ui) {
    ui.heading("Pixel alignment test");
    ui.label("If anything is blurry, then everything will be blurry, including text.");
    ui.label("You might need a magnifying glass to check this test.");

    if cfg!(target_arch = "wasm32") {
        ui.label("Make sure these test pass even when you zoom in/out and resize the browser.");
    }

    ui.add_space(4.0);

    pixel_test_lines(ui);

    ui.add_space(4.0);

    pixel_test_squares(ui);

    ui.add_space(4.0);

    pixel_test_strokes(ui);
}

fn pixel_test_strokes(ui: &mut Ui) {
    ui.label("The strokes should align to the physical pixel grid.");
    let color = if ui.style().visuals.dark_mode {
        egui::Color32::WHITE
    } else {
        egui::Color32::BLACK
    };

    let pixels_per_point = ui.ctx().pixels_per_point();

    for thickness_pixels in 1..=3 {
        let thickness_pixels = thickness_pixels as f32;
        let thickness_points = thickness_pixels / pixels_per_point;
        let num_squares = (pixels_per_point * 10.0).round().max(10.0) as u32;
        let size_pixels = vec2(ui.min_size().x, num_squares as f32 + thickness_pixels * 2.0);
        let size_points = size_pixels / pixels_per_point + Vec2::splat(2.0);
        let (response, painter) = ui.allocate_painter(size_points, Sense::hover());

        let mut cursor_pixel = Pos2::new(
            response.rect.min.x * pixels_per_point + thickness_pixels,
            response.rect.min.y * pixels_per_point + thickness_pixels,
        )
        .ceil();

        let stroke = Stroke::new(thickness_points, color);
        for size in 1..=num_squares {
            let rect_points = Rect::from_min_size(
                Pos2::new(cursor_pixel.x, cursor_pixel.y),
                Vec2::splat(size as f32),
            );
            painter.rect_stroke(
                rect_points / pixels_per_point,
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            cursor_pixel.x += (1 + size) as f32 + thickness_pixels * 2.0;
        }
    }
}

fn pixel_test_squares(ui: &mut Ui) {
    ui.label("The first square should be exactly one physical pixel big.");
    ui.label("They should be exactly one physical pixel apart.");
    ui.label("Each subsequent square should be one physical pixel larger than the previous.");
    ui.label("They should be perfectly aligned to the physical pixel grid.");

    let color = if ui.style().visuals.dark_mode {
        egui::Color32::WHITE
    } else {
        egui::Color32::BLACK
    };

    let pixels_per_point = ui.ctx().pixels_per_point();

    let num_squares = (pixels_per_point * 10.0).round().max(10.0) as u32;
    let size_pixels = vec2(
        ((num_squares + 1) * (num_squares + 2) / 2) as f32,
        num_squares as f32,
    );
    let size_points = size_pixels / pixels_per_point + Vec2::splat(2.0);
    let (response, painter) = ui.allocate_painter(size_points, Sense::hover());

    let mut cursor_pixel = Pos2::new(
        response.rect.min.x * pixels_per_point,
        response.rect.min.y * pixels_per_point,
    )
    .ceil();
    for size in 1..=num_squares {
        let rect_points = Rect::from_min_size(
            Pos2::new(cursor_pixel.x, cursor_pixel.y),
            Vec2::splat(size as f32),
        );
        painter.rect_filled(rect_points / pixels_per_point, 0.0, color);
        cursor_pixel.x += (1 + size) as f32;
    }
}

fn pixel_test_lines(ui: &mut Ui) {
    let pixels_per_point = ui.ctx().pixels_per_point();
    let n = (96.0 * pixels_per_point) as usize;

    ui.label("The lines should be exactly one physical pixel wide, one physical pixel apart.");
    ui.label("They should be perfectly white and black.");

    let hspace_px = pixels_per_point * 4.0;

    let size_px = Vec2::new(2.0 * n as f32 + hspace_px, n as f32);
    let size_points = size_px / pixels_per_point + Vec2::splat(2.0);
    let (response, painter) = ui.allocate_painter(size_points, Sense::hover());

    let mut cursor_px = Pos2::new(
        response.rect.min.x * pixels_per_point,
        response.rect.min.y * pixels_per_point,
    )
    .ceil();

    // Vertical stripes:
    for x in 0..n / 2 {
        let rect_px = Rect::from_min_size(
            Pos2::new(cursor_px.x + 2.0 * x as f32, cursor_px.y),
            Vec2::new(1.0, n as f32),
        );
        painter.rect_filled(rect_px / pixels_per_point, 0.0, egui::Color32::WHITE);
        let rect_px = rect_px.translate(vec2(1.0, 0.0));
        painter.rect_filled(rect_px / pixels_per_point, 0.0, egui::Color32::BLACK);
    }

    cursor_px.x += n as f32 + hspace_px;

    // Horizontal stripes:
    for y in 0..n / 2 {
        let rect_px = Rect::from_min_size(
            Pos2::new(cursor_px.x, cursor_px.y + 2.0 * y as f32),
            Vec2::new(n as f32, 1.0),
        );
        painter.rect_filled(rect_px / pixels_per_point, 0.0, egui::Color32::WHITE);
        let rect_px = rect_px.translate(vec2(0.0, 1.0));
        painter.rect_filled(rect_px / pixels_per_point, 0.0, egui::Color32::BLACK);
    }
}

fn blending_and_feathering_test(ui: &mut Ui) {
    ui.label("The left side shows how lines of different widths look.");
    ui.label("The right side tests text rendering at different opacities and sizes.");
    ui.label("The top and bottom images should look symmetrical in their intensities.");

    let size = vec2(512.0, 512.0);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = response.rect;

    let mut top_half = rect;
    top_half.set_bottom(top_half.center().y);
    painter.rect_filled(top_half, 0.0, Color32::BLACK);
    paint_fine_lines_and_text(&painter, top_half, Color32::WHITE);

    let mut bottom_half = rect;
    bottom_half.set_top(bottom_half.center().y);
    painter.rect_filled(bottom_half, 0.0, Color32::WHITE);
    paint_fine_lines_and_text(&painter, bottom_half, Color32::BLACK);
}

fn text_on_bg(ui: &mut egui::Ui, fg: Color32, bg: Color32) {
    assert!(fg.is_opaque());
    assert!(bg.is_opaque());

    ui.horizontal(|ui| {
        ui.label(
            RichText::from("▣ The quick brown fox jumps over the lazy dog and runs away.")
                .background_color(bg)
                .color(fg),
        );
        ui.label(format!(
            "({} {} {}) on ({} {} {})",
            fg.r(),
            fg.g(),
            fg.b(),
            bg.r(),
            bg.g(),
            bg.b(),
        ));
    });
}

fn paint_fine_lines_and_text(painter: &egui::Painter, mut rect: Rect, color: Color32) {
    {
        let mut y = 0.0;
        for opacity in [1.00, 0.50, 0.25, 0.10, 0.05, 0.02, 0.01, 0.00] {
            painter.text(
                rect.center_top() + vec2(0.0, y),
                Align2::LEFT_TOP,
                format!("{:.0}% white", 100.0 * opacity),
                FontId::proportional(14.0),
                Color32::WHITE.gamma_multiply(opacity),
            );
            painter.text(
                rect.center_top() + vec2(80.0, y),
                Align2::LEFT_TOP,
                format!("{:.0}% gray", 100.0 * opacity),
                FontId::proportional(14.0),
                Color32::GRAY.gamma_multiply(opacity),
            );
            painter.text(
                rect.center_top() + vec2(160.0, y),
                Align2::LEFT_TOP,
                format!("{:.0}% black", 100.0 * opacity),
                FontId::proportional(14.0),
                Color32::BLACK.gamma_multiply(opacity),
            );
            y += 20.0;
        }

        for font_size in [6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0] {
            painter.text(
                rect.center_top() + vec2(0.0, y),
                Align2::LEFT_TOP,
                format!(
                    "{font_size}px - The quick brown fox jumps over the lazy dog and runs away."
                ),
                FontId::proportional(font_size),
                color,
            );
            y += font_size + 1.0;
        }
    }

    rect.max.x = rect.center().x;

    rect = rect.shrink(16.0);
    for width in [0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0] {
        painter.text(
            rect.left_top(),
            Align2::CENTER_CENTER,
            width.to_string(),
            FontId::monospace(12.0),
            color,
        );

        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [
                rect.left_top() + vec2(16.0, 0.0),
                rect.right_top(),
                rect.right_center(),
                rect.right_bottom(),
            ],
            false,
            Color32::TRANSPARENT,
            Stroke::new(width, color),
        ));

        rect.min.y += 24.0;
        rect.max.x -= 24.0;
    }

    rect.min.y += 16.0;
    painter.text(
        rect.left_top(),
        Align2::LEFT_CENTER,
        "transparent --> opaque",
        FontId::monospace(10.0),
        color,
    );
    rect.min.y += 12.0;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_bottom(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.left_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.right_bottom(), color);
    mesh.colored_vertex(rect.right_top(), color);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
}

fn mul_color_gamma(left: Color32, right: Color32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (left.r() as f32 * right.r() as f32 / 255.0).round() as u8,
        (left.g() as f32 * right.g() as f32 / 255.0).round() as u8,
        (left.b() as f32 * right.b() as f32 / 255.0).round() as u8,
        (left.a() as f32 * right.a() as f32 / 255.0).round() as u8,
    )
}
