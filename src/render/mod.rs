pub use render_pass::*;

/// Defines Egui node graph.
pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    /// Egui subgraph (is run by [`super::RunEguiSubgraphOnEguiViewNode`]).
    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct SubGraphEgui;

    /// Egui node defining the Egui rendering pass.
    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodeEgui {
        /// Egui rendering pass.
        EguiPass,
    }
}

use crate::{
    EguiContextSettings, EguiRenderOutput, RenderComputedScaleFactor,
    render::graph::{NodeEgui, SubGraphEgui},
};
use bevy_app::SubApp;
use bevy_asset::{Handle, RenderAssetUsages, uuid_handle};
use bevy_camera::Camera;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Has,
    resource::Resource,
    system::{Commands, Local, ResMut},
    world::{FromWorld, World},
};
use bevy_image::{
    BevyDefault, Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
};
use bevy_math::{Mat4, UVec4};
use bevy_mesh::VertexBufferLayout;
use bevy_platform::collections::HashSet;
use bevy_render::{
    MainWorld,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext},
    render_phase::TrackedRenderPass,
    render_resource::{
        BindGroupLayout, BindGroupLayoutEntries, FragmentState, RenderPipelineDescriptor,
        SpecializedRenderPipeline, VertexState,
        binding_types::{sampler, texture_2d, uniform_buffer},
    },
    renderer::{RenderContext, RenderDevice},
    sync_world::{RenderEntity, TemporaryRenderEntity},
    view::{ExtractedView, Hdr, RetainedViewEntity, ViewTarget},
};
use bevy_shader::{Shader, ShaderDefVal};
use egui::{TextureFilter, TextureOptions};
use std::num::NonZero;
use systems::{EguiTextureId, EguiTransform};
use wgpu_types::{
    BlendState, ColorTargetState, ColorWrites, Extent3d, MultisampleState, PrimitiveState,
    PushConstantRange, SamplerBindingType, ShaderStages, TextureDimension, TextureFormat,
    TextureSampleType, VertexFormat, VertexStepMode,
};

mod render_pass;
/// Plugin systems for the render app.
#[cfg(feature = "render")]
pub mod systems;

/// A render-world component that lives on the main render target view and
/// specifies the corresponding Egui view.
///
/// For example, if Egui is being rendered to a 3D camera, this component lives on
/// the 3D camera and contains the entity corresponding to the Egui view.
///
/// Entity id of the temporary render entity with the corresponding extracted Egui view.
#[derive(Component, Debug)]
pub struct EguiCameraView(pub Entity);

/// A render-world component that lives on the Egui view and specifies the
/// corresponding main render target view.
///
/// For example, if Egui is being rendered to a 3D camera, this component
/// lives on the Egui view and contains the entity corresponding to the 3D camera.
///
/// This is the inverse of [`EguiCameraView`].
#[derive(Component, Debug)]
pub struct EguiViewTarget(pub Entity);

/// Adds and returns an Egui subgraph.
pub fn get_egui_graph(render_app: &mut SubApp) -> RenderGraph {
    let pass_node = EguiPassNode::new(render_app.world_mut());
    let mut graph = RenderGraph::default();
    graph.add_node(NodeEgui::EguiPass, pass_node);
    graph
}

/// A [`Node`] that executes the Egui rendering subgraph on the Egui view.
pub struct RunEguiSubgraphOnEguiViewNode;

impl Node for RunEguiSubgraphOnEguiViewNode {
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        _: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Fetch the UI view.
        let Some(mut render_views) = world.try_query::<&EguiCameraView>() else {
            return Ok(());
        };
        let Ok(default_camera_view) = render_views.get(world, graph.view_entity()) else {
            return Ok(());
        };

        // Run the subgraph on the Egui view.
        graph.run_sub_graph(SubGraphEgui, vec![], Some(default_camera_view.0))?;
        Ok(())
    }
}

