use crate::{
    render_systems::{
        EguiPipelines, EguiTextureBindGroups, EguiTextureId, EguiTransform, EguiTransforms,
    },
    EguiContextSettings, EguiRenderOutput, EguiRenderToImage, RenderTargetSize,
};
use bevy_asset::prelude::*;
use bevy_ecs::{
    prelude::*,
    world::{FromWorld, World},
};
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy_log as log;
use bevy_render::{
    render_asset::{RenderAssetUsages, RenderAssets},
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::TrackedRenderPass,
    render_resource::{
        BindGroupLayout, BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor,
        BlendOperation, BlendState, Buffer, BufferAddress, BufferBindingType, BufferDescriptor,
        BufferUsages, ColorTargetState, ColorWrites, Extent3d, FragmentState, FrontFace,
        IndexFormat, LoadOp, MultisampleState, Operations, PipelineCache, PrimitiveState,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
        SamplerBindingType, Shader, ShaderStages, ShaderType, SpecializedRenderPipeline, StoreOp,
        TextureDimension, TextureFormat, TextureSampleType, TextureViewDimension,
        VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    sync_world::{MainEntity, RenderEntity},
    texture::GpuImage,
    view::{ExtractedWindow, ExtractedWindows},
};
use bytemuck::cast_slice;
use egui::{TextureFilter, TextureOptions};

/// Egui shader.
pub const EGUI_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(9898276442290979394);

/// Egui render pipeline.
#[derive(Resource)]
pub struct EguiPipeline {
    /// Transform bind group layout.
    pub transform_bind_group_layout: BindGroupLayout,
    /// Texture bind group layout.
    pub texture_bind_group_layout: BindGroupLayout,
}

impl FromWorld for EguiPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let transform_bind_group_layout = render_device.create_bind_group_layout(
            "egui transform bind group layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(EguiTransform::min_size()),
                },
                count: None,
            }],
        );

        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "egui texture bind group layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        );

        EguiPipeline {
            transform_bind_group_layout,
            texture_bind_group_layout,
        }
    }
}

/// Key for specialized pipeline.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct EguiPipelineKey {
    /// Texture format of a window's swap chain to render to.
    pub texture_format: TextureFormat,
    /// Render target type (e.g. window, image).
    pub render_target_type: EguiRenderTargetType,
}

/// Is used to make a render node aware of a render target type.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum EguiRenderTargetType {
    /// Render to a window.
    Window,
    /// Render to an image.
    Image,
}

impl EguiPipelineKey {
    /// Constructs a pipeline key from a window.
    pub fn from_extracted_window(window: &ExtractedWindow) -> Option<Self> {
        Some(Self {
            texture_format: window.swap_chain_texture_format?.add_srgb_suffix(),
            render_target_type: EguiRenderTargetType::Window,
        })
    }

    /// Constructs a pipeline key from a gpu image.
    pub fn from_gpu_image(image: &GpuImage) -> Self {
        EguiPipelineKey {
            texture_format: image.texture_format.add_srgb_suffix(),
            render_target_type: EguiRenderTargetType::Image,
        }
    }
}