/// Extracts all Egui contexts associated with a camera into the render world.
pub fn extract_egui_camera_view_system(
    mut commands: Commands,
    mut world: ResMut<MainWorld>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();
    let mut q = world.query::<(
        Entity,
        RenderEntity,
        &Camera,
        &mut EguiRenderOutput,
        &EguiContextSettings,
        Has<Hdr>,
    )>();

    for (main_entity, render_entity, camera, mut egui_render_output, settings, hdr) in
        &mut q.iter_mut(&mut world)
    {
        // Move Egui shapes and textures out of the main world into the render one.
        let egui_render_output = std::mem::take(egui_render_output.as_mut());

        // Ignore inactive cameras.
        if !camera.is_active {
            commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.")
                .remove::<EguiCameraView>();
            continue;
        }

        const UI_CAMERA_FAR: f32 = 1000.0;
        const EGUI_CAMERA_SUBVIEW: u32 = 2095931312;
        const UI_CAMERA_TRANSFORM_OFFSET: f32 = -0.1;

        if let Some(physical_viewport_rect) = camera.physical_viewport_rect() {
            // Use a projection matrix with the origin in the top left instead of the bottom left that comes with OrthographicProjection.
            let projection_matrix = Mat4::orthographic_rh(
                0.0,
                physical_viewport_rect.width() as f32,
                physical_viewport_rect.height() as f32,
                0.0,
                0.0,
                UI_CAMERA_FAR,
            );
            // We use `EGUI_CAMERA_SUBVIEW` here so as not to conflict with the
            // main 3D or 2D camera or UI view, which will have subview index 0 or 1.
            let retained_view_entity =
                RetainedViewEntity::new(main_entity.into(), None, EGUI_CAMERA_SUBVIEW);
            // Creates the UI view.
            let ui_camera_view = commands
                .spawn((
                    ExtractedView {
                        retained_view_entity,
                        clip_from_view: projection_matrix,
                        world_from_view: bevy_transform::components::GlobalTransform::from_xyz(
                            0.0,
                            0.0,
                            UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
                        ),
                        clip_from_world: None,
                        hdr,
                        viewport: UVec4::from((
                            physical_viewport_rect.min,
                            physical_viewport_rect.size(),
                        )),
                        color_grading: Default::default(),
                    },
                    // Link to the main camera view.
                    EguiViewTarget(render_entity),
                    egui_render_output,
                    RenderComputedScaleFactor {
                        scale_factor: settings.scale_factor
                            * camera.target_scaling_factor().unwrap_or(1.0),
                    },
                    TemporaryRenderEntity,
                ))
                .id();

            let mut entity_commands = commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.");
            // Link from the main 2D/3D camera view to the UI view.
            entity_commands.insert(EguiCameraView(ui_camera_view));
            live_entities.insert(retained_view_entity);
        }
    }
}

/// Egui shader.
pub const EGUI_SHADER_HANDLE: Handle<Shader> = uuid_handle!("05a4d7a0-4f24-4d7f-b606-3f399074261f");

/// Egui render settings.
#[derive(Resource)]
pub struct EguiRenderSettings {
    /// See [`super::EguiPlugin`] for setting description.
    pub bindless_mode_array_size: Option<NonZero<u32>>,
}

/// Egui render pipeline.
#[derive(Resource)]
pub struct EguiPipeline {
    /// Transform bind group layout.
    pub transform_bind_group_layout: BindGroupLayout,
    /// Texture bind group layout.
    pub texture_bind_group_layout: BindGroupLayout,
    /// Is bindless rendering mode enabled
    /// and how many textures can be rendered in one bind group.
    pub bindless: Option<NonZero<u32>>,
}

impl FromWorld for EguiPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let settings = render_world.resource::<EguiRenderSettings>();

        let features = render_device.features();

        // TODO: In wgpu 0.26
        // Check: max_binding_array_elements_per_shader_stage and
        // max_binding_array_sampler_elements-per_shader_stage
        // to be sure that device support provided limits
        let bindless = if features.contains(wgpu_types::Features::TEXTURE_BINDING_ARRAY)
            && features.contains(wgpu_types::Features::PUSH_CONSTANTS)
        {
            settings.bindless_mode_array_size
        } else {
            None
        };

        let transform_bind_group_layout = render_device.create_bind_group_layout(
            "egui_transform_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<EguiTransform>(true),
            ),
        );

        let texture_bind_group_layout = if let Some(bindless) = bindless {
            render_device.create_bind_group_layout(
                "egui_texture_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }).count(bindless),
                        sampler(SamplerBindingType::Filtering).count(bindless),
                    ),
                ),
            )
        } else {
            render_device.create_bind_group_layout(
                "egui_texture_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            )
        };

        EguiPipeline {
            transform_bind_group_layout,
            texture_bind_group_layout,
            bindless,
        }
    }
}

/// Key for specialized pipeline.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct EguiPipelineKey {
    /// Reflects the value of [`Camera::hdr`].
    pub hdr: bool,
}

impl SpecializedRenderPipeline for EguiPipeline {
    type Key = EguiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        let mut push_constant_ranges = Vec::new();