impl SpecializedRenderPipeline for EguiPipeline {
    type Key = EguiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("egui render pipeline".into()),
            layout: vec![
                self.transform_bind_group_layout.clone(),
                self.texture_bind_group_layout.clone(),
            ],
            vertex: VertexState {
                shader: EGUI_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "vs_main".into(),
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
                shader_defs: Vec::new(),
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Cw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
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

/// Egui render node.
pub struct EguiNode {
    render_target_main_entity: MainEntity,
    render_target_render_entity: RenderEntity,
    render_target_type: EguiRenderTargetType,
    vertex_data: Vec<u8>,
    vertex_buffer_capacity: usize,
    vertex_buffer: Option<Buffer>,
    index_data: Vec<u8>,
    index_buffer_capacity: usize,
    index_buffer: Option<Buffer>,
    draw_commands: Vec<DrawCommand>,
    postponed_updates: Vec<(egui::Rect, PaintCallbackDraw)>,
    pixels_per_point: f32,
}

impl EguiNode {
    /// Constructs Egui render node.
    pub fn new(
        render_target_main_entity: MainEntity,
        render_target_render_entity: RenderEntity,
        render_target_type: EguiRenderTargetType,
    ) -> Self {
        EguiNode {
            render_target_main_entity,
            render_target_render_entity,
            render_target_type,
            draw_commands: Vec::new(),
            vertex_data: Vec::new(),
            vertex_buffer_capacity: 0,
            vertex_buffer: None,
            index_data: Vec::new(),
            index_buffer_capacity: 0,
            index_buffer: None,
            postponed_updates: Vec::new(),
            pixels_per_point: 1.,
        }
    }
}

impl Node for EguiNode {
    fn update(&mut self, world: &mut World) {
        let mut render_target_query = world.query::<(
            &EguiContextSettings,
            &RenderTargetSize,
            &mut EguiRenderOutput,
            Option<&EguiRenderToImage>,
        )>();

        let Ok((egui_settings, render_target_size, mut render_output, render_to_image)) =
            render_target_query.get_mut(world, self.render_target_render_entity.id())
        else {
            log::error!(
                "Failed to update Egui render node for {:?} context: missing components",
                self.render_target_main_entity.id()
            );
            return;
        };
        let render_target_size = *render_target_size;
        let egui_settings = egui_settings.clone();
        let image_handle =
            render_to_image.map(|render_to_image| render_to_image.handle.clone_weak());

        let paint_jobs = std::mem::take(&mut render_output.paint_jobs);

        // Construct a pipeline key based on a render target.
        let key = match self.render_target_type {
            EguiRenderTargetType::Window => {
                let Some(key) = world
                    .resource::<ExtractedWindows>()
                    .windows
                    .get(&self.render_target_main_entity.id())
                    .and_then(EguiPipelineKey::from_extracted_window)
                else {
                    return;
                };
                key
            }
            EguiRenderTargetType::Image => {
                let image_handle = image_handle
                    .expect("Expected an image handle for a render to image node")
                    .clone();
                let Some(key) = world
                    .resource::<RenderAssets<GpuImage>>()
                    .get(&image_handle)
                    .map(EguiPipelineKey::from_gpu_image)
                else {
                    return;
                };
                key
            }
        };

        self.pixels_per_point = render_target_size.scale_factor * egui_settings.scale_factor;
        if render_target_size.physical_width == 0.0 || render_target_size.physical_height == 0.0 {
            return;
        }

        let mut index_offset = 0;

        self.draw_commands.clear();
        self.vertex_data.clear();
        self.index_data.clear();
        self.postponed_updates.clear();

        let render_device = world.resource::<RenderDevice>();

        for egui::epaint::ClippedPrimitive {
            clip_rect,
            primitive,
        } in paint_jobs
        {
            let clip_urect = bevy_math::URect {
                min: bevy_math::UVec2 {
                    x: (clip_rect.min.x * self.pixels_per_point).round() as u32,
                    y: (clip_rect.min.y * self.pixels_per_point).round() as u32,
                },
                max: bevy_math::UVec2 {
                    x: (clip_rect.max.x * self.pixels_per_point).round() as u32,
                    y: (clip_rect.max.y * self.pixels_per_point).round() as u32,
                },
            };

            if clip_urect
                .intersect(bevy_math::URect::new(
                    0,
                    0,
                    render_target_size.physical_width as u32,
                    render_target_size.physical_height as u32,
                ))
                .is_empty()
            {
                continue;
            }

            let mesh = match primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh,
                egui::epaint::Primitive::Callback(paint_callback) => {
                    let callback = match paint_callback.callback.downcast::<EguiBevyPaintCallback>()
                    {
                        Ok(callback) => callback,
                        Err(err) => {
                            log::error!("Unsupported Egui paint callback type: {err:?}");
                            continue;
                        }
                    };

                    self.postponed_updates.push((
                        clip_rect,
                        PaintCallbackDraw {
                            callback: callback.clone(),
                            rect: paint_callback.rect,
                        },
                    ));

                    self.draw_commands.push(DrawCommand {
                        primitive: DrawPrimitive::PaintCallback(PaintCallbackDraw {
                            callback,
                            rect: paint_callback.rect,
                        }),
                        clip_rect,
                    });
                    continue;
                }
            };

            self.vertex_data
                .extend_from_slice(cast_slice::<_, u8>(mesh.vertices.as_slice()));
            let indices_with_offset = mesh
                .indices
                .iter()
                .map(|i| i + index_offset)
                .collect::<Vec<_>>();
            self.index_data
                .extend_from_slice(cast_slice(indices_with_offset.as_slice()));
            index_offset += mesh.vertices.len() as u32;

            let texture_handle = match mesh.texture_id {
                egui::TextureId::Managed(id) => {
                    EguiTextureId::Managed(self.render_target_main_entity, id)
                }
                egui::TextureId::User(id) => EguiTextureId::User(id),
            };

            self.draw_commands.push(DrawCommand {
                primitive: DrawPrimitive::Egui(EguiDraw {
                    vertices_count: mesh.indices.len(),
                    egui_texture: texture_handle,
                }),
                clip_rect,
            });
        }

        if self.vertex_data.len() > self.vertex_buffer_capacity {
            self.vertex_buffer_capacity = if self.vertex_data.len().is_power_of_two() {
                self.vertex_data.len()
            } else {
                self.vertex_data.len().next_power_of_two()
            };
            self.vertex_buffer = Some(render_device.create_buffer(&BufferDescriptor {
                label: Some("egui vertex buffer"),
                size: self.vertex_buffer_capacity as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
                mapped_at_creation: false,
            }));
        }
        if self.index_data.len() > self.index_buffer_capacity {
            self.index_buffer_capacity = if self.index_data.len().is_power_of_two() {
                self.index_data.len()
            } else {
                self.index_data.len().next_power_of_two()
            };
            self.index_buffer = Some(render_device.create_buffer(&BufferDescriptor {
                label: Some("egui index buffer"),
                size: self.index_buffer_capacity as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
                mapped_at_creation: false,
            }));
        }

        for (clip_rect, command) in self.postponed_updates.drain(..) {
            let info = egui::PaintCallbackInfo {
                viewport: command.rect,
                clip_rect,
                pixels_per_point: self.pixels_per_point,
                screen_size_px: [
                    render_target_size.physical_width as u32,
                    render_target_size.physical_height as u32,
                ],
            };
            command
                .callback
                .cb()
                .update(info, self.render_target_render_entity, key, world);
        }
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let egui_pipelines = &world.resource::<EguiPipelines>().0;
        let pipeline_cache = world.resource::<PipelineCache>();

        let (key, swap_chain_texture_view, physical_width, physical_height, load_op) =
            match self.render_target_type {
                EguiRenderTargetType::Window => {
                    let Some(window) = world
                        .resource::<ExtractedWindows>()
                        .windows
                        .get(&self.render_target_main_entity.id())
                    else {
                        return Ok(());
                    };

                    let Some(swap_chain_texture_view) = &window.swap_chain_texture_view else {
                        return Ok(());
                    };

                    let Some(key) = EguiPipelineKey::from_extracted_window(window) else {
                        return Ok(());
                    };
                    (
                        key,
                        swap_chain_texture_view,
                        window.physical_width,
                        window.physical_height,
                        LoadOp::Load,
                    )
                }
                EguiRenderTargetType::Image => {
                    let Some(extracted_render_to_image): Option<&EguiRenderToImage> =
                        world.get(self.render_target_render_entity.id())
                    else {
                        return Ok(());
                    };

                    let gpu_images = world.resource::<RenderAssets<GpuImage>>();
                    let Some(gpu_image) = gpu_images.get(&extracted_render_to_image.handle) else {
                        return Ok(());
                    };
                    (
                        EguiPipelineKey::from_gpu_image(gpu_image),
                        &gpu_image.texture_view,
                        gpu_image.size.x,
                        gpu_image.size.y,
                        extracted_render_to_image.load_op,
                    )
                }
            };

        let render_queue = world.resource::<RenderQueue>();

        let (vertex_buffer, index_buffer) = match (&self.vertex_buffer, &self.index_buffer) {
            (Some(vertex), Some(index)) => (vertex, index),
            _ => {
                return Ok(());
            }
        };

        render_queue.write_buffer(vertex_buffer, 0, &self.vertex_data);
        render_queue.write_buffer(index_buffer, 0, &self.index_data);

        for draw_command in &self.draw_commands {
            match &draw_command.primitive {
                DrawPrimitive::Egui(_command) => {}
                DrawPrimitive::PaintCallback(command) => {
                    let info = egui::PaintCallbackInfo {
                        viewport: command.rect,
                        clip_rect: draw_command.clip_rect,
                        pixels_per_point: self.pixels_per_point,
                        screen_size_px: [physical_width, physical_height],
                    };

                    command.callback.cb().prepare_render(
                        info,
                        render_context,
                        self.render_target_render_entity,
                        key,
                        world,
                    );
                }
            }
        }

        let bind_groups = &world.resource::<EguiTextureBindGroups>().0;
        let egui_transforms = world.resource::<EguiTransforms>();
        let device = world.resource::<RenderDevice>();

        let render_pass =
            render_context
                .command_encoder()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("egui render pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: swap_chain_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: load_op,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
        let mut render_pass = TrackedRenderPass::new(device, render_pass);

        let pipeline_id = egui_pipelines
            .get(&self.render_target_main_entity)
            .expect("Expected a queued pipeline");
        let Some(pipeline) = pipeline_cache.get_render_pipeline(*pipeline_id) else {
            return Ok(());
        };

        let transform_buffer_offset = egui_transforms.offsets[&self.render_target_main_entity];
        let transform_buffer_bind_group = &egui_transforms
            .bind_group
            .as_ref()
            .expect("Expected a prepared bind group")
            .1;

        let mut requires_reset = true;

        let mut vertex_offset: u32 = 0;
        for draw_command in &self.draw_commands {
            if requires_reset {
                render_pass.set_viewport(
                    0.,
                    0.,
                    physical_width as f32,
                    physical_height as f32,
                    0.,
                    1.,
                );
                render_pass.set_render_pipeline(pipeline);
                render_pass.set_bind_group(
                    0,
                    transform_buffer_bind_group,
                    &[transform_buffer_offset],
                );

                requires_reset = false;
            }

            let clip_urect = bevy_math::URect {
                min: bevy_math::UVec2 {
                    x: (draw_command.clip_rect.min.x * self.pixels_per_point).round() as u32,
                    y: (draw_command.clip_rect.min.y * self.pixels_per_point).round() as u32,
                },
                max: bevy_math::UVec2 {
                    x: (draw_command.clip_rect.max.x * self.pixels_per_point).round() as u32,
                    y: (draw_command.clip_rect.max.y * self.pixels_per_point).round() as u32,
                },
            };

            let scissor_rect =
                clip_urect.intersect(bevy_math::URect::new(0, 0, physical_width, physical_height));
            if scissor_rect.is_empty() {
                continue;
            }

            render_pass.set_scissor_rect(
                scissor_rect.min.x,
                scissor_rect.min.y,
                scissor_rect.width(),
                scissor_rect.height(),
            );

            match &draw_command.primitive {
                DrawPrimitive::Egui(command) => {
                    let texture_bind_group = match bind_groups.get(&command.egui_texture) {
                        Some(texture_resource) => texture_resource,
                        None => {
                            vertex_offset += command.vertices_count as u32;
                            continue;
                        }
                    };

                    render_pass.set_bind_group(1, texture_bind_group, &[]);

                    render_pass.set_vertex_buffer(
                        0,
                        self.vertex_buffer
                            .as_ref()
                            .expect("Expected an initialized vertex buffer")
                            .slice(..),
                    );
                    render_pass.set_index_buffer(
                        self.index_buffer
                            .as_ref()
                            .expect("Expected an initialized index buffer")
                            .slice(..),
                        0,
                        IndexFormat::Uint32,
                    );

                    render_pass.draw_indexed(
                        vertex_offset..(vertex_offset + command.vertices_count as u32),
                        0,
                        0..1,
                    );

                    vertex_offset += command.vertices_count as u32;
                }
                DrawPrimitive::PaintCallback(command) => {
                    let info = egui::PaintCallbackInfo {
                        viewport: command.rect,
                        clip_rect: draw_command.clip_rect,
                        pixels_per_point: self.pixels_per_point,
                        screen_size_px: [physical_width, physical_height],
                    };

                    let viewport = info.viewport_in_pixels();
                    if viewport.width_px > 0 && viewport.height_px > 0 {
                        requires_reset = true;
                        render_pass.set_viewport(
                            viewport.left_px as f32,
                            viewport.top_px as f32,
                            viewport.width_px as f32,
                            viewport.height_px as f32,
                            0.,
                            1.,
                        );

                        command.callback.cb().render(
                            info,
                            &mut render_pass,
                            self.render_target_render_entity,
                            key,
                            world,
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

pub(crate) fn as_color_image(image: egui::ImageData) -> egui::ColorImage {
    match image {
        egui::ImageData::Color(image) => (*image).clone(),
        egui::ImageData::Font(image) => alpha_image_as_color_image(&image),
    }
}

fn alpha_image_as_color_image(image: &egui::FontImage) -> egui::ColorImage {
    egui::ColorImage {
        size: image.size,
        pixels: image.srgba_pixels(None).collect(),
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
        // convenient way to convert them to premultiplied ones), we do the this with Egui ones.
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

/// Callback to execute custom 'wgpu' rendering inside [`EguiNode`] render graph node.
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
        window_entity: RenderEntity,
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
        window_entity: RenderEntity,
        pipeline_key: EguiPipelineKey,
        world: &'w World,
    ) {
        let _ = (info, render_context, window_entity, pipeline_key, world);
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
        window_entity: RenderEntity,
        pipeline_key: EguiPipelineKey,
        world: &'pass World,
    );
}