        if let Some(bindless) = self.bindless {
            shader_defs.push(ShaderDefVal::UInt("BINDLESS".into(), u32::from(bindless)));
            push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..4,
            });
        }

        RenderPipelineDescriptor {
            label: Some("egui_pipeline".into()),
            layout: vec![
                self.transform_bind_group_layout.clone(),
                self.texture_bind_group_layout.clone(),
            ],
            vertex: VertexState {
                shader: EGUI_SHADER_HANDLE,
                shader_defs: shader_defs.clone(),
                entry_point: Some("vs_main".into()),
                buffers: vec![VertexBufferLayout::from_vertex_formats(
                    VertexStepMode::Vertex,
                    [
                        VertexFormat::Float32x2, // position
                        VertexFormat::Float32x2, // UV
                        VertexFormat::Unorm8x4,  // color (sRGB)
                    ],
                )],
            },
            fragment: Some(FragmentState {
                shader: EGUI_SHADER_HANDLE,
                shader_defs,
                entry_point: Some("fs_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges,
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub(crate) struct DrawCommand {
    pub(crate) clip_rect: egui::Rect,
    pub(crate) primitive: DrawPrimitive,
}

pub(crate) enum DrawPrimitive {
    Egui(EguiDraw),
    PaintCallback(PaintCallbackDraw),
}

pub(crate) struct PaintCallbackDraw {
    pub(crate) callback: std::sync::Arc<EguiBevyPaintCallback>,
    pub(crate) rect: egui::Rect,
}

pub(crate) struct EguiDraw {
    pub(crate) vertices_count: usize,
    pub(crate) egui_texture: EguiTextureId,
}

pub(crate) fn as_color_image(image: &egui::ImageData) -> egui::ColorImage {
    match image {
        egui::ImageData::Color(image) => (**image).clone(),
    }
}

pub(crate) fn color_image_as_bevy_image(
    egui_image: &egui::ColorImage,
    sampler_descriptor: ImageSampler,
) -> Image {
    let pixels = egui_image
        .pixels
        .iter()
        // We unmultiply Egui textures to premultiply them later in the fragment shader.
        // As user textures loaded as Bevy assets are not premultiplied (and there seems to be no
        // convenient way to convert them to premultiplied ones), we do this with Egui ones.
        .flat_map(|color| color.to_srgba_unmultiplied())
        .collect();

    Image {
        sampler: sampler_descriptor,
        ..Image::new(
            Extent3d {
                width: egui_image.width() as u32,
                height: egui_image.height() as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
    }
}

pub(crate) fn texture_options_as_sampler_descriptor(
    options: &TextureOptions,
) -> ImageSamplerDescriptor {
    fn convert_filter(filter: &TextureFilter) -> ImageFilterMode {
        match filter {
            egui::TextureFilter::Nearest => ImageFilterMode::Nearest,
            egui::TextureFilter::Linear => ImageFilterMode::Linear,
        }
    }
    let address_mode = match options.wrap_mode {
        egui::TextureWrapMode::ClampToEdge => ImageAddressMode::ClampToEdge,
        egui::TextureWrapMode::Repeat => ImageAddressMode::Repeat,
        egui::TextureWrapMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
    };
    ImageSamplerDescriptor {
        mag_filter: convert_filter(&options.magnification),
        min_filter: convert_filter(&options.minification),
        address_mode_u: address_mode,
        address_mode_v: address_mode,
        ..Default::default()
    }
}

/// Callback to execute custom 'wgpu' rendering inside [`EguiPassNode`] render graph node.
///
/// Rendering can be implemented using for example:
/// * native wgpu rendering libraries,
/// * or with [`bevy_render::render_phase`] approach.
pub struct EguiBevyPaintCallback(Box<dyn EguiBevyPaintCallbackImpl>);

impl EguiBevyPaintCallback {
    /// Creates a new [`egui::epaint::PaintCallback`] from a callback trait instance.
    pub fn new_paint_callback<T>(rect: egui::Rect, callback: T) -> egui::epaint::PaintCallback
    where
        T: EguiBevyPaintCallbackImpl + 'static,
    {
        let callback = Self(Box::new(callback));
        egui::epaint::PaintCallback {
            rect,
            callback: std::sync::Arc::new(callback),
        }
    }

    pub(crate) fn cb(&self) -> &dyn EguiBevyPaintCallbackImpl {
        self.0.as_ref()
    }
}

/// Callback that executes custom rendering logic
pub trait EguiBevyPaintCallbackImpl: Send + Sync {
    /// Paint callback will be rendered in near future, all data must be finalized for render step
    fn update(
        &self,
        info: egui::PaintCallbackInfo,
        render_entity: RenderEntity,
        pipeline_key: EguiPipelineKey,
        world: &mut World,
    );

    /// Paint callback call before render step
    ///
    ///
    /// Can be used to implement custom render passes
    /// or to submit command buffers for execution before egui render pass
    fn prepare_render<'w>(
        &self,
        info: egui::PaintCallbackInfo,
        render_context: &mut RenderContext<'w>,
        render_entity: RenderEntity,
        pipeline_key: EguiPipelineKey,
        world: &'w World,
    ) {
        let _ = (info, render_context, render_entity, pipeline_key, world);
        // Do nothing by default
    }

    /// Paint callback render step
    ///
    /// Native wgpu RenderPass can be retrieved from [`TrackedRenderPass`] by calling
    /// [`TrackedRenderPass::wgpu_pass`].
    fn render<'pass>(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut TrackedRenderPass<'pass>,
        render_entity: RenderEntity,
        pipeline_key: EguiPipelineKey,
        world: &'pass World,
    );
}
